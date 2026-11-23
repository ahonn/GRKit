//! Local Network permission check: a Bonjour self-loop. The app publishes a
//! private service and browses for it; the browser finds it only when local
//! network access is granted, and waits with a policy-denied DNS error when
//! it is denied. A connection to the camera cannot tell this reliably: it
//! reports the denial only while a host answers at the address.

use std::ffi::{CString, c_char, c_void};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use block2::{Block, RcBlock};
use dispatch2::DispatchQueue;
use futures_channel::oneshot;
use futures_util::future::{Either, select};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_foundation::{NSArray, NSString};

use crate::domain::LocalNetworkAccess;

/// The private service type. It must be listed in the Info.plist
/// `NSBonjourServices`, or the system rejects both the listener and the browser.
const SERVICE_TYPE: &str = "_grkit_probe._tcp";
/// Without a verdict by then (the permission prompt is still open, or the
/// framework stalled) the access is unknown.
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);

const NW_LISTENER_STATE_FAILED: u32 = 3;
const NW_BROWSER_STATE_WAITING: u32 = 4;
const NW_BROWSE_RESULT_CHANGE_RESULT_ADDED: u64 = 0x02;
const NW_ERROR_DOMAIN_DNS: u32 = 2;
const DNS_SERVICE_ERR_POLICY_DENIED: i32 = -65570;

type NwObject = *mut c_void;

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {
    static _nw_parameters_configure_protocol_disable: NwObject;
    static _nw_parameters_configure_protocol_default_configuration: NwObject;

    fn nw_parameters_create() -> NwObject;
    fn nw_parameters_create_secure_tcp(
        configure_tls: NwObject,
        configure_tcp: NwObject,
    ) -> NwObject;
    fn nw_parameters_set_include_peer_to_peer(parameters: NwObject, include: bool);

    fn nw_listener_create(parameters: NwObject) -> NwObject;
    fn nw_advertise_descriptor_create_bonjour_service(
        name: *const c_char,
        kind: *const c_char,
        domain: *const c_char,
    ) -> NwObject;
    fn nw_listener_set_advertise_descriptor(listener: NwObject, descriptor: NwObject);
    fn nw_listener_set_queue(listener: NwObject, queue: &DispatchQueue);
    fn nw_listener_set_new_connection_handler(
        listener: NwObject,
        handler: &Block<dyn Fn(NwObject)>,
    );
    fn nw_listener_set_state_changed_handler(
        listener: NwObject,
        handler: &Block<dyn Fn(u32, NwObject)>,
    );
    fn nw_listener_start(listener: NwObject);
    fn nw_listener_cancel(listener: NwObject);

    fn nw_browse_descriptor_create_bonjour_service(
        kind: *const c_char,
        domain: *const c_char,
    ) -> NwObject;
    fn nw_browser_create(descriptor: NwObject, parameters: NwObject) -> NwObject;
    fn nw_browser_set_queue(browser: NwObject, queue: &DispatchQueue);
    fn nw_browser_set_state_changed_handler(
        browser: NwObject,
        handler: &Block<dyn Fn(u32, NwObject)>,
    );
    fn nw_browser_set_browse_results_changed_handler(
        browser: NwObject,
        // The last argument is the C `bool` batch-complete flag.
        handler: &Block<dyn Fn(NwObject, NwObject, u8)>,
    );
    fn nw_browse_result_get_changes(old_result: NwObject, new_result: NwObject) -> u64;
    fn nw_browser_start(browser: NwObject);
    fn nw_browser_cancel(browser: NwObject);

    fn nw_error_get_error_domain(error: NwObject) -> u32;
    fn nw_error_get_error_code(error: NwObject) -> i32;
    fn nw_release(object: NwObject);
}

type Verdict = Arc<Mutex<Option<oneshot::Sender<LocalNetworkAccess>>>>;

fn resolve(verdict: &Verdict, access: LocalNetworkAccess) {
    if let Some(tx) = verdict.lock().unwrap().take() {
        let _ = tx.send(access);
    }
}

