use axum::routing::{get, post, put};
use axum::Router;
use tokio::sync::mpsc::{channel, Receiver, Sender};

mod app_events;
pub mod database;
pub mod dynamodb;
mod kafka_consumer;
mod kafka_producer;
mod report_status;
mod request_handlers;
mod tasks;

pub(super) use app_events::AppEvent;

#[derive(Debug, Clone)]
struct V4AppState<D> {
    app_event_sender: Sender<AppEvent>,
    report_status_sender: Sender<app_events::ReportStatusUpdate>,
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

    let (report_status_sender, report_status_receiver) = channel(100);

    tokio::spawn(kafka_producer::produce_kafka_messages(
        report_status_receiver,
    ));

    let state = V4AppState {
        app_event_sender: sender,
        report_status_sender,
        database,
    };

    Router::new()
        .route("/sse", get(request_handlers::sse_handler_v4))
        .route("/new/report", post(request_handlers::create_report))
        .route("/reports", get(request_handlers::list_reports))
        .route("/report", put(request_handlers::change_report_status))
        .with_state(state)
}
