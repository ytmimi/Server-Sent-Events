//! Based on the Server-Sent-Event example in the axum crate:
//! <https://github.com/tokio-rs/axum/blob/main/examples/sse/src/main.rs>
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use server_sent_events::{create_app, get_dynamo_db_client};

#[tokio::main]
async fn main() {
    // logging configuration from the `SSE_LOG` environemnt variable
    let env_filter = EnvFilter::try_from_env("SSE_LOG").unwrap_or_else(|_| {
        "server_sent_events=debug,tower_http=debug,rdkafka=debug,aws-sdk-dynamodb=trace".into()
    });

    // setup logging
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let dynamodb_client = std::sync::Arc::new(get_dynamo_db_client().await);

    // build our application and add a middleware layer to enable tracing (logging)
    let app = create_app(dynamodb_client).layer(TraceLayer::new_for_http());

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
