//! The routes of the service: each v1 path, the methods that it takes, the
//! body of each change, and the query that a change can have. The transport
//! checks a body against its route before it reads it (`transport::admit`);
//! the service admits each change once (`api::Service`) and gives the
//! handler of the route its typed route.

use crate::protocol::Problem;
use grkit_core::effects;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
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
    Looks(LookRoute),
    Recipes(RecipeRoute),
    Modules(ModuleRoute),
    Settings(SettingsRoute),
}

/// `/v1/looks/...` and `/v1/params/looks/order` (`resources::looks`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookRoute {
    Collection,
    Configuration(String),
    Content(String),
    Order,
}

/// `/v1/recipes/...` (`resources::recipes`). A recipe is found by its ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeRoute {
    Collection,
    Order,
    Undo,
    Recipe(u32),
    Apply(u32),
}

/// `/v1/modules/...` and `/v1/params/modules/...` (`resources::modules`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleRoute {
    Catalog,
    Icon(String),
    Params,
    Order,
    Layout,
    Module(String),
}

/// `/v1/params/settings` and `/v1/shutdown-picture`
/// (`resources::settings`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsRoute {
    Settings,
    ShutdownPicture,
}

/// The body of a change: its media type and its largest length, or the only
/// lengths that it can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub media: &'static str,
    pub limit: usize,
    pub lengths: Option<&'static [usize]>,
}

const JSON_SMALL: Body = Body::json(1024);
const JSON_LIST: Body = Body::json(4096);

impl Body {
    const fn json(limit: usize) -> Self {
        Body {
            media: "application/json",
            limit,
            lengths: None,
        }
    }

    /// Checks the declared length and media type of a body before the
    /// service reads it.
    pub fn check(&self, length: usize, media: Option<&str>) -> Result<(), Problem> {
        if let Some(lengths) = self.lengths {
            if !lengths.contains(&length) {
                return Err(Problem::new(
                    413,
                    "invalid_content_length",
                    "The body does not have a length that this resource takes.",
                ));
            }
        } else if length > self.limit {
            return Err(Problem::new(
                413,
                "body_too_large",
                "The body exceeds this resource's limit.",
            ));
        }
        let media = media.map(|value| value.split(';').next().unwrap_or("").trim());
        if media != Some(self.media) {
            return Err(Problem::new(
                415,
                "unsupported_media_type",
                "The request content type is not supported.",
            ));
        }
        Ok(())
    }
}

impl Route {
    /// The route of `path`, without its query. None for a path of no
    /// resource.
    pub fn parse(path: &str) -> Option<Self> {
        let parts: Vec<&str> = path.strip_prefix("/v1/")?.split('/').collect();
        let look = |id: &str| effects::valid_id(id).then(|| id.to_owned());
        let recipe = |id: &str| {
            (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
                .then(|| id.parse().ok())
                .flatten()
        };
        let module = |id: &str| (!id.is_empty()).then(|| id.to_owned());
        Some(match parts.as_slice() {
            ["ping"] => Self::Ping,
            ["changes"] => Self::Changes,
            ["props"] => Self::Props,
            ["debug", "diagnostics"] => Self::Diagnostics,
            ["update"] => Self::Update,
            ["update", "cancel"] => Self::UpdateCancel,
            ["looks"] => Self::Looks(LookRoute::Collection),
            ["looks", id] => Self::Looks(LookRoute::Configuration(look(id)?)),
            ["looks", id, "content"] => Self::Looks(LookRoute::Content(look(id)?)),
            ["params", "looks", "order"] => Self::Looks(LookRoute::Order),
            ["recipes"] => Self::Recipes(RecipeRoute::Collection),
            ["recipes", "order"] => Self::Recipes(RecipeRoute::Order),
            ["recipes", "undo"] => Self::Recipes(RecipeRoute::Undo),
            ["recipes", id] => Self::Recipes(RecipeRoute::Recipe(recipe(id)?)),
            ["recipes", id, "apply"] => Self::Recipes(RecipeRoute::Apply(recipe(id)?)),
            ["modules", "catalog"] => Self::Modules(ModuleRoute::Catalog),
            ["modules", id, "icon"] => Self::Modules(ModuleRoute::Icon(module(id)?)),
            ["params", "modules"] => Self::Modules(ModuleRoute::Params),
            ["params", "modules", "order"] => Self::Modules(ModuleRoute::Order),
            ["params", "modules", "layout"] => Self::Modules(ModuleRoute::Layout),
            ["params", "modules", id] => Self::Modules(ModuleRoute::Module(module(id)?)),
            ["params", "settings"] => Self::Settings(SettingsRoute::Settings),
            ["shutdown-picture"] => Self::Settings(SettingsRoute::ShutdownPicture),
            _ => return None,
        })
    }

    /// The methods that the route takes.
    pub fn methods(&self) -> &'static [&'static str] {
        use {LookRoute as L, ModuleRoute as M, RecipeRoute as R, SettingsRoute as S};
        match self {
            Self::Ping | Self::Changes | Self::Props | Self::Diagnostics => &["GET"],
            Self::Update => &["GET", "PUT"],
            Self::UpdateCancel => &["POST"],
            Self::Looks(L::Collection) => &["GET"],
            Self::Looks(L::Configuration(_)) => &["GET", "PATCH", "DELETE"],
            Self::Looks(L::Content(_) | L::Order) => &["GET", "PUT"],
            Self::Recipes(R::Collection) => &["GET", "POST"],
            Self::Recipes(R::Order) => &["PUT"],
            Self::Recipes(R::Undo | R::Apply(_)) => &["POST"],
            Self::Recipes(R::Recipe(_)) => &["PATCH", "DELETE"],
            Self::Modules(M::Catalog | M::Icon(_) | M::Params) => &["GET"],
            Self::Modules(M::Order | M::Layout) => &["PUT"],
            Self::Modules(M::Module(_)) => &["PATCH"],
            Self::Settings(S::Settings) => &["GET", "PATCH"],
            Self::Settings(S::ShutdownPicture) => &["GET", "PUT", "DELETE"],
        }
    }

