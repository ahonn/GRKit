//! App state: tabs, camera session, update sheets. The camera, the release
//! site and the phone's records come through `camera_api::CameraApi`.
//!
//! Reactive fields use NSH `Signal`s so the UI updates when they change.

mod alerts;
mod camera_settings;
mod changes;
mod connect;
#[cfg(demo)]
mod demo_hooks;
mod liveness;
mod looks;
mod modules;
pub mod native_looks;
pub mod page_refresh;
mod recipes;
mod rename;
#[cfg(demo)]
mod simulated_link;
mod update;
mod warmup;

pub use alerts::{Alert, AlertRequest};
pub use camera_settings::{PictureRead, SettingsRead};
#[cfg(demo)]
pub(crate) use looks::off_thread;
pub use looks::{CatalogItem, ImportDraft, LookAction, LooksStore};
pub use modules::{ModulePart, ModulesBlock, ModulesStore};
pub use recipes::{RecipeAction, RecipesStore, native_type_name};

use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Waker},
    time::{Duration, Instant},
};

use nsh_core::prelude::*;

use crate::camera_api::{CameraApi, ReleaseOffer};
use crate::domain::auto_connect::{AutoTrigger, LeaseWatch};
use crate::domain::link_health::{LinkHealth, LinkState};
use crate::domain::nearby_watch::{AdvertisedPower, NearbyWatch, PRUNE_INTERVAL_MS};
use crate::domain::{
    CameraModel, CameraStatus, ConnectionState, LinkError, NearbyCamera, PackageInfo,
    RememberedCamera, UpdateError, UpdateKind, UpdatePhase,
};
use crate::platform;

/// Placeholder App Store id until the listing exists.
pub const APP_STORE_ID: &str = "0000000000";
pub const PRIVACY_URL: &str = "https://grkit.app/app/privacy/";
/// The site, which installs GRKit on a memory card.
pub const INSTALL_URL: &str = "https://grkit.app/";
/// The public feedback board.
pub const HELP_URL: &str = "https://grkit.userjot.com";
/// GR Snap: its App Store page on iOS, its site elsewhere.
pub const GR_SNAP_URL: &str = if cfg!(target_os = "android") {
    "https://grsnap.app"
} else {
    "https://apps.apple.com/app/gr-snap-ricoh-gr-wireless/id6768040383"
};
/// Every GRKit release.
pub const RELEASES_URL: &str = "https://github.com/ahonn/GRKit/releases";
pub const APP_VERSION: &str = "1.0.0";
pub const APP_BUILD: &str = "1";

pub(crate) const MAX_DEPTH: usize = 4;

/// Which tab is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Camera,
    Looks,
    Recipes,
    Settings,
}

/// Pages pushed on a tab's navigation stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    SoftwareUpdate,
    /// The notes of earlier GRKit versions, from Software Update.
    ReleaseHistory,
    /// One Look on the camera, by ID.
    LookDetail(String),
    /// Edit the cover of a Look.
    LookCover(String),
    /// Official Looks that the camera does not have, and the import.
    LookAdd,
    /// A Look made from a file on the phone.
    LookImport,
    /// Edit the cover of the Look being imported.
    LookImportCover,
    /// The saved cameras, from Settings.
    SavedCameras,
    /// One recipe on the camera, by ID.
    RecipeDetail(u32),
    /// The presets of the site.
    RecipePresets,
    /// One preset, by ID.
    RecipePreset(String),
    /// The modules of the camera, from the camera tab.
    Modules,
    /// One module of the camera, by ID.
    ModuleDetail(String),
    /// Which values of a module ADJ shows, and their order.
    ModuleOptions(String),
    /// GRKit Settings pages of the Settings tab, as the camera's MENU has them.
    ShootingSettings,
    DisplaySettings,
    ShutdownScreen,
    /// The picture made from a picked photo, before it goes to the camera.
    ShutdownCrop,
    /// The film looks of another GR camera, made from its firmware file.
    LookUnlock(native_looks::Firmware),
}

