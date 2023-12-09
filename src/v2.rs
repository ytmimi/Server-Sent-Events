use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::Stream;
use std::collections::HashMap;
use std::{convert::Infallible, time::Duration};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_stream::StreamExt as _;

use crate::QueryParams;

pub fn create_app_v2(sender: Sender<Command>, receiver: Receiver<Command>) -> Router {
    tokio::spawn(handle_command_messages(receiver));

    let state = V2AppState {
        command_sender: sender,
    };

    Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(send_message))
        .with_state(state)
}

#[derive(Clone)]
struct V2AppState {
    command_sender: Sender<Command>,
}

#[derive(Debug)]
pub(crate) enum Command {
    Connect {
        username: String,
        sender: Sender<String>,
    },
    Message {
        username: String,
        message: String,
    },
    Closed {
        username: String,
    },
}

/// Async task Loop that process all the `Command` messages received on the Receiver
async fn handle_command_messages(mut receiver: Receiver<Command>) {
    let mut map = HashMap::new();

    while let Some(command) = receiver.recv().await {
        match command {
            Command::Connect { username, sender } => {
                tracing::info!("got a connection from {username:?}");
                map.insert(username, sender);
            }
            Command::Closed { ref username } => {
                tracing::info!("{username:?} closed the connection");
                let _ = map.remove(username);
            }
            Command::Message { username, message } => {
                tracing::info!("sending message to {username:?}");
                match map.get(&username) {
                    Some(tx) => {
                        let _ = tx.send(message).await.map_err(|err| {
                            tracing::error!("Failed to send message to {username:?}");
                            err
                        });
                    }
                    None => {
                        tracing::warn!("Not connected to {username:?}. Cannot send message");
                    }
                }
            }
        }
    }
}

/// Helper task that notifies the main async loop that a user has disconnected
async fn handle_user_disconnect(
    app_command_sender: Sender<Command>,
    user_sse_sender: Sender<String>,
    username: String,
) {
    // `closed()` will wait for the receiving end of the stream to be dropped.
    // The receiving end is dropped when the user disconnects from the server
    user_sse_sender.closed().await;

    let closed = Command::Closed {
        username: username.clone(),
    };

    if let Err(err) = app_command_sender.send(closed).await {
        tracing::error!("{err:?}");
        tracing::warn!("Can't issue closed event for {username:?}");
    }
}

/// Handles [Server Sent Events]
///
/// Once the connection has been established the server can keep sending data to the client.
///
/// [Server Sent Events]: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
#[axum::debug_handler]
async fn sse_handler(
    State(state): State<V2AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, String> {
    let (sse_sender, sse_receiver): (Sender<String>, Receiver<String>) = channel(100);

    let connect = Command::Connect {
        username: params.username.clone(),
        sender: sse_sender.clone(),
    };
    let Ok(()) = state.command_sender.send(connect).await else {
        return Err("Error! Connecting".into());
    };

    // Register a task that will help clean up the connection when the stream is closed
    // Inspired by https://github.com/tokio-rs/axum/discussions/1060#discussioncomment-7457290
    tokio::spawn(handle_user_disconnect(
        state.command_sender,
        sse_sender,
        params.username,
    ));

    // Create the event stream from the receiving end of the channel
    let stream = tokio_stream::wrappers::ReceiverStream::new(sse_receiver)
        .map(|data| Ok(Event::default().data(data)));

    // Create and return the server sent event response
    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive-text"),
    );
    Ok(sse)
}

#[axum::debug_handler]
async fn send_message(
    State(state): State<V2AppState>,
    Query(params): Query<QueryParams>,
    body: String,
) -> StatusCode {
    let message = Command::Message {
        username: params.username.clone(),
        message: body,
    };
    match state.command_sender.send(message).await {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(err) => {
            tracing::error!("{err:?}");
            tracing::error!("failed to send message event to {:?}", params.username);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
