use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::Consumer;
use rdkafka::error::KafkaError;
use rdkafka::error::KafkaResult;
use rdkafka::message::Message;
use rdkafka::types::RDKafkaErrorCode;
use rdkafka::ClientConfig;
use tokio::sync::mpsc::Sender;

use crate::v4::app_events::{AppEvent, ReportStatusUpdate};

pub(super) const KAFKA_TOPIC: &str = "v4_messages";

/// Continuously listen for messages on the `v4_messages` Kafka topic
pub(super) async fn consume_kafka_messages(sender: Sender<AppEvent>) {
    let mut config = ClientConfig::new();
    config
        .set("group.id", "server_sent_events_v4")
        .set("bootstrap.servers", "localhost:9092")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false");

    let Ok(consumer): KafkaResult<StreamConsumer> = config.create() else {
        tracing::error!("Could not connect to kafka server");
        return;
    };

    let topics = [KAFKA_TOPIC];

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

                let Ok(report_status) = serde_json::from_slice::<ReportStatusUpdate>(bytes) else {
                    tracing::error!(
                        "Cannot parse `report_status_update` message from the kafka message: {:?}",
                        std::str::from_utf8(bytes)
                    );
                    continue 'consumer_loop;
                };

                let message = AppEvent::report_status_update_message(report_status);
                if let Err(err) = sender.send(message).await {
                    tracing::error!("couldn't send kafka message to {err:?}");
                }
            }
            Err(err) => {
                match err {
                    KafkaError::MessageConsumption(RDKafkaErrorCode::AllBrokersDown) => {
                        tracing::warn!(concat!(
                            "Please check that Kafka is running. All Brokers are down. ",
                            "The v4 app cannot accept messages from Kafka at this time."
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