/// Modal sheets over the camera tab.
#[derive(Clone, Debug, PartialEq)]
pub enum Sheet {
    Nearby,
    Scan,
    Connecting {
        step: u8,
    },
    ConnectError(LinkError),
    Sending,
    Confirm,
    Restart,
    Installed {
        version: String,
        previous: String,
    },
    Error(UpdateError),
    /// A confirmed update did not install: GRKit does not answer after the
    /// restart.
    InstallFailed,
    /// Choose what kind of file becomes a new Look.
    ImportLook,
}

/// The package assembled for a target version: (version, zip bytes).
type StagedPackage = (String, Vec<u8>);

/// The update task of this phone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateRun {
    Idle,
    Running,
    /// The person cancelled it: the task ends it.
    Cancelling,
}

/// Shared app state provided at the root (`Copy` handle on signals).
#[derive(Clone, Copy)]
pub struct AppState {
    /// The camera is simulated (`CameraApi::simulated`): no Bluetooth or
    /// Wi-Fi link to it.
    demo: bool,
    /// The camera, the release site and the phone's records.
    api: Stored<Rc<dyn CameraApi>>,
    /// Alerts for the pages to show (`pages::present_alerts`).
    alerts: Stored<alerts::AlertQueue>,
    tab: Signal<Tab>,
    /// The last request to scroll a tab's root page to its top, with a
    /// count that makes each request new.
    scroll_top: Signal<Option<(Tab, u64)>>,
    stacks: Signal<[Vec<Route>; 4]>,
    pub(crate) looks: LooksStore,
    pub(crate) recipes: RecipesStore,
    pub(crate) modules: ModulesStore,
    camera_settings: camera_settings::CameraSettingsStore,
    native_looks: native_looks::NativeLooksStore,
    sheet: Signal<Option<Sheet>>,
    /// A name that was typed and not saved yet.
    name_draft: Signal<Option<(rename::NameTarget, String)>>,
    pub(crate) status: Signal<Option<CameraStatus>>,
    pub(crate) package: Signal<Option<PackageInfo>>,
    pub(crate) phase: Signal<UpdatePhase>,
    connect_started: Signal<Option<Instant>>,
    initial_connect: Signal<bool>,
    /// When true, the ticker does not advance a simulated connect.
    frozen: Signal<bool>,
    pub(crate) baseline: Signal<Option<String>>,
    nearby: Signal<Vec<NearbyCamera>>,
    /// The saved cameras, the most recent first; the first is the one a
    /// saved connect reaches.
    saved: Signal<Vec<RememberedCamera>>,
    selected_id: Signal<Option<String>>,
    connect_generation: Signal<u64>,
    connect_abort: Stored<RefCell<Option<futures_util::future::AbortHandle>>>,
    /// Suppresses the automatic nearby sheet until the camera powers off.
    nearby_watch: Stored<RefCell<NearbyWatch>>,
    /// A saved-camera connect waiting for this camera's advertisement.
    peek_waiter: Stored<RefCell<Option<connect::PeekWaiter>>>,
    /// The flow of the last attempt, which a retry repeats.
    attempt_kind: Signal<connect::AttemptKind>,
    /// A silent connect is running.
    auto_running: Signal<bool>,
    /// The user disconnected: no silent connect until the user connects again.
    auto_suppressed: Signal<bool>,
    /// The user went back from the one camera found to the camera list.
    scan_list_pinned: Signal<bool>,
    /// Camera leases seen while disconnected.
    lease: Stored<RefCell<LeaseWatch>>,
    /// The probe policy of the current camera session.
    health: Stored<RefCell<LinkHealth>>,
    link_state: Signal<LinkState>,
    /// Short notices at the bottom of the screen.
    toasts: crate::ui::Toasts,
    /// How far up from the bottom of the content the Connect page's buttons
    /// reach, so notices show above them.
    connect_foot: Signal<f32>,
    /// The tab a lost link moved the user from, restored after a reconnect.
    return_tab: Signal<Option<Tab>>,
    /// When the change stream last got a frame; `None` while it is closed.
    stream_heard: Stored<Cell<Option<Instant>>>,
    /// The app root's owner. Tasks that must outlive the sheet or page that
    /// starts them (a connect, an update) run under it: a task made with
    /// `spawn` ends with the owner it was made under, and a sheet's owner is
    /// disposed as soon as the sheet changes.
    root: Owner,
    update_run: Signal<UpdateRun>,
    update_reconnect: Signal<bool>,
    update_kind: Signal<UpdateKind>,
    release_busy: Signal<bool>,
    /// The phone is turning the camera off for the person.
    powering_off: Signal<bool>,
    /// The phone restarts the camera so it reads changed Look files.
    camera_restarting: Signal<bool>,
    /// The connect that ends a restart for changed Looks is running.
    restart_connect: Signal<bool>,
    release_problem: Signal<Option<crate::domain::ReleaseProblem>>,
    release_notes: Signal<Vec<crate::domain::release_notes::VersionNotes>>,
    release: Stored<RefCell<Option<ReleaseOffer>>>,
    /// The phone restarts the camera for the update.
    restart_request: Signal<bool>,
    /// The person opened the update: its steps show in the sheet.
    update_shown: Signal<bool>,
    /// Why an update stopped after its send, for Software Update to say:
    /// the sheet closes once the camera has the update.
    update_notice: Signal<Option<UpdateError>>,
    /// The update journal of the connected camera, when one waits.
    journal: Signal<Option<crate::domain::JournalState>>,
    /// The package assembled for the update target, kept for a send again.
    staged: Stored<RefCell<Option<StagedPackage>>>,
}

