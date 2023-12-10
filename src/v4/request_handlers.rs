use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::fmt::Debug;
use std::{convert::Infallible, time::Duration};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_stream::StreamExt as _;
use uuid::Uuid;

use super::app_events::{AppEvent, Report, ReportStatusUpdate, ServerSentEventMessage};
use super::tasks::handle_user_disconnect;
use super::V4AppState;

#[derive(Debug, serde::Deserialize)]
pub(super) struct QueryParams {
    user_id: Uuid,
}

/// Handles [Server Sent Events]
///
/// Once the connection has been established the server can keep sending data to the client.
///
/// [Server Sent Events]: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
pub(super) async fn sse_handler_v4<D>(
    State(state): State<V4AppState<D>>,
    Query(params): Query<QueryParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, String> {
    let (sse_sender, sse_receiver): (
        Sender<ServerSentEventMessage>,
        Receiver<ServerSentEventMessage>,
    ) = channel(100);

    let connect = AppEvent::UserConnected {
        user_id: params.user_id,
        sender: sse_sender.clone(),
    };

    let Ok(()) = state.app_event_sender.send(connect).await else {
        return Err(format!("Error Connecting {}", params.user_id));
    };

    // Register a task that will help clean up the connection when the stream is closed
    // Inspired by https://github.com/tokio-rs/axum/discussions/1060#discussioncomment-7457290
    tokio::spawn(handle_user_disconnect(
        state.app_event_sender,
        sse_sender,
        params.user_id,
    ));

    // Create the event stream from the receiving end of the channel
    let stream =
        tokio_stream::wrappers::ReceiverStream::new(sse_receiver).filter_map(|data| match data {
            ServerSentEventMessage::ReportStatusUpdate(report_status) => {
                let data = serde_json::to_string(&report_status).ok()?;
                let mut event = Event::default();
                event = event.event("report_status_update").data(data);
                Some(Ok(event))
            }
            ServerSentEventMessage::NewReport { .. } => {
                // We shouldn't get these kinds of messages, but even if we do we don't want to
                // send these messages to the user since they know they just created a report.
                None
            }
        });

    // Create and return the server sent event response
    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive-text"),
    );
    Ok(sse)
}

pub(super) async fn create_report<D>(
    State(state): State<V4AppState<D>>,
    Query(params): Query<QueryParams>,
) -> (StatusCode, Result<Json<Report>, String>) {
    let new_report = Report::new(params.user_id);
    let new_report_message = AppEvent::new_report(new_report.clone());
    match state.app_event_sender.send(new_report_message).await {
        Err(err) => {
            tracing::error!(
                "unable to send `new_report_message` for user {}. {err:?}",
                params.user_id
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Err("Unable to create a new report".to_owned()),
            )
        }
        Ok(()) => {
            tracing::info!("created new report {new_report:?}");
            (StatusCode::CREATED, Ok(Json(new_report)))
        }
    }
}

pub(super) async fn list_reports<D>(
    State(state): State<V4AppState<D>>,
    Query(params): Query<QueryParams>,
) -> (StatusCode, Result<Json<Vec<Report>>, String>)
where
    D: super::database::Database + Clone + Sync + Send,
    D::Error: Debug,
{
    let reports = match state.database.list_reports(params.user_id).await {
        Err(err) => {
            tracing::warn!(
                "Unable to fetch reports for user {}. {err:?}",
                params.user_id
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Err("unable to fetch reports".to_string()),
            );
        }
        Ok(reports) => reports,
    };

    if reports.is_empty() {
        // exit early. Send the user an empty response
        return (StatusCode::OK, Ok(Json(reports)));
    }

    let message = AppEvent::cache_reports(reports.clone());
    if let Err(err) = state.app_event_sender.send(message).await {
        tracing::warn!(
            "unable to cache reports for user {}. {err:?}",
            params.user_id
        );
    }

    (StatusCode::OK, Ok(Json(reports)))
}

pub(super) async fn change_report_status<D>(
    State(state): State<V4AppState<D>>,
    Query(params): Query<QueryParams>,
    Json(update): Json<ReportStatusUpdate>,
) -> StatusCode {
    if let Err(err) = state.report_status_sender.send(update).await {
        tracing::error!(
            "Unable to to send report status update message for user {}. {err:?}",
            params.user_id
        );
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        tracing::info!(
            "Successfully sent report status update message for user {}",
            params.user_id
        );
        StatusCode::ACCEPTED
    }
}
