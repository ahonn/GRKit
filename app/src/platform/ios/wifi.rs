//! Join the camera access point via AccessorySetupKit (iOS 18+) or HotspotConfiguration.

use futures_util::future::{Either, select};
use nsh_core::platform::Lifecycle;
use nsh_core::prelude::{lifecycle, timer};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use futures_channel::oneshot;
use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_accessory_setup_kit::{
    ASAccessory, ASAccessoryEvent, ASAccessoryEventType, ASAccessorySession, ASAccessoryState,
    ASDiscoveryDescriptor, ASErrorCode, ASErrorDomain, ASPickerDisplayItem,
};
use objc2_foundation::{NSArray, NSBundle, NSError, NSProcessInfo, NSString};
use objc2_network_extension::{
    NEHotspotConfiguration, NEHotspotConfigurationError, NEHotspotConfigurationErrorDomain,
    NEHotspotConfigurationManager,
};
use objc2_ui_kit::{UIColor, UIImage, UIImageRenderingMode};

use crate::domain::connect_policy::{CAMERA_AP_PREFIX, NetworkSnapshot};
use crate::domain::{Join, LinkError, WifiCredentials};
use crate::platform::link_support;

/// Hard ceiling for one native join, in foreground time. Normal joins
/// finish in 5 to 15 s; past this the system call is wedged.
const JOIN_TIMEOUT: Duration = Duration::from_secs(30);
const JOIN_DEADLINE_POLL: Duration = Duration::from_millis(250);
const JOIN_HTTP_PROBE_INTERVAL: Duration = Duration::from_millis(600);
/// The accessory picker gives up after this long and fails the join.
const ASK_AUTHORIZATION_LIMIT: Duration = Duration::from_secs(120);
/// After a forced removal, the old association needs time to detach before
/// a same-SSID join can redo the handshake instead of reusing it.
const REASSOCIATE_SETTLE_POLL: Duration = Duration::from_millis(500);
const REASSOCIATE_SETTLE_TIMEOUT: Duration = Duration::from_secs(3);
/// The interface cannot show association while the default route is
/// elsewhere, so the removal always gets this floor first.
const REASSOCIATE_SETTLE_MIN: Duration = Duration::from_secs(1);

struct PendingJoin {
    id: u64,
    joining: bool,
    ssid: String,
    passphrase: String,
    /// The camera model the accessory picker names, such as "RICOH GR III".
    name: String,
    /// The picker reported this accessory authorized.
    selected: bool,
    /// The picker could not set up the accessory.
    picker_failed: bool,
    reply: Option<oneshot::Sender<Result<(), LinkError>>>,
}

struct AskState {
    session: Retained<ASAccessorySession>,
    activated: bool,
    next_id: u64,
    picker_id: Option<u64>,
    pending: Option<PendingJoin>,
    activate_waiters: Vec<oneshot::Sender<()>>,
}

/// One session for the app lifetime; activated once and reused for each join.
static ASK: OnceLock<Mutex<AskState>> = OnceLock::new();

/// Join the camera access point. A failure HTTP may still disprove is a
/// soft failure; the flow probes before it shows an error.
pub async fn join(creds: &WifiCredentials, name: &str, force: bool) -> Result<Join, LinkError> {
    if force && !reassociate_settled(&creds.ssid).await {
        log::warn!("Wi-Fi reassociation did not settle");
        return Ok(Join::SoftFailed(LinkError::WifiJoinFailed));
    }
    match join_once(creds, name).await {
        Ok(()) => Ok(Join::Joined),
        Err(error) if error.is_soft_join_failure() => Ok(Join::SoftFailed(error)),
        Err(error) => Err(error),
    }
}

async fn join_once(creds: &WifiCredentials, name: &str) -> Result<(), LinkError> {
    if creds.ssid.is_empty() {
        return Err(LinkError::WifiJoinRejected);
    }
    app_info!("Wi-Fi join start ssid_len={}", creds.ssid.len());
    if ios_major() >= 18 {
        // ASK consent is authoritative. Never bypass a denial with another API.
        return join_accessory_setup(creds, name).await;
    }
    let result = join_hotspot_configuration(creds).await;
    match &result {
        Ok(()) => app_info!("Wi-Fi join via HotspotConfiguration ok"),
        Err(e) => log::warn!("Wi-Fi join failed: {e:?}"),
    }
    result
}