impl AppState {
    /// The app state on `api`: a real camera starts the link watches, a
    /// simulated one waits for the demo to drive it.
    pub fn new(api: Rc<dyn CameraApi>) -> Self {
        let demo = api.simulated().is_some();
        let dev_url = dev_camera_url() && !demo;
        let saved = api.saved_cameras();
        let app = Self {
            demo,
            api: stored(api),
            alerts: stored(alerts::AlertQueue::new()),
            update_run: signal(UpdateRun::Idle),
            update_reconnect: signal(false),
            update_kind: signal(UpdateKind::Install),
            release_busy: signal(false),
            powering_off: signal(false),
            camera_restarting: signal(false),
            restart_connect: signal(false),
            release_problem: signal(None),
            release_notes: signal(Vec::new()),
            release: stored(RefCell::new(None)),
            restart_request: signal(false),
            update_shown: signal(false),
            update_notice: signal(None),
            journal: signal(None),
            staged: stored(RefCell::new(None)),
            tab: signal(Tab::Camera),
            scroll_top: signal(None),
            stacks: signal([Vec::new(), Vec::new(), Vec::new(), Vec::new()]),
            looks: LooksStore::new(),
            recipes: RecipesStore::new(),
            modules: ModulesStore::new(),
            camera_settings: camera_settings::CameraSettingsStore::new(),
            native_looks: native_looks::NativeLooksStore::default(),
            sheet: signal(None),
            status: signal(None),
            package: signal(None),
            phase: signal(UpdatePhase::Idle),
            connect_started: signal(None),
            initial_connect: signal(dev_url),
            frozen: signal(false),
            baseline: signal(None),
            nearby: signal(Vec::new()),
            saved: signal(saved),
            selected_id: signal(None),
            connect_generation: signal(0),
            connect_abort: stored(RefCell::new(None)),
            root: Owner::current(),
            nearby_watch: stored(RefCell::new(NearbyWatch::new())),
            peek_waiter: stored(RefCell::new(None)),
            attempt_kind: signal(connect::AttemptKind::Device),
            auto_running: signal(false),
            auto_suppressed: signal(false),
            scan_list_pinned: signal(false),
            lease: stored(RefCell::new(LeaseWatch::new())),
            health: stored(RefCell::new(LinkHealth::default())),
            link_state: signal(LinkState::Idle),
            toasts: crate::ui::Toasts::new(),
            connect_foot: signal(0.0),
            name_draft: signal(None),
            return_tab: signal(None),
            stream_heard: stored(Cell::new(None)),
        };
        if demo {
            // The demo drives the simulated camera.
        } else if !dev_url {
            app.watch_link();
            app.watch_changes();
            app.start_ble_scan();
            // The phone may already be on the camera network.
            spawn(async move {
                next_frame().await;
                app.auto_connect(AutoTrigger::Launch);
            });
        } else {
            app.watch_changes();
            app_info!(
                "GRKIT_CAMERA_URL set; skipping BLE scan ({})",
                crate::camera_api::camera_http_base()
            );
            // The development URL connects at launch, so a simulator run
            // needs no tap to reach the HTTP part.
            spawn(async move {
                next_frame().await;
                app.open_scan();
            });
        }
        app
    }