    /// The body that the route takes with `method`, or None for a request
    /// without a body.
    pub fn body(&self, method: &str) -> Option<Body> {
        use {LookRoute as L, ModuleRoute as M, RecipeRoute as R, SettingsRoute as S};
        Some(match (self, method) {
            (Self::Update, "PUT") => Body {
                media: "application/zip",
                limit: crate::update::MAX_PACKAGE,
                lengths: None,
            },
            (Self::Looks(L::Content(_)), "PUT") => Body {
                media: "application/octet-stream",
                limit: effects::SIZE_WITH_LEVELS,
                lengths: Some(&[effects::SIZE, effects::SIZE_WITH_LEVELS]),
            },
            (Self::Looks(L::Configuration(_)), "PATCH") => JSON_SMALL,
            (Self::Looks(L::Order), "PUT") => JSON_LIST,
            (Self::Recipes(R::Collection), "POST") | (Self::Recipes(R::Order), "PUT") => JSON_LIST,
            (Self::Recipes(R::Recipe(_)), "PATCH") => JSON_SMALL,
            (Self::Modules(M::Order | M::Layout), "PUT") => JSON_SMALL,
            (Self::Modules(M::Module(_)), "PATCH") => JSON_LIST,
            (Self::Settings(S::Settings), "PATCH") => JSON_SMALL,
            (Self::Settings(S::ShutdownPicture), "PUT") => Body {
                media: "image/jpeg",
                limit: grkit_core::goodbye::MAX_BYTES,
                lengths: None,
            },
            _ => return None,
        })
    }

    /// The query that a change with `method` can have: a DELETE of a Look
    /// can leave it active until the next start.
    pub fn change_query(&self, method: &str) -> Option<&'static str> {
        matches!(
            (self, method),
            (Self::Looks(LookRoute::Configuration(_)), "DELETE")
        )
        .then_some("active=off")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_path_has_its_route_and_each_change_its_body() {
        assert_eq!(
            Route::parse("/v1/looks/alpha/content"),
            Some(Route::Looks(LookRoute::Content("alpha".into())))
        );
        assert_eq!(
            Route::parse("/v1/recipes/7/apply"),
            Some(Route::Recipes(RecipeRoute::Apply(7)))
        );
        for path in ["/v1/recipes/x", "/v1/looks/BAD ID", "/v2/ping", "/v1/other"] {
            assert_eq!(Route::parse(path), None, "{path}");
        }
        // The paths of the wire contract are routes of the service.
        use grkit_protocol::routes as paths;
        for path in [
            paths::PING.to_owned(),
            paths::CHANGES.to_owned(),
            paths::PROPS.to_owned(),
            paths::UPDATE.to_owned(),
            paths::UPDATE_CANCEL.to_owned(),
            paths::LOOKS.to_owned(),
            paths::look("alpha"),
            paths::look_content("alpha"),
            paths::LOOKS_ORDER.to_owned(),
            paths::RECIPES.to_owned(),
            paths::recipe(1),
            paths::recipe_apply(1),
            paths::RECIPES_ORDER.to_owned(),
            paths::RECIPES_UNDO.to_owned(),
            paths::MODULES_CATALOG.to_owned(),
            paths::module_icon("film"),
            paths::MODULES_PARAMS.to_owned(),
            paths::module_params("film"),
            paths::MODULES_ORDER.to_owned(),
            paths::MODULES_LAYOUT.to_owned(),
            paths::SETTINGS.to_owned(),
            paths::SHUTDOWN_PICTURE.to_owned(),
        ] {
            assert!(Route::parse(&path).is_some(), "{path}");
        }
        // Each method with a body is a method of its route.
        for path in [
            "/v1/update",
            "/v1/looks/alpha",
            "/v1/looks/alpha/content",
            "/v1/params/looks/order",
            "/v1/recipes",
            "/v1/recipes/1",
            "/v1/params/modules/layout",
            "/v1/params/settings",
            "/v1/shutdown-picture",
        ] {
            let route = Route::parse(path).unwrap();
            for method in ["PUT", "PATCH", "POST"] {
                if route.body(method).is_some() {
                    assert!(route.methods().contains(&method), "{method} {path}");
                }
            }
        }
        let content = Route::Looks(LookRoute::Content("alpha".into()))
            .body("PUT")
            .unwrap();
        assert!(content
            .check(effects::SIZE, Some("application/octet-stream"))
            .is_ok());
        assert_eq!(
            content
                .check(100, Some("application/octet-stream"))
                .unwrap_err()
                .code,
            "invalid_content_length"
        );
        assert_eq!(
            JSON_SMALL.check(10, Some("text/plain")).unwrap_err().code,
            "unsupported_media_type"
        );
    }
}
