use axum::Router;
use serde::Deserialize;
use tokio::sync::mpsc::{channel, Receiver, Sender};

mod v1;
mod v2;

pub fn create_app() -> Router {
    // Often you'll see sender and reciver named `tx` and `rx` in code that uses channels,
    // But here's we're using sender and reciver to avoid any confusion.
    let (sender, reciver): (Sender<v2::Command>, Receiver<v2::Command>) = channel(100);

    Router::new()
        .nest("/v1", v1::create_app_v1())
        .nest("/v2", v2::create_app_v2(sender, reciver))
}

#[derive(Debug, Deserialize)]
pub(crate) struct QueryParams {
    username: String,
}