    /// The camera, the release site and the phone's records.
    pub(crate) fn api(self) -> Rc<dyn CameraApi> {
        self.api.with(|api| api.clone())
    }

    /// Run `task` under the app root, so that it outlives the page that
    /// starts it. A task that needs no wait (the demo camera answers at
    /// once) ends before this returns, as a change on a camera in hand does.
    pub(crate) fn run(self, task: impl Future<Output = ()> + 'static) {
        self.root.run(|| {
            let mut task = Box::pin(task);
            let mut cx = Context::from_waker(Waker::noop());
            if task.as_mut().poll(&mut cx).is_pending() {
                spawn(task);
            }
        });
    }

    pub fn initial_connect(self) -> bool {
        self.initial_connect.get()
    }

    /// The camera tab's root is the Connect page: no camera is connected
    /// and no first connect runs.
    fn connect_page_shown(self) -> bool {
        !self.initial_connect() && !self.camera_online()
    }

    pub(crate) fn set_connect_foot(self, height: f32) {
        self.connect_foot.set(height);
    }

    /// How far up from the bottom of the content notices must stay clear
    /// of the page in front: the Connect page's buttons, else nothing.
    pub fn toast_floor(self) -> f32 {
        let connect_in_front = self.tab() == Tab::Camera
            && self.route_at(Tab::Camera, 0).is_none()
            && self.connect_page_shown();
        if connect_in_front {
            self.connect_foot.get()
        } else {
            0.0
        }
    }

    /// The camera is simulated: the store steps through its link.
    fn demo(self) -> bool {
        cfg!(demo) && self.demo
    }

    pub fn set_frozen(self, frozen: bool) {
        self.frozen.set(frozen);
    }

    pub fn frozen(self) -> bool {
        self.frozen.get()
    }

    /// The tab on screen. Without a connected camera, only the camera tab
    /// exists.
    /// The chosen tab, or Camera while that tab is not available.
    pub fn tab(self) -> Tab {
        let tab = self.tab.get();
        if self.tab_available(tab) {
            tab
        } else {
            Tab::Camera
        }
    }

    /// The tabs of the camera show with a connected camera, and stay through
    /// a restart for changed Looks and the connect that ends it. Camera and
    /// Settings always show.
    pub fn tabs_shown(self) -> bool {
        self.camera_online()
            || self.camera_restarting.get()
            || (self.restart_connect.get()
                && (self.connect_started.get().is_some()
                    || matches!(
                        self.sheet.get(),
                        Some(Sheet::Connecting { .. } | Sheet::ConnectError(_))
                    )))
    }

    pub fn set_tab(self, tab: Tab) {
        let tab = if self.tab_available(tab) {
            tab
        } else {
            Tab::Camera
        };
        self.tab.set(tab);
        if tab == Tab::Looks {
            self.open_looks();
        }
        if tab == Tab::Recipes {
            self.open_recipes();
        }
    }

    /// The person chose the tab that shows, as the system's tab bars do:
    /// back to its root page, or at the root, to the top of it.
    pub fn reselect_tab(self, tab: Tab) {
        if self.route_at(tab, 0).is_some() {
            self.pop_to(tab, 0);
        } else {
            let count = self.scroll_top.get().map_or(0, |(_, count)| count + 1);
            self.scroll_top.set(Some((tab, count)));
        }
    }

    /// The last request to scroll a root page to its top.
    pub fn scroll_top_request(self) -> Option<(Tab, u64)> {
        self.scroll_top.get()
    }

    pub fn route_at(self, tab: Tab, depth: usize) -> Option<Route> {
        self.stacks
            .get()
            .get(tab_index(tab))
            .and_then(|stack| stack.get(depth).cloned())
    }

