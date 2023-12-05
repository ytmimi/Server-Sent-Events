use crate::QueryParams;
use axum::extract::Query;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::{debug_handler, Router};
use futures::stream::{repeat_with, Stream};
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;

pub fn create_app_v1() -> Router {
    Router::new().route("/sse", get(sse_handler))
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
