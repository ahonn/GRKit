//! Bluetooth setup: connect, read the Wi-Fi credentials, wake the camera and
//! start its access point. Platforms supply the GATT operations through
//! [`GattPort`]; the order, the retries and the tolerated failures live here.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::domain::connect_flow::{BleSetup, WifiCredentials, timing};
use crate::domain::uuids::{CAMERA_POWER, NETWORK_TYPE, PASSPHRASE, SSID};
use crate::domain::{CameraModel, LinkError, decode_cred};

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// GATT operations on one camera link. Characteristics are the UUIDs of
/// [`crate::domain::uuids`].
pub trait GattPort {
    /// Connect within the platform's connect budget and discover the services.
    fn connect(&mut self) -> BoxFut<'_, Result<(), LinkError>>;
    fn read(&mut self, characteristic: &'static str) -> BoxFut<'_, Result<Vec<u8>, LinkError>>;
    /// Write with response. Notifications that came before the write are
    /// dropped, so a later [`GattPort::wait_notify`] answers this write.
    fn write(
        &mut self,
        characteristic: &'static str,
        value: &'static [u8],
    ) -> BoxFut<'_, Result<(), LinkError>>;
    fn subscribe(&mut self, characteristic: &'static str) -> BoxFut<'_, Result<(), LinkError>>;
    /// Whether `characteristic` notified a value that starts with `value`
    /// since the last write, waiting up to `timeout` for it.
    fn wait_notify(
        &mut self,
        characteristic: &'static str,
        value: u8,
        timeout: Duration,
    ) -> BoxFut<'_, Result<bool, LinkError>>;
    /// Stop notifications. Best effort: the camera's answer is not awaited.
    fn unsubscribe(&mut self, characteristic: &'static str);
    fn pause(&mut self, duration: Duration) -> BoxFut<'_, ()>;
}

/// Connect, read the credentials, wake the camera unless a fresh
/// advertisement says it is on, and start its access point.
pub async fn run_ble_setup<P: GattPort>(
    port: &mut P,
    peripheral_id: &str,
    model: CameraModel,
    powered_on_already: bool,
) -> Result<BleSetup, LinkError> {
    port.connect().await?;

    app_info!("BLE reading Wi-Fi credentials");
    let ssid = decode_cred(&port.read(SSID).await?);
    let passphrase = decode_cred(&port.read(PASSPHRASE).await?);
    if ssid.is_empty() || passphrase.is_empty() {
        // The camera hides its network until this phone is paired.
        app_info!("BLE Wi-Fi credentials empty");
        return Err(LinkError::NotPaired);
    }

    if !powered_on_already {
        app_info!("BLE waking camera");
        // Some firmware wakes before it acknowledges the write.
        let _ = port.write(CAMERA_POWER, &[0x01]).await;
    }

    app_info!("BLE starting access point");
    let ap_ready = match start_access_point(port).await {
        Ok(ready) => ready,
        Err(LinkError::Cancelled) => return Err(LinkError::Cancelled),
        Err(error) => {
            // Some camera firmware drops BLE while bringing up its AP. The
            // credentials are already available; HTTP will verify the link.
            app_info!("BLE AP start failed; continuing to Wi-Fi: {error:?}");
            false
        }
    };
    port.unsubscribe(NETWORK_TYPE);
    // The shared connect flow waits out the learned access point warm-up.
    Ok(BleSetup {
        peripheral_id: peripheral_id.to_owned(),
        model,
        powered_on_already,
        credentials: WifiCredentials { ssid, passphrase },
        ap_ready,
    })
}

/// Write Network Type 0x01 until the camera notifies 0x01. Without
/// notifications the write is still sent, as many times, but no notify can
/// tell that the access point is up.
async fn start_access_point<P: GattPort>(port: &mut P) -> Result<bool, LinkError> {
    let subscribed = match port.subscribe(NETWORK_TYPE).await {
        Ok(()) => true,
        Err(LinkError::Cancelled) => return Err(LinkError::Cancelled),
        Err(error) => {
            app_info!("BLE Network Type notify unavailable: {error:?}");
            false
        }
    };
    for attempt in 0..timing::NETWORK_TYPE_NOTIFY_ATTEMPTS {
        port.write(NETWORK_TYPE, &[0x01]).await?;
        if subscribed
            && port
                .wait_notify(NETWORK_TYPE, 0x01, timing::NETWORK_TYPE_NOTIFY_TIMEOUT)
                .await?
        {
            return Ok(true);
        }
        if attempt + 1 < timing::NETWORK_TYPE_NOTIFY_ATTEMPTS {
            port.pause(timing::NETWORK_TYPE_RETRY_DELAY).await;
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A scripted link: reads answer in order, notify waits answer in order.
    struct Script {
        reads: VecDeque<Vec<u8>>,
        subscribe: Result<(), LinkError>,
        notifies: VecDeque<bool>,
        log: Vec<String>,
    }

    impl Script {
        fn new() -> Self {
            Self {
                reads: VecDeque::from([b"GR_AP\0".to_vec(), b"secret\0\0".to_vec()]),
                subscribe: Ok(()),
                notifies: VecDeque::new(),
                log: Vec::new(),
            }
        }
    }

    fn name(uuid: &str) -> &'static str {
        match uuid {
            SSID => "ssid",
            PASSPHRASE => "passphrase",
            CAMERA_POWER => "power",
            NETWORK_TYPE => "network",
            _ => "other",
        }
    }

    impl GattPort for Script {
        fn connect(&mut self) -> BoxFut<'_, Result<(), LinkError>> {
            self.log.push("connect".into());
            Box::pin(async { Ok(()) })
        }
        fn read(&mut self, c: &'static str) -> BoxFut<'_, Result<Vec<u8>, LinkError>> {
            self.log.push(format!("read {}", name(c)));
            let value = self.reads.pop_front().unwrap_or_default();
            Box::pin(async { Ok(value) })
        }
        fn write(
            &mut self,
            c: &'static str,
            v: &'static [u8],
        ) -> BoxFut<'_, Result<(), LinkError>> {
            self.log.push(format!("write {} {v:?}", name(c)));
            Box::pin(async { Ok(()) })
        }
        fn subscribe(&mut self, c: &'static str) -> BoxFut<'_, Result<(), LinkError>> {
            self.log.push(format!("subscribe {}", name(c)));
            let result = self.subscribe.clone();
            Box::pin(async { result })
        }
        fn wait_notify(
            &mut self,
            c: &'static str,
            _value: u8,
            _timeout: Duration,
        ) -> BoxFut<'_, Result<bool, LinkError>> {
            self.log.push(format!("wait {}", name(c)));
            let notified = self.notifies.pop_front().unwrap_or(false);
            Box::pin(async move { Ok(notified) })
        }
        fn unsubscribe(&mut self, c: &'static str) {
            self.log.push(format!("unsubscribe {}", name(c)));
        }
        fn pause(&mut self, _duration: Duration) -> BoxFut<'_, ()> {
            self.log.push("pause".into());
            Box::pin(async {})
        }
    }

    fn run(script: &mut Script) -> Result<BleSetup, LinkError> {
        use std::task::{Context, Poll, Waker};
        let mut fut = Box::pin(run_ble_setup(script, "id", CameraModel::Gr3, true));
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn a_failed_subscribe_still_starts_the_access_point() {
        let mut s = Script::new();
        s.subscribe = Err(LinkError::BleConnectFailed);
        let setup = run(&mut s).unwrap();
        assert!(!setup.ap_ready);
        assert_eq!(setup.credentials.ssid, "GR_AP");
        assert_eq!(
            s.log[3..],
            [
                "subscribe network",
                "write network [1]",
                "pause",
                "write network [1]",
                "unsubscribe network",
            ]
        );
    }

    #[test]
    fn a_notify_on_the_second_write_means_the_access_point_is_up() {
        let mut s = Script::new();
        s.notifies = VecDeque::from([false, true]);
        assert!(run(&mut s).unwrap().ap_ready);
        assert_eq!(
            s.log[4..],
            [
                "write network [1]",
                "wait network",
                "pause",
                "write network [1]",
                "wait network",
                "unsubscribe network",
            ]
        );
    }

    #[test]
    fn empty_credentials_mean_not_paired() {
        let mut s = Script::new();
        s.reads = VecDeque::from([vec![0; 32], b"secret".to_vec()]);
        assert_eq!(run(&mut s), Err(LinkError::NotPaired));
        assert_eq!(s.log, ["connect", "read ssid", "read passphrase"]);
    }
}