async fn join_hotspot_configuration(creds: &WifiCredentials) -> Result<(), LinkError> {
    let (tx, rx) = oneshot::channel::<Result<(), LinkError>>();
    let ssid = creds.ssid.clone();
    let passphrase = creds.passphrase.clone();
    DispatchQueue::main().exec_async(move || {
        // Cancellation may happen before the main queue starts this operation.
        if tx.is_canceled() {
            return;
        }
        let ssid_ns = NSString::from_str(&ssid);
        let pass_ns = NSString::from_str(&passphrase);
        let config = unsafe {
            NEHotspotConfiguration::initWithSSID_passphrase_isWEP(
                NEHotspotConfiguration::alloc(),
                &ssid_ns,
                &pass_ns,
                false,
            )
        };
        unsafe { config.setJoinOnce(false) };
        let reply = Mutex::new(Some(tx));
        let handler = RcBlock::new(move |error: *mut NSError| {
            let Some(tx) = reply.lock().unwrap().take() else {
                return;
            };
            let _ = tx.send(map_hotspot_error(error));
        });
        unsafe {
            NEHotspotConfigurationManager::sharedManager()
                .applyConfiguration_completionHandler(&config, Some(&handler));
        }
    });
    wait_join(rx, || true).await
}

/// Dropping an attempt detaches its waiter; native callbacks retain their own id.
struct JoinLease(u64);
impl Drop for JoinLease {
    fn drop(&mut self) {
        fail_pending(self.0, LinkError::Cancelled);
    }
}

async fn wait_join(
    rx: oneshot::Receiver<Result<(), LinkError>>,
    is_joining: impl Fn() -> bool,
) -> Result<(), LinkError> {
    let deadline = async {
        let mut foreground = Duration::ZERO;
        loop {
            let active = lifecycle() == Lifecycle::Active;
            timer(JOIN_DEADLINE_POLL).await;
            // Exclude consent time and suspension behind system dialogs.
            if is_joining() && active && lifecycle() == Lifecycle::Active {
                foreground += JOIN_DEADLINE_POLL;
                if foreground >= JOIN_TIMEOUT {
                    return Err(LinkError::WifiJoinTimeout);
                }
            }
        }
    };
    let http_proof = async {
        loop {
            // Never let an HTTP answer bypass the ASK consent sheet.
            if is_joining()
                && lifecycle() == Lifecycle::Active
                && crate::platform::http::service_ready().await
            {
                return Ok(());
            }
            timer(JOIN_HTTP_PROBE_INTERVAL).await;
        }
    };
    match select(
        Box::pin(rx),
        Box::pin(select(Box::pin(deadline), Box::pin(http_proof))),
    )
    .await
    {
        Either::Left((result, _)) => result.unwrap_or(Err(LinkError::Cancelled)),
        Either::Right((Either::Left((result, _)) | Either::Right((result, _)), _)) => result,
    }
}

async fn join_accessory_setup(creds: &WifiCredentials, name: &str) -> Result<(), LinkError> {
    if lifecycle() != Lifecycle::Active {
        log::warn!("Wi-Fi accessory setup needs the foreground");
        return Err(LinkError::WifiJoinRejected);
    }
    ensure_ask_activated().await?;
    let (tx, rx) = oneshot::channel::<Result<(), LinkError>>();
    let id = {
        let mut state = ask_state().lock().unwrap();
        // A previous picker may still be on screen after app-side cancellation.
        if state.pending.is_some() || state.picker_id.is_some() {
            return Err(LinkError::Busy);
        }
        state.next_id = state.next_id.wrapping_add(1);
        let id = state.next_id;
        state.pending = Some(PendingJoin {
            id,
            joining: false,
            ssid: creds.ssid.clone(),
            passphrase: creds.passphrase.clone(),
            name: name.to_owned(),
            selected: false,
            picker_failed: false,
            reply: Some(tx),
        });
        id
    };
    let _lease = JoinLease(id);
    DispatchQueue::main().exec_async(move || start_authorize_or_join(id));
    let pending_is = |joining: bool| {
        ask_state()
            .lock()
            .unwrap()
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == id && pending.joining == joining)
    };
    let joined = wait_join(rx, || pending_is(true));
    // An authorization still open at the limit fails; the join itself has
    // its own foreground deadline.
    let authorization_limit = async {
        timer(ASK_AUTHORIZATION_LIMIT).await;
        if pending_is(false) {
            log::warn!("Wi-Fi accessory authorization timed out");
            return Err(LinkError::WifiJoinRejected);
        }
        std::future::pending().await
    };
    match select(Box::pin(joined), Box::pin(authorization_limit)).await {
        Either::Left((result, _)) | Either::Right((result, _)) => result,
    }
}

