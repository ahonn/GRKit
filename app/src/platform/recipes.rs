//! Camera recipes over HTTP (`/v1/recipes`). Each change applies at once and
//! answers with the list as it is after the change. A change names the
//! camera start that the phone read the recipes in (`X-GRKit-Boot-Id`).

use std::time::Duration;

use nsh_core::element::HttpResponse;

use super::camera_http_base;
use crate::domain::{
    LookError,
    recipes::{RecipeError, RecipesSnapshot, parse_recipes, recipe_error},
};

const TIMEOUT: Duration = Duration::from_secs(15);

async fn request(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
    boot_id: Option<&str>,
) -> Result<HttpResponse, RecipeError> {
    let mut headers = Vec::new();
    if let Some(boot_id) = boot_id {
        headers.push(("X-GRKit-Boot-Id", boot_id.to_owned()));
    }
    let body = match body {
        Some(value) => {
            headers.push(("Content-Type", "application/json".to_owned()));
            serde_json::to_vec(&value).expect("JSON encodes")
        }
        None => Vec::new(),
    };
    super::camera_request(
        method,
        format!("{}{path}", camera_http_base()),
        body,
        TIMEOUT,
        &headers,
    )
    .await
    .map_err(|_| RecipeError::Look(LookError::ConnectionLost))
}

fn snapshot(response: HttpResponse) -> Result<RecipesSnapshot, RecipeError> {
    if !response.ok() {
        let body = response.text();
        log::warn!(
            "Recipes: HTTP {}: {}",
            response.status(),
            body.chars().take(300).collect::<String>()
        );
        return Err(recipe_error(response.status(), &body));
    }
    parse_recipes(&response.body).map_err(|e| RecipeError::Look(LookError::Service(e)))
}

/// The recipes of the camera.
pub async fn list() -> Result<RecipesSnapshot, RecipeError> {
    snapshot(request("GET", "/v1/recipes", None, None).await?)
}

/// One change; the answer is the list after it.
pub async fn change(
    boot_id: &str,
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<RecipesSnapshot, RecipeError> {
    snapshot(request(method, &format!("/v1/recipes{path}"), body, Some(boot_id)).await?)
}
