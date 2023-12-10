use std::time::Duration;

use rdkafka::error::KafkaResult;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use tokio::sync::mpsc::Receiver;

use super::app_events::ReportStatusUpdate;
use super::kafka_consumer::KAFKA_TOPIC;

pub(super) async fn produce_kafka_messages(mut reciever: Receiver<ReportStatusUpdate>) {
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .set("enable.auto.commit", "false");

    // todo setup Kafka producer
    let Ok(producer): KafkaResult<FutureProducer> = config.create() else {
        tracing::error!("unable to create kafka producer");
        return;
    };

    'producer_loop: loop {
        match reciever.recv().await {
            None => break 'producer_loop,
            Some(report_status_update) => {
                let Ok(payload) = serde_json::to_string(&report_status_update) else {
                    continue 'producer_loop;
                };

                tracing::info!("producer_loop received message: {payload}");

                let message = FutureRecord::to(KAFKA_TOPIC)
                    .payload(&payload)
                    // This was a helpful article explaining the importance of Keys:
                    // https://forum.confluent.io/t/what-should-i-use-as-the-key-for-my-kafka-message/312
                    .key(report_status_update.id.as_bytes());
                match producer.send(message, Duration::from_secs(0)).await {
                    Ok((partition, offset)) => {
                        tracing::debug!(
                            "message sent to partition {partition} with offset {offset}"
                        )
                    }
                    Err((kafka_err, _message)) => {
                        tracing::error!("could not produce kafka message: {kafka_err:?}")
                        // FIXME(ytmimi) we shouldn't just drop the message here. We should try
                        // to stick the message in a queue so that we can retry it later.
                    }
                }
            }
        }
    }
}
