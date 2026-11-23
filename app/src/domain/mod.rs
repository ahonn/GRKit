//! Camera and update flow models used by the UI and by demo mode.
//!
//! Kept free of NSH so unit tests run without an engine.

mod advertisement;
pub mod auto_connect;
mod camera;
pub mod changes;
mod connect_flow;
pub mod connect_policy;
mod event_filter;
mod fake;
pub mod fake_looks;
pub(crate) mod http_api;
mod link;
pub mod link_health;
pub mod look_media;
pub mod looks;
pub mod nearby_watch;
pub mod recipes;
pub mod release_notes;
mod remembered;
pub mod saved_connect;
mod update;
mod update_api;
pub use update_api::{
    JournalState, SoftwareRequestSummary, UpdateIntent, UpdateObservation, UpdateResource,
    UpdateSnapshot, observe_update, problem_code, response_error,
};
pub mod uuids;
mod wifi_cred;

pub use advertisement::{Advertisement, is_supported_camera, model_from_code, parse_advertisement};
pub use camera::{Camera, CameraModel, ConnectionState, PackageInfo};
pub use connect_flow::{
    BleSetup, ConnectOptions, ConnectReport, ConnectStep, ConnectTransport, Join,
    LocalNetworkAccess, Probe, WifiCredentials, run_connect, timing,
};
pub use event_filter::{ApReadyLatch, is_ble_pairing_error, matches_operation};
pub use fake::{DemoFail, FakeCamera, FakeConfig};
pub use http_api::{
    CAMERA_HTTP_BASE, PackageCamera, PackageMapError, PackageResponse, PropsResponse,
    STATUS_POLL_BUDGET_MS, StatusAttempt, classify_props_response, in_ble_startup,
    installed_to_models, package_to_models, parse_package, parse_props, should_retry_status,
    status_backoff_ms, status_poll_timeout_error, version_at_least,
};
pub use link::{
    CameraLink, CameraStatus, ErrorAction, LinkError, NearbyCamera, SendProgress, UpdatePoll,
    block_on_ready,
};
pub use looks::{
    Catalog, CatalogLook, CoverGroup, Look, LookError, LookFilter, LookGroup, LookSource,
    LooksSnapshot, PendingChange, group_of, look_error, parse_catalog, parse_looks, valid_name,
};
pub use remembered::{
    RememberedCamera, SAVED_MAX, decode as decode_saved, encode as encode_saved,
    remove as remove_saved, upsert as upsert_saved,
};
pub use update::{
    CARD_SPARE_BYTES, CONFIRM_DEADLINE, ReleaseProblem, RestartStep, SERVICE_PROTOCOL, UpdateError,
    UpdateFlow, UpdateKind, UpdatePhase, cancel_body_key, card_shortfall, firmware_problem,
    needs_newer_app, update_step,
};
pub use wifi_cred::decode_cred;
