use axum::Router;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::error::KafkaError;
use rdkafka::error::KafkaResult;
use rdkafka::message::Message;
use rdkafka::types::RDKafkaErrorCode;
use rdkafka::ClientConfig;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::v2::create_app_v2;
use crate::v2::Command;

/// The v3 router is the exact same as the v2 router except it adds a feature to listen
/// for messages on a Kafka topic.
pub fn create_app_v3(sender: Sender<Command>, receiver: Receiver<Command>) -> Router {
    tokio::spawn(listen_for_kafka_message(sender.clone()));
    create_app_v2(sender, receiver)
}

/// Continuously listen for messages on the `v3_messages` Kafka topic
async fn listen_for_kafka_message(sender: Sender<Command>) {
    let mut config = ClientConfig::new();
    config
        .set("group.id", "server_sent_events_v3")
        .set("bootstrap.servers", "localhost:9092")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false");

    let Ok(consumer): KafkaResult<StreamConsumer> = config.create() else {
        tracing::error!("Could not connect to kafka server");
        return;
    };

    let topics = ["v3_messages"];

    if let Err(err) = consumer.subscribe(&topics) {
        tracing::error!("Could not subscribe to kafka topics {topics:?}. {err:?}");
        return;
    }

    tracing::info!("listening for messages on kafka topics: {topics:?}");

    'consumer_loop: loop {
        match consumer.recv().await {
            Ok(message) => {
                let Some(bytes) = message.payload() else {
                    tracing::error!("couldn't get message payload from kafka topic");
                    continue 'consumer_loop;
                };

                let Some((username, message)) = parse_username_and_message_from_bytes(bytes) else {
                    tracing::error!(
                        "Cannot parse `username` and `message` from the kafka message: {:?}",
                        std::str::from_utf8(bytes)
                    );
                    continue 'consumer_loop;
                };

                let message = Command::Message { username, message };
                if let Err(err) = sender.send(message).await {
                    tracing::error!("couldn't send kafka message to {err:?}");
                }
            }
            Err(err) => {
                match err {
                    KafkaError::MessageConsumption(RDKafkaErrorCode::AllBrokersDown) => {
                        tracing::warn!(concat!(
                            "Please check that Kafka is running. All Brokers are down. ",
                            "The v3 app cannot accept messages from Kafka at this time."
                        ));
                        // To prevent endlessly looping in the demo app we simply stop this
                        // async task when we detect that the broker is unavailable.
                        return;
                    }
                    _ => {
                        tracing::warn!("Error {err:?}");
                    }
                }
            }
        }
    }
}

fn parse_username_and_message_from_bytes(bytes: &[u8]) -> Option<(String, String)> {
    // The message is expected to be in the following format `username:message`
    let colon_index = bytes.iter().position(|b| *b == b':')?;
    let username = String::from_utf8(bytes[..colon_index].to_vec()).ok()?;
    let message = String::from_utf8(bytes[colon_index + 1..].to_vec()).ok()?;
    Some((username, message))
}