fn ask_state() -> &'static Mutex<AskState> {
    ASK.get_or_init(|| {
        let session = unsafe { ASAccessorySession::new() };
        Mutex::new(AskState {
            session,
            activated: false,
            next_id: 0,
            picker_id: None,
            pending: None,
            activate_waiters: Vec::new(),
        })
    })
}

async fn ensure_ask_activated() -> Result<(), LinkError> {
    let (need_activate, waiter) = {
        let mut state = ask_state().lock().unwrap();
        if state.activated {
            return Ok(());
        }
        let (tx, rx) = oneshot::channel();
        // Timed-out or cancelled callers must not suppress a fresh activation.
        state
            .activate_waiters
            .retain(|waiter| !waiter.is_canceled());
        state.activate_waiters.push(tx);
        let need = state.activate_waiters.len() == 1;
        (need, rx)
    };
    if need_activate {
        DispatchQueue::main().exec_async(|| {
            activate_ask_session();
        });
    }
    crate::platform::with_timeout(waiter, Duration::from_secs(10))
        .await
        .ok_or(LinkError::WifiJoinRejected)?
        .map_err(|_| LinkError::Cancelled)?;
    Ok(())
}

fn activate_ask_session() {
    let session = {
        let state = ask_state().lock().unwrap();
        state.session.clone()
    };

    let handler = RcBlock::new(move |event_ptr: std::ptr::NonNull<ASAccessoryEvent>| {
        let event = unsafe { event_ptr.as_ref() };
        let event_type = unsafe { event.eventType() };
        if event_type == ASAccessoryEventType::Activated {
            let waiters = {
                let mut state = ask_state().lock().unwrap();
                state.activated = true;
                std::mem::take(&mut state.activate_waiters)
            };
            for w in waiters {
                let _ = w.send(());
            }
        } else if event_type == ASAccessoryEventType::AccessoryAdded
            || event_type == ASAccessoryEventType::AccessoryChanged
        {
            if let Some(accessory) = unsafe { event.accessory() } {
                note_picker_selection(&accessory);
            }
        } else if event_type == ASAccessoryEventType::PickerSetupFailed {
            let mut state = ask_state().lock().unwrap();
            let picker_id = state.picker_id;
            if let Some(pending) = state.pending.as_mut().filter(|p| Some(p.id) == picker_id) {
                pending.picker_failed = true;
            }
        } else if event_type == ASAccessoryEventType::PickerDidDismiss {
            let id = ask_state().lock().unwrap().picker_id.take();
            if let Some(id) = id {
                finish_after_picker(id);
            }
        } else if event_type == ASAccessoryEventType::Invalidated {
            // Should not happen for a lifetime session; fail any in-flight join.
            let id = ask_state().lock().unwrap().pending.as_ref().map(|p| p.id);
            if let Some(id) = id {
                fail_pending(id, LinkError::WifiJoinRejected);
            }
            let mut state = ask_state().lock().unwrap();
            state.activated = false;
            state.picker_id = None;
            state.activate_waiters.clear();
        }
    });

    unsafe {
        session.activateWithQueue_eventHandler(DispatchQueue::main(), &handler);
    }
}

