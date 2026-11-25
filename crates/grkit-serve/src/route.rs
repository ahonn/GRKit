//! The resource of each v1 path. The service routes a request here once;
//! the handler of a resource reads the rest of its path.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    /// `/v1/ping`, which needs no card.
    Ping,
    /// `/v1/changes`, the one connection that stays open (`changes`).
    Changes,
    Props,
    Diagnostics,
    /// `/v1/update`.
    Update,
    /// `/v1/update/cancel`.
    UpdateCancel,
    /// `/v1/looks/...` and `/v1/params/looks/...` (`look_api`).
    Looks,
    /// `/v1/recipes` and `/v1/recipes/...` (`look_api::recipe_api`).
    Recipes,
    /// `/v1/modules/...` and `/v1/params/modules/...` (`look_api::module_api`).
    Modules,
    /// `/v1/params/settings` and `/v1/shutdown-picture`
    /// (`look_api::settings_api`).
    Settings,
}

impl Resource {
    /// The resource of `path`, without its query. None for a path of no
    /// resource.
    pub fn of(path: &str) -> Option<Self> {
        let parts: Vec<&str> = path.strip_prefix("/v1/")?.split('/').collect();
        Some(match parts.as_slice() {
            ["ping"] => Self::Ping,
            ["changes"] => Self::Changes,
            ["props"] => Self::Props,
            ["debug", "diagnostics"] => Self::Diagnostics,
            ["update"] => Self::Update,
            ["update", "cancel"] => Self::UpdateCancel,
            ["looks", ..] | ["params", "looks", ..] => Self::Looks,
            ["recipes", ..] => Self::Recipes,
            ["modules", ..] | ["params", "modules", ..] => Self::Modules,
            ["params", "settings"] | ["shutdown-picture"] => Self::Settings,
            _ => return None,
        })
    }
}