    pub fn push(self, route: Route) {
        let tab = self.tab();
        self.stacks.update(|stacks| {
            let stack = &mut stacks[tab_index(tab)];
            if stack.len() < MAX_DEPTH {
                stack.push(route);
            }
        });
    }

    pub fn pop_to(self, tab: Tab, depth: usize) {
        self.stacks.update(|stacks| {
            stacks[tab_index(tab)].truncate(depth);
        });
    }

    pub fn sheet(self) -> Option<Sheet> {
        self.sheet.get()
    }

    pub fn set_sheet(self, sheet: Option<Sheet>) {
        self.sheet.set(sheet);
    }

    pub fn status(self) -> Option<CameraStatus> {
        self.status.get()
    }

    pub fn package(self) -> Option<PackageInfo> {
        self.package.get()
    }

    pub fn flow_phase(self) -> UpdatePhase {
        self.phase.get()
    }

    pub fn nearby_cameras(self) -> Vec<NearbyCamera> {
        #[cfg(demo)]
        if self.demo() {
            return self.simulated_nearby();
        }
        self.nearby.get()
    }

    /// The camera a saved connect reaches: the most recent one.
    pub fn remembered(self) -> Option<RememberedCamera> {
        self.saved.with(|cams| cams.first().cloned())
    }

    /// The saved cameras, the most recent first.
    pub fn saved_cameras(self) -> Vec<RememberedCamera> {
        self.saved.get()
    }

    /// Save `camera` as the most recent one.
    pub(crate) fn remember_camera(self, camera: RememberedCamera) {
        let mut cams = self.saved.get();
        crate::domain::upsert_saved(&mut cams, camera);
        self.store_saved(cams);
    }

    /// Remove a saved camera. The connected camera stays.
    pub fn forget_camera(self, peripheral_id: &str) {
        let connected = self.camera_online()
            && self
                .remembered()
                .is_some_and(|c| c.peripheral_id.eq_ignore_ascii_case(peripheral_id));
        if connected {
            return;
        }
        let mut cams = self.saved.get();
        crate::domain::remove_saved(&mut cams, peripheral_id);
        self.store_saved(cams);
    }

    fn store_saved(self, cams: Vec<RememberedCamera>) {
        if let Err(error) = self.api().save_cameras(&cams) {
            log::warn!("Could not save the cameras: {error}");
        }
        self.saved.set(cams);
    }

    pub fn link_state(self) -> LinkState {
        self.link_state.get()
    }

    /// The link went down without the user asking, and no connect followed.
    pub fn toasts(self) -> crate::ui::Toasts {
        self.toasts
    }

    /// Show a short notice; it outlives the page that asks for it.
    pub fn show_toast(self, message: String) {
        self.root.run(|| self.toasts.show(message));
    }

    /// Show a short notice with one action, such as Undo.
    pub fn show_toast_action(self, message: String, title: String, action: impl Fn() + 'static) {
        self.root
            .run(|| self.toasts.show_action(message, title, action));
    }

    /// The camera is connected and its link answers.
    pub fn camera_online(self) -> bool {
        self.status
            .get()
            .is_some_and(|s| s.camera.connection == ConnectionState::Connected)
    }

    pub fn model_name(self) -> String {
        self.status
            .get()
            .map(|s| s.camera.model.display_name().to_owned())
            .or_else(|| self.selected_model().map(|m| m.display_name().to_owned()))
            .or_else(|| self.api().simulated().map(|m| m.display_name().to_owned()))
            .unwrap_or_else(|| CameraModel::Gr3.display_name().to_owned())
    }

    pub fn selected_model(self) -> Option<CameraModel> {
        if let Some(id) = self.selected_id.get() {
            if let Some(cam) = self
                .nearby_cameras()
                .into_iter()
                .find(|c| c.peripheral_id == id)
            {
                return Some(cam.model);
            }
            if let Some(rem) = self.remembered()
                && rem.peripheral_id == id
            {
                return Some(rem.model);
            }
        }
        self.nearby_model()
    }