/// Ask whether the app has local network access. The first call also shows
/// the system prompt. Bounded by `CHECK_TIMEOUT`.
pub async fn check_access() -> LocalNetworkAccess {
    if !service_declared() {
        log::warn!("local network check skipped: {SERVICE_TYPE} is not in NSBonjourServices");
        return LocalNetworkAccess::Unknown;
    }
    let (tx, rx) = oneshot::channel();
    let verdict: Verdict = Arc::new(Mutex::new(Some(tx)));
    let queue = DispatchQueue::new("grkit.local-network", None);
    let kind = CString::new(SERVICE_TYPE).expect("service type has no NUL");
    let name = CString::new(format!("grkit-{:x}", std::process::id() ^ nonce()))
        .expect("service name has no NUL");

    let listener_failed = {
        let verdict = verdict.clone();
        RcBlock::new(move |state: u32, _error: NwObject| {
            if state == NW_LISTENER_STATE_FAILED {
                log::warn!("local network listener failed");
                resolve(&verdict, LocalNetworkAccess::Unknown);
            }
        })
    };
    let ignore_connection = RcBlock::new(|_connection: NwObject| {});
    let browser_waiting = {
        let verdict = verdict.clone();
        RcBlock::new(move |state: u32, error: NwObject| {
            let policy_denied = state == NW_BROWSER_STATE_WAITING
                && !error.is_null()
                && unsafe { nw_error_get_error_domain(error) } == NW_ERROR_DOMAIN_DNS
                && unsafe { nw_error_get_error_code(error) } == DNS_SERVICE_ERR_POLICY_DENIED;
            if policy_denied {
                // The simulator has no local network privacy: a denial there
                // is a false positive.
                let access = if cfg!(target_abi = "sim") {
                    LocalNetworkAccess::Granted
                } else {
                    LocalNetworkAccess::Denied
                };
                resolve(&verdict, access);
            }
        })
    };
    let browser_found = {
        let verdict = verdict.clone();
        RcBlock::new(move |old: NwObject, new: NwObject, _batch_complete: u8| {
            let changes = unsafe { nw_browse_result_get_changes(old, new) };
            if changes & NW_BROWSE_RESULT_CHANGE_RESULT_ADDED != 0 {
                resolve(&verdict, LocalNetworkAccess::Granted);
            }
        })
    };

    // SAFETY: each object is created here, used only through the Network
    // framework calls below, cancelled before release, and released once.
    // The framework copies the handler blocks.
    let (listener, browser) = unsafe {
        let listener_parameters = nw_parameters_create_secure_tcp(
            _nw_parameters_configure_protocol_disable,
            _nw_parameters_configure_protocol_default_configuration,
        );
        let listener = nw_listener_create(listener_parameters);
        nw_release(listener_parameters);
        let advertise = nw_advertise_descriptor_create_bonjour_service(
            name.as_ptr(),
            kind.as_ptr(),
            std::ptr::null(),
        );
        if listener.is_null() || advertise.is_null() {
            log::warn!("local network listener could not be created");
            for object in [listener, advertise] {
                if !object.is_null() {
                    nw_release(object);
                }
            }
            return LocalNetworkAccess::Unknown;
        }
        nw_listener_set_advertise_descriptor(listener, advertise);
        nw_release(advertise);
        nw_listener_set_queue(listener, &queue);
        nw_listener_set_new_connection_handler(listener, &ignore_connection);
        nw_listener_set_state_changed_handler(listener, &listener_failed);

        let browse = nw_browse_descriptor_create_bonjour_service(kind.as_ptr(), std::ptr::null());
        let browser_parameters = nw_parameters_create();
        nw_parameters_set_include_peer_to_peer(browser_parameters, true);
        let browser = nw_browser_create(browse, browser_parameters);
        nw_release(browse);
        nw_release(browser_parameters);
        if !browser.is_null() {
            nw_browser_set_queue(browser, &queue);
            nw_browser_set_state_changed_handler(browser, &browser_waiting);
            nw_browser_set_browse_results_changed_handler(browser, &browser_found);
        }

        nw_listener_start(listener);
        if !browser.is_null() {
            nw_browser_start(browser);
        }
        (listener, browser)
    };

    let access = match select(rx, Box::pin(nsh_core::prelude::timer(CHECK_TIMEOUT))).await {
        Either::Left((Ok(access), _)) => access,
        _ => LocalNetworkAccess::Unknown,
    };
    // SAFETY: the objects were created above and are released exactly once.
    unsafe {
        nw_listener_cancel(listener);
        nw_release(listener);
        if !browser.is_null() {
            nw_browser_cancel(browser);
            nw_release(browser);
        }
    }
    if access == LocalNetworkAccess::Denied {
        log::warn!("local network access {access:?}");
    } else {
        app_info!("local network access {access:?}");
    }
    access
}

/// Whether the Info.plist declares the probe's service type.
fn service_declared() -> bool {
    let key = NSString::from_str("NSBonjourServices");
    // SAFETY: `mainBundle` and `objectForInfoDictionaryKey:` are plain
    // Foundation getters; the result is checked before it is used as an array.
    let services: Option<Retained<AnyObject>> = unsafe {
        let bundle: Retained<AnyObject> = msg_send![class!(NSBundle), mainBundle];
        msg_send![&*bundle, objectForInfoDictionaryKey: &*key]
    };
    let Some(services) = services else {
        return false;
    };
    let Some(services) = services.downcast_ref::<NSArray>() else {
        return false;
    };
    services.iter().any(|item| {
        item.downcast_ref::<NSString>()
            .is_some_and(|s| s.to_string() == SERVICE_TYPE)
    })
}

/// A per-check value, so two checks never publish the same service name.
fn nonce() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
}
