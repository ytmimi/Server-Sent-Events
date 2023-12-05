//! Based on the Server-Sent-Event example in the axum crate:
//! <https://github.com/tokio-rs/axum/blob/main/examples/sse/src/main.rs>

use axum::extract::Query;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::{debug_handler, Router};
use futures::stream::{repeat_with, Stream};
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    // logging configuration from the `SSE_LOG` environemnt variable
    let env_filter = EnvFilter::try_from_env("SSE_LOG")
        .unwrap_or_else(|_| "server_sent_events=debug,tower_http=debug".into());

    // setup logging
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .layer(TraceLayer::new_for_http());

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    username: String,
}

#[debug_handler]
async fn sse_handler(
    Query(params): Query<QueryParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("`{:?}` connected", params.username);

    // A `Stream` that repeats an event every 15 second
    let stream = repeat_with(|| Ok(Event::default().data("hi!"))).throttle(Duration::from_secs(15));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(3))
            .text("keep-alive-text"),
    )
}
