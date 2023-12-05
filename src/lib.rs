use axum::Router;
use serde::Deserialize;

mod v1;

pub fn create_app() -> Router {
    Router::new().nest("/v1", v1::create_app_v1())
}

#[derive(Debug, Deserialize)]
pub(crate) struct QueryParams {
    username: String,
}