    pub fn nearby_model(self) -> Option<CameraModel> {
        if self.demo() {
            return self.api().simulated();
        }
        self.nearby_cameras()
            .first()
            .map(|c| c.model)
            .or_else(|| self.remembered().map(|r| r.model))
    }

    /// Watch for cameras nearby while the app is in the foreground: a
    /// background scan keeps the radio busy for a prompt nobody can see.
    fn start_ble_scan(self) {
        spawn(async move {
            let mut scanning = false;
            loop {
                let active = lifecycle() == nsh_core::platform::Lifecycle::Active;
                if active && !scanning {
                    self.resume_nearby_watch(false);
                    scanning = true;
                } else if !active && scanning {
                    platform::stop_scanning();
                    self.nearby_watch.with(|w| w.borrow_mut().clear());
                    self.nearby.set(Vec::new());
                    scanning = false;
                }
                if scanning {
                    let changed = self.nearby_watch.with(|w| w.borrow_mut().prune(now_ms()));
                    if changed {
                        self.publish_nearby();
                    }
                }
                timer(Duration::from_millis(PRUNE_INTERVAL_MS)).await;
            }
        });
    }

    /// Watch advertisements. Only `ask`, from a user action, may show the
    /// Bluetooth permission request.
    fn resume_nearby_watch(self, ask: bool) {
        let (tx, rx) = async_channel::unbounded();
        let remembered_id = self.remembered().map(|r| r.peripheral_id.clone());
        platform::start_scanning(tx, remembered_id.as_deref(), ask);
        spawn(async move {
            while let Ok(frame) = rx.recv().await {
                if frame.power == AdvertisedPower::Standby {
                    self.camera_powered_off(&frame.peripheral_id);
                }
                self.answer_peek(&frame.peripheral_id, frame.power);
                let changed = self
                    .nearby_watch
                    .with(|w| w.borrow_mut().frame(&frame, now_ms()));
                if changed {
                    self.publish_nearby();
                }
            }
        });
    }

    /// Show the sightings and offer the nearby prompt once per power-on.
    fn publish_nearby(self) {
        let list = self.nearby_watch.with(|w| w.borrow().sightings().to_vec());
        self.nearby.set(list);
        // Only an idle, disconnected app offers a camera.
        let connected = self
            .status()
            .is_some_and(|s| s.camera.connection == ConnectionState::Connected);
        if self.sheet().is_some() || connected || self.connect_in_flight() {
            return;
        }
        // The camera restarted for an update: the update joins it again
        // without a prompt.
        if self.update_restart_pending() {
            let back = self
                .remembered()
                .is_some_and(|r| self.advertised_powered_on(&r.peripheral_id));
            if back {
                self.reconnect_for_update();
            }
            return;
        }
        let remembered = self.remembered().map(|r| r.peripheral_id);
        let candidate = self
            .nearby_watch
            .with(|w| w.borrow().candidate(remembered.as_deref()));
        if let Some(camera) = candidate {
            self.suppress_nearby(&camera.peripheral_id);
            self.selected_id.set(Some(camera.peripheral_id));
            self.set_sheet(Some(Sheet::Nearby));
        }
    }

    /// Keep a camera out of the nearby prompt for its current power-on.
    pub(super) fn suppress_nearby(self, peripheral_id: &str) {
        self.nearby_watch
            .with(|w| w.borrow_mut().suppress(peripheral_id, now_ms()));
    }

    /// Whether a fresh advertisement says this camera is on.
    fn advertised_powered_on(self, peripheral_id: &str) -> bool {
        self.nearby_watch.with(|w| {
            w.borrow()
                .sightings()
                .iter()
                .any(|c| c.powered_on && c.peripheral_id.eq_ignore_ascii_case(peripheral_id))
        })
    }

    pub fn dismiss_nearby(self) {
        if let Some(id) = self.selected_id.get().or_else(|| {
            self.nearby_cameras()
                .first()
                .map(|c| c.peripheral_id.clone())
        }) {
            self.suppress_nearby(&id);
        }
        self.set_sheet(None);
    }