fn start_authorize_or_join(id: u64) {
    let (session, ssid, passphrase, name) = {
        let state = ask_state().lock().unwrap();
        if !state.activated {
            return;
        }
        let Some(pending) = state.pending.as_ref().filter(|pending| pending.id == id) else {
            return;
        };
        (
            state.session.clone(),
            pending.ssid.clone(),
            pending.passphrase.clone(),
            pending.name.clone(),
        )
    };

    if let Some(accessory) = find_authorized(&session, &ssid) {
        finish_join(id, &accessory, &passphrase);
        return;
    }

    let descriptor = unsafe { ASDiscoveryDescriptor::new() };
    let ssid_ns = NSString::from_str(&ssid);
    unsafe { descriptor.setSSID(Some(&ssid_ns)) };
    let Some(image) = picker_image() else {
        fail_pending(id, LinkError::WifiJoinRejected);
        return;
    };
    let item = unsafe {
        ASPickerDisplayItem::initWithName_productImage_descriptor(
            ASPickerDisplayItem::alloc(),
            &NSString::from_str(if name.is_empty() { &ssid } else { &name }),
            &image,
            &descriptor,
        )
    };
    let items = NSArray::from_retained_slice(&[item]);
    ask_state().lock().unwrap().picker_id = Some(id);
    let picker_done = RcBlock::new(move |error: *mut NSError| {
        if error.is_null() {
            return;
        }
        let error = picker_error(unsafe { &*error });
        {
            // The first of this callback and the dismissal decides.
            let mut state = ask_state().lock().unwrap();
            if state.picker_id != Some(id) {
                return;
            }
            state.picker_id = None;
        }
        fail_pending(id, error);
    });
    unsafe {
        session.showPickerForDisplayItems_completionHandler(&items, &picker_done);
    }
}

/// The picker closed: join when the user selected the camera, fail when
/// the picker could not set it up, and treat anything else as a cancel.
/// Joining only after dismissal keeps the next system sheet in front.
fn finish_after_picker(id: u64) {
    let (session, ssid, passphrase, selected, picker_failed) = {
        let state = ask_state().lock().unwrap();
        let Some(pending) = state.pending.as_ref().filter(|p| p.id == id && !p.joining) else {
            return;
        };
        (
            state.session.clone(),
            pending.ssid.clone(),
            pending.passphrase.clone(),
            pending.selected,
            pending.picker_failed,
        )
    };
    let accessory = find_authorized(&session, &ssid);
    if selected || accessory.is_some() {
        match accessory {
            Some(accessory) => finish_join(id, &accessory, &passphrase),
            None => fail_pending(id, LinkError::WifiJoinRejected),
        }
    } else if picker_failed {
        fail_pending(id, LinkError::WifiJoinRejected)
    } else {
        fail_pending(id, LinkError::Cancelled)
    }
}

/// Record whether the picker authorized the camera this join waits for.
fn note_picker_selection(accessory: &ASAccessory) {
    let ssid = unsafe { accessory.SSID() }.map(|s| s.to_string());
    let authorized = unsafe { accessory.state() } == ASAccessoryState::Authorized;
    let mut state = ask_state().lock().unwrap();
    let picker_id = state.picker_id;
    if let Some(pending) = state
        .pending
        .as_mut()
        .filter(|p| Some(p.id) == picker_id && ssid.as_deref() == Some(p.ssid.as_str()))
    {
        pending.selected = authorized;
    }
}

/// A user cancel closes the attempt quietly; every other picker error is a
/// terminal join failure (a denial is never bypassed with another prompt).
fn picker_error(error: &NSError) -> LinkError {
    let domain = error.domain().to_string();
    let accessory_error = domain == unsafe { ASErrorDomain }.to_string();
    if accessory_error && error.code() == ASErrorCode::UserCancelled.0 {
        return LinkError::Cancelled;
    }
    log::warn!("Wi-Fi accessory picker failed: {domain} {}", error.code());
    LinkError::WifiJoinRejected
}

fn find_authorized(session: &ASAccessorySession, ssid: &str) -> Option<Retained<ASAccessory>> {
    let accessories = unsafe { session.accessories() };
    for accessory in accessories {
        let accessory_ssid = unsafe { accessory.SSID() }
            .map(|s| s.to_string())
            .unwrap_or_default();
        let state = unsafe { accessory.state() };
        if accessory_ssid == ssid && state == ASAccessoryState::Authorized {
            return Some(accessory);
        }
    }
    None
}

fn finish_join(id: u64, accessory: &ASAccessory, passphrase: &str) {
    let pass_ns = NSString::from_str(passphrase);
    {
        let mut state = ask_state().lock().unwrap();
        let Some(pending) = state.pending.as_mut().filter(|p| p.id == id && !p.joining) else {
            return;
        };
        pending.joining = true;
    }
    let done = RcBlock::new(move |error: *mut NSError| {
        finish_pending(id, map_hotspot_error(error));
    });
    let manager = unsafe { NEHotspotConfigurationManager::sharedManager() };
    if passphrase.is_empty() {
        unsafe {
            manager.joinAccessoryHotspotWithoutSecurity_completionHandler(accessory, Some(&done))
        };
    } else {
        unsafe {
            manager.joinAccessoryHotspot_passphrase_completionHandler(
                accessory,
                &pass_ns,
                Some(&done),
            )
        };
    }
}

