use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use super::app_events::{AppEvent, ReportStatusUpdate, ServerSentEventMessage};
use super::report_status::{ReportStatus, ReportStatusError};

/// Async task Loop that process all the `Command` messages received on the Receiver
pub(super) async fn handle_app_events<D>(mut receiver: Receiver<AppEvent>, database: D)
where
    D: super::database::Database,
    <D as super::database::Database>::Error: std::fmt::Debug,
{
    let mut user_connection_map = HashMap::new();
    let mut report_status_cache = LruCache::new(NonZeroUsize::new(200).expect("value is > 0"));

    while let Some(event) = receiver.recv().await {
        match event {
            AppEvent::UserConnected { user_id, sender } => {
                tracing::info!("got a connection from user {user_id:?}");
                user_connection_map.insert(user_id, sender);
            }
            AppEvent::UserDisconnected { ref user_id } => {
                tracing::info!("user {user_id:?} closed the connection");
                let _ = user_connection_map.remove(user_id);
            }
            AppEvent::CacheReports(reports) => {
                for report in reports {
                    report_status_cache
                        .push(report.report_id, (report.user_id, report.report_status));
                }
            }
            AppEvent::UserMessage(event_message) => {
                match &event_message {
                    ServerSentEventMessage::ReportStatusUpdate(report) => {
                        // Grab the report_status / user_id from the database
                        match update_report_status(report, &mut report_status_cache, &database)
                            .await
                        {
                            Ok(user_id) => {
                                tracing::info!(
                                    "sending report_status_update message to {user_id:?} for report {}",
                                    report.id
                                );
                                if let Some(user_id) = user_id {
                                    if let Some(sender) = user_connection_map.get(&user_id) {
                                        let _ = sender.send(event_message).await.map_err(|err| {
                                            tracing::error!(
                                                "Failed to send message to {user_id:?}"
                                            );
                                            err
                                        });
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::error!("{err:?}");
                            }
                        }
                    }
                    ServerSentEventMessage::NewReport(new_report) => {
                        tracing::info!(
                            "Storing report {} in the cache for user {}",
                            new_report.report_id,
                            new_report.user_id
                        );
                        report_status_cache.push(
                            new_report.report_id,
                            (new_report.user_id, new_report.report_status),
                        );

                        tracing::info!(
                            "Storing report {} in the database for user {}",
                            new_report.report_id,
                            new_report.user_id
                        );
                        if let Err(err) = database.insert_report(new_report.clone()).await {
                            tracing::error!("could not store the report in the database {err:?}");
                            // TODO(ytmimi) probably should add the report to some dead letter
                            // queue so we can add it to the database later
                        }
                    }
                }
            }
        }
    }
}

async fn update_report_status<D>(
    report_status_update: &ReportStatusUpdate,
    report_status_cache: &mut LruCache<Uuid, (Uuid, ReportStatus)>,
    database: &D,
) -> Result<Option<Uuid>, ReportStatusError>
where
    D: super::database::Database,
    <D as super::database::Database>::Error: std::fmt::Debug,
{
    let Some(current_status) =
        get_current_report_status(report_status_update.id, report_status_cache, database).await
    else {
        return Err(ReportStatusError::ReportNotFound(
            report_status_update.id,
            report_status_update.status,
        ));
    };

    let new_status = current_status.transition(report_status_update.status)?;

    let user_id = database
        .update_report_status(report_status_update)
        .await
        .map_err(|err| {
            // log the error if we could not write to the DB
            tracing::error!("could not updated the database: {err:?}");
            ReportStatusError::DatabaseUpdateFailed
        })?;

    if let Some(user_id) = user_id {
        // update the status in the cache.
        report_status_cache.push(report_status_update.id, (user_id, new_status));
    }

    Ok(user_id)
}

async fn get_current_report_status<D>(
    report_id: Uuid,
    report_status_cache: &mut LruCache<Uuid, (Uuid, ReportStatus)>,
    database: &D,
) -> Option<ReportStatus>
where
    D: super::database::Database,
    <D as super::database::Database>::Error: std::fmt::Debug,
{
    // try to lookup the status in the cache
    if let Some((_, current_report_status)) = report_status_cache.get(&report_id) {
        return Some(*current_report_status);
    }

    // otherwise we need to lookup the report in the database
    database.get_report_status(report_id).await.ok()?
}

/// Helper task that notifies the main async loop that a user has disconnected
pub(super) async fn handle_user_disconnect(
    app_command_sender: Sender<AppEvent>,
    user_sse_sender: Sender<ServerSentEventMessage>,
    user_id: Uuid,
) {
    // `closed()` will wait for the receiving end of the stream to be dropped.
    // The receiving end is dropped when the user disconnects from the server
    user_sse_sender.closed().await;

    let closed = AppEvent::UserDisconnected { user_id };

    if let Err(err) = app_command_sender.send(closed).await {
        tracing::error!("{err:?}");
        tracing::warn!("Can't issue closed event for {user_id:?}");
    }
}