    pub fn open_scan(self) {
        if !self.demo() && dev_camera_url() {
            self.selected_id.set(Some("dev-url".into()));
            self.start_connect();
            return;
        }
        if !self.demo() && platform::bluetooth_unavailable() {
            self.set_sheet(Some(Sheet::ConnectError(LinkError::BluetoothUnavailable)));
            return;
        }
        // The first scan the user asks for may show the permission request.
        if !self.demo() {
            self.resume_nearby_watch(true);
        }
        self.scan_list_pinned.set(false);
        self.set_sheet(Some(Sheet::Scan));
    }

    /// Add Camera shows the camera list even when it has one camera.
    pub fn pin_scan_list(self) {
        self.scan_list_pinned.set(true);
    }

    pub fn scan_list_pinned(self) -> bool {
        self.scan_list_pinned.get()
    }

    /// Whether this camera is a saved one.
    pub fn is_saved(self, peripheral_id: &str) -> bool {
        self.saved.with(|cams| {
            cams.iter()
                .any(|c| c.peripheral_id.eq_ignore_ascii_case(peripheral_id))
        })
    }

    /// Whether the selected camera is a saved one.
    pub fn selected_is_saved(self) -> bool {
        self.selected_id.get().is_some_and(|id| self.is_saved(&id))
    }

    /// The connect under way reaches a saved camera (its first step wakes
    /// it) rather than pairing a new one.
    pub fn connecting_saved(self) -> bool {
        self.attempt_kind.get() == connect::AttemptKind::Saved
    }

    pub fn open_software_update(self) {
        self.push(Route::SoftwareUpdate);
    }

    /// Spawn a light poll loop: a simulated connect advances, and a scan
    /// notices that Bluetooth went away.
    pub fn ensure_ticker(self) {
        spawn(async move {
            loop {
                timer(Duration::from_millis(150)).await;
                if self.frozen() {
                    continue;
                }
                match self.sheet() {
                    #[cfg(demo)]
                    Some(Sheet::Connecting { .. }) if self.demo() => self.tick_connecting(),
                    Some(Sheet::Scan) if !self.demo() && platform::bluetooth_unavailable() => {
                        self.set_sheet(Some(Sheet::ConnectError(LinkError::BluetoothUnavailable)));
                    }
                    _ => {}
                }
            }
        });
    }
}

impl Tab {
    /// A stable name of the tab, as a page names its tab
    /// (`pages::page_scroll`'s `top_of`).
    pub fn key(self) -> &'static str {
        match self {
            Tab::Camera => "camera",
            Tab::Looks => "looks",
            Tab::Recipes => "recipes",
            Tab::Settings => "settings",
        }
    }
}

fn tab_index(tab: Tab) -> usize {
    match tab {
        Tab::Camera => 0,
        Tab::Looks => 1,
        Tab::Recipes => 2,
        Tab::Settings => 3,
    }
}

/// `GRKIT_CAMERA_URL` is set: development builds connect to that camera
/// service over HTTP alone, without Bluetooth or Wi-Fi.
fn dev_camera_url() -> bool {
    cfg!(demo) && crate::camera_api::has_dev_camera_url()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Connected and up to date? Only a release check that answered says so.
pub fn is_up_to_date(package: &PackageInfo) -> bool {
    package
        .available
        .as_ref()
        .is_some_and(|available| crate::domain::version_at_least(&package.installed, available))
}

/// The app's page in the platform's store.
pub fn app_store_url() -> String {
    if cfg!(target_os = "android") {
        "https://play.google.com/store/apps/details?id=studio.techflow.grkit".into()
    } else {
        format!("https://apps.apple.com/app/id{APP_STORE_ID}")
    }
}

/// Rate URL for the App Store review prompt.
pub fn rate_url() -> String {
    format!("https://apps.apple.com/app/id{APP_STORE_ID}?action=write-review")
}

/// User-facing copy for a connect-time link error.
pub fn link_error_title(error: &LinkError) -> String {
    crate::strings::tr(error.copy().0, &[])
}

pub fn link_error_body(error: &LinkError) -> String {
    use crate::strings::tr;
    match error {
        LinkError::Http(msg) => format!("{} {msg}", tr(error.copy().1, &[])),
        _ => tr(error.copy().1, &[]),
    }
}