fn finish_pending(id: u64, result: Result<(), LinkError>) {
    let mut state = ask_state().lock().unwrap();
    if state.pending.as_ref().is_some_and(|p| p.id == id)
        && let Some(mut pending) = state.pending.take()
        && let Some(tx) = pending.reply.take()
    {
        let _ = tx.send(result);
    }
}

fn fail_pending(id: u64, err: LinkError) {
    finish_pending(id, Err(err));
}

/// Classify a NetworkExtension join result. Wrong password, bad network
/// name, invalid configuration and a denied or unauthorized join are
/// terminal; any other failure may still have reached the camera.
fn map_hotspot_error(error: *mut NSError) -> Result<(), LinkError> {
    use NEHotspotConfigurationError as E;
    if error.is_null() {
        return Ok(());
    }
    let error = unsafe { &*error };
    let domain = error.domain().to_string();
    if domain != unsafe { NEHotspotConfigurationErrorDomain }.to_string() {
        log::warn!("Wi-Fi join error {domain} {}", error.code());
        return Err(LinkError::WifiJoinFailed);
    }
    match E(error.code()) {
        E::AlreadyAssociated => Ok(()),
        E::UserDenied => Err(LinkError::WifiJoinDenied),
        E::InvalidWPAPassphrase | E::InvalidWEPPassphrase => Err(LinkError::WifiWrongPassword),
        E::InvalidSSID | E::InvalidSSIDPrefix => Err(LinkError::WifiInvalidSsid),
        E::Invalid => Err(LinkError::WifiInvalidConfig),
        E::ApplicationIsNotInForeground | E::UserUnauthorized | E::SystemDenied => {
            Err(LinkError::WifiJoinRejected)
        }
        _ => Err(LinkError::WifiJoinFailed),
    }
}

fn ios_major() -> u32 {
    NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion as u32
}

/// Forced reassociation: forget the saved camera network, then wait until
/// the old association has detached, so the join that follows redoes the
/// handshake instead of resolving at once on the current network. False when
/// the interface still looks attached at the end of the settle window.
async fn reassociate_settled(ssid: &str) -> bool {
    app_info!("Wi-Fi reassociate start");
    remove_configuration(ssid);
    let deadline = std::time::Instant::now() + REASSOCIATE_SETTLE_TIMEOUT;
    timer(REASSOCIATE_SETTLE_MIN).await;
    loop {
        if !attached_after_removal(link_support::network_snapshot()) {
            app_info!("Wi-Fi reassociate settled");
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        timer(REASSOCIATE_SETTLE_POLL).await;
    }
}

/// A camera lease or a link-local address can still be the removed
/// association. An interface without an IPv4 address counts as detached:
/// the interface list cannot tell an association without a lease apart
/// from no association.
fn attached_after_removal(snapshot: NetworkSnapshot) -> bool {
    snapshot.address.is_some_and(|address| {
        let octets = address.octets();
        octets[..3] == CAMERA_AP_PREFIX || address.is_link_local()
    })
}

/// Forget the saved camera network.
fn remove_configuration(ssid: &str) {
    let ssid = ssid.to_owned();
    DispatchQueue::main().exec_async(move || unsafe {
        NEHotspotConfigurationManager::sharedManager()
            .removeConfigurationForSSID(&NSString::from_str(&ssid));
    });
}

/// The camera in the accessory picker: the GR III line art of the app's
/// assets, in the color of the text, as the camera tab draws it. The Wi-Fi
/// symbol stands in when the file is missing.
fn picker_image() -> Option<Retained<UIImage>> {
    let art = NSBundle::mainBundle()
        .resourcePath()
        .map(|root| format!("{root}/nsh/assets/@grkit-app/art-gr3.png"))
        .and_then(|path| UIImage::imageWithContentsOfFile(&NSString::from_str(&path)));
    match art {
        Some(art) => Some(art.imageWithTintColor_renderingMode(
            &UIColor::labelColor(),
            UIImageRenderingMode::AlwaysOriginal,
        )),
        None => UIImage::systemImageNamed(&NSString::from_str("wifi")),
    }
}
