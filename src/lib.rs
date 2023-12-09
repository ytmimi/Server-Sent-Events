use axum::Router;
use serde::Deserialize;
use tokio::sync::mpsc::{channel, Receiver, Sender};

mod v1;
mod v2;
mod v3;
mod v4;

pub use v4::dynamodb::get_dynamo_db_client;

pub fn create_app<D>(database: D) -> Router
where
    D: v4::database::Database + Clone + Sync + Send + 'static,
    <D as v4::database::Database>::Error: std::fmt::Debug,
{
    // Often you'll see sender and reciver named `tx` and `rx` in code that uses channels,
    // But here's we're using sender and reciver to avoid any confusion.
    let (sender_v2, receiver_v2): (Sender<v2::Command>, Receiver<v2::Command>) = channel(100);
    let (sender_v3, receiver_v3): (Sender<v2::Command>, Receiver<v2::Command>) = channel(100);
    let (sender_v4, receiver_v4): (Sender<v4::AppEvent>, Receiver<v4::AppEvent>) = channel(100);

    Router::new()
        .nest("/v1", v1::create_app_v1())
        .nest("/v2", v2::create_app_v2(sender_v2, receiver_v2))
        .nest("/v3", v3::create_app_v3(sender_v3, receiver_v3))
        .nest("/v4", v4::create_app_v4(sender_v4, receiver_v4, database))
}

#[derive(Debug, Deserialize)]
pub(crate) struct QueryParams {
    username: String,
}
