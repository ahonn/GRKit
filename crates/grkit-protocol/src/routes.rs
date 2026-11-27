//! The paths of the service. Each path starts with `/v1/`.

pub const PING: &str = "/v1/ping";
/// The WebSocket that reports which resource changed.
pub const CHANGES: &str = "/v1/changes";
pub const PROPS: &str = "/v1/props";
pub const UPDATE: &str = "/v1/update";
pub const UPDATE_CANCEL: &str = "/v1/update/cancel";
pub const LOOKS: &str = "/v1/looks";
pub const LOOKS_ORDER: &str = "/v1/params/looks/order";
pub const RECIPES: &str = "/v1/recipes";
pub const RECIPES_ORDER: &str = "/v1/recipes/order";
pub const RECIPES_UNDO: &str = "/v1/recipes/undo";
pub const MODULES_CATALOG: &str = "/v1/modules/catalog";
pub const MODULES_PARAMS: &str = "/v1/params/modules";
pub const MODULES_ORDER: &str = "/v1/params/modules/order";
pub const MODULES_LAYOUT: &str = "/v1/params/modules/layout";
pub const SETTINGS: &str = "/v1/params/settings";
pub const SHUTDOWN_PICTURE: &str = "/v1/shutdown-picture";

/// The configuration of the Look `id`.
pub fn look(id: &str) -> String {
    format!("{LOOKS}/{id}")
}

/// The GRF file of the Look `id`.
pub fn look_content(id: &str) -> String {
    format!("{LOOKS}/{id}/content")
}

pub fn recipe(id: u32) -> String {
    format!("{RECIPES}/{id}")
}

pub fn recipe_apply(id: u32) -> String {
    format!("{RECIPES}/{id}/apply")
}

/// The icon of a module; `segment` is the module ID as a path segment.
pub fn module_icon(segment: &str) -> String {
    format!("/v1/modules/{segment}/icon")
}

/// The settings of a module; `segment` is the module ID as a path segment.
pub fn module_params(segment: &str) -> String {
    format!("{MODULES_PARAMS}/{segment}")
}
