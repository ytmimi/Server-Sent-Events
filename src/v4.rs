use axum::routing::{get, post};
use axum::Router;
use tokio::sync::mpsc::{Receiver, Sender};

mod app_events;
pub mod database;
pub mod dynamodb;
mod kafka_consumer;
mod report_status;
mod request_handlers;
mod tasks;

pub(super) use app_events::AppEvent;

#[derive(Debug, Clone)]
struct V4AppState<D> {
    app_event_sender: Sender<AppEvent>,
    database: D,
}

pub fn create_app_v4<D>(
    sender: Sender<AppEvent>,
    receiver: Receiver<AppEvent>,
    database: D,
) -> Router
where
    D: database::Database + Clone + Send + Sync + 'static,
    <D as database::Database>::Error: std::fmt::Debug,
{
    tokio::spawn(tasks::handle_app_events(receiver, database.clone()));
    tokio::spawn(kafka_consumer::consume_kafka_messages(sender.clone()));

    let state = V4AppState {
        app_event_sender: sender,
        database,
    };

    Router::new()
        .route("/sse", get(request_handlers::sse_handler_v4))
        .route("/new/report", post(request_handlers::create_report))
        .route("/reports", get(request_handlers::list_reports))
        .with_state(state)
}
