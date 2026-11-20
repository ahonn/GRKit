//! The iOS transport of the shared connect flow (`run_connect`).

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::domain::connect_policy::NetworkSnapshot;
use crate::domain::{
    BleSetup, CameraModel, CameraStatus, ConnectOptions, ConnectStep, ConnectTransport, Join,
    LinkError, LocalNetworkAccess, PackageInfo, Probe, RememberedCamera, WifiCredentials,
    run_connect,
};

use super::{ble, local_network, wifi};
use crate::platform::{Connected, http, link_support};

/// The latest connect attempt. A cancelled attempt's teardown stops once a
/// newer attempt starts, so it never drops the successor's link.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

struct LinkAttempt {
    peripheral_id: String,
    generation: u64,
    /// The Bluetooth link came up during this attempt.
    ble_connected: Rc<Cell<bool>>,
    /// The attempt ran to a result; dropped before that, it was cancelled.
    finished: bool,
    succeeded: bool,
}

impl LinkAttempt {
    /// Dropped unfinished because the caller's attempt deadline expired,
    /// not because the user cancelled.
    fn timed_out(&self) -> bool {
        crate::platform::connect_deadline_reached()
    }
}

impl Drop for LinkAttempt {
    fn drop(&mut self) {
        let user_cancel = !self.finished && !self.timed_out();
        if user_cancel && self.ble_connected.get() {
            // Close the camera access point before dropping the link. Drop
            // cannot wait, so a bounded task does it.
            let peripheral_id = self.peripheral_id.clone();
            let generation = self.generation;
            app_info!("connect cancelled; closing access point");
            nsh_core::prelude::spawn(async move {
                let owns = || ATTEMPT.load(Ordering::SeqCst) == generation;
                ble::close_access_point(&peripheral_id, owns).await;
            });
        } else if !self.succeeded {
            ble::cancel_connection(&self.peripheral_id);
        }
        ble::resume_scanning();
    }
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

struct IosTransport {
    package: Option<(CameraStatus, PackageInfo)>,
    model_hint: CameraModel,
    powered_on: bool,
    ble_budget: (Duration, u32),
    ble_connected: Rc<Cell<bool>>,
}

impl ConnectTransport for IosTransport {
    fn ble_setup(&mut self, peripheral_id: &str) -> BoxFut<'_, Result<BleSetup, LinkError>> {
        let model_hint = self.model_hint;
        let peripheral_id = peripheral_id.to_owned();
        let connected = self.ble_connected.clone();
        let powered_on = self.powered_on;
        let budget = self.ble_budget;
        Box::pin(async move {
            app_info!("connect step Bluetooth");
            let setup =
                ble::ble_setup(&peripheral_id, model_hint, powered_on, budget, &connected).await;
            // The model the camera reports names it in the accessory picker.
            if let Ok(setup) = &setup {
                self.model_hint = setup.model;
            }
            setup
        })
    }

    fn wait_foreground(&mut self) -> BoxFut<'_, ()> {
        Box::pin(link_support::wait_foreground())
    }

    fn join_wifi(
        &mut self,
        creds: &WifiCredentials,
        force: bool,
    ) -> BoxFut<'_, Result<Join, LinkError>> {
        let creds = creds.clone();
        // The accessory picker names the camera by its model, as the camera tab does.
        let name = format!("RICOH {}", self.model_hint.display_name());
        Box::pin(async move {
            app_info!("connect step Wi-Fi force={force}");
            wifi::join(&creds, &name, force).await
        })
    }

    fn network(&mut self) -> BoxFut<'_, NetworkSnapshot> {
        Box::pin(async { link_support::network_snapshot() })
    }

    fn probe(&mut self, timeout: Duration) -> BoxFut<'_, Probe> {
        Box::pin(link_support::probe_props(timeout))
    }

    fn camera_api_answers(&mut self) -> BoxFut<'_, bool> {
        Box::pin(http::camera_api_answers())
    }

    fn read_version(&mut self, model: CameraModel) -> BoxFut<'_, Result<(), LinkError>> {
        Box::pin(async move {
            app_info!("connect step Version");
            let (status, package) = http::read_package(model).await?;
            self.package = Some((status, package));
            Ok(())
        })
    }

    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()> {
        Box::pin(nsh_core::prelude::timer(duration))
    }

    fn clock(&mut self) -> Duration {
        link_support::foreground_clock()
    }

    fn local_network_access(&mut self) -> BoxFut<'_, LocalNetworkAccess> {
        Box::pin(local_network::check_access())
    }
}

/// Connect to a nearby camera and return status, package, and remembered snapshot.
pub async fn connect(
    peripheral_id: &str,
    model_hint: CameraModel,
    options: ConnectOptions,
    on_step: &mut dyn FnMut(ConnectStep),
) -> Result<Connected, LinkError> {
    app_info!("link connect {}", short_id(peripheral_id));
    let ble_connected = Rc::new(Cell::new(false));
    let mut attempt = LinkAttempt {
        peripheral_id: peripheral_id.to_owned(),
        generation: ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1,
        ble_connected: ble_connected.clone(),
        finished: false,
        succeeded: false,
    };
    let mut transport = IosTransport {
        model_hint,
        package: None,
        powered_on: options.advertised_powered_on,
        ble_budget: options.ble_connect_budget(),
        ble_connected,
    };
    let result = run_connect(&mut transport, peripheral_id, options, on_step).await;
    attempt.finished = true;
    let result = result.and_then(|report| {
        let (status, package) = transport
            .package
            .take()
            .ok_or(LinkError::Http("missing package after connect".into()))?;
        let remembered = RememberedCamera::new(
            report.setup.peripheral_id.clone(),
            status.camera.model,
            report.setup.credentials.ssid.clone(),
        )
        .with_device_id(status.device_id.clone());
        Ok(Connected {
            status,
            package,
            remembered,
            report: Some(report),
        })
    });
    attempt.succeeded = result.is_ok();
    match &result {
        Ok(_) => app_info!("connect ok {}", short_id(peripheral_id)),
        Err(e) => log::warn!("connect failed {}: {e:?}", short_id(peripheral_id)),
    }
    result
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}
