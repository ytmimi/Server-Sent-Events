use std::str::FromStr;

use async_trait::async_trait;
use aws_config::{self, BehaviorVersion, SdkConfig};
use aws_sdk_dynamodb::operation::query::QueryOutput;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue, Select};
use aws_sdk_dynamodb::Client as DynamoDB;
use uuid::Uuid;

use super::app_events::{Report, ReportStatusUpdate};
use super::database::Database;
use super::report_status::ReportStatus;

async fn get_aws_config() -> SdkConfig {
    // The `BehaviorVersion` is a mechanism that can help maintain stability in user code,
    // while allowing the aws-sdk library to evolve it's defaults
    aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await
}

pub async fn get_dynamo_db_client() -> DynamoDB {
    let aws_skd_config = get_aws_config().await;
    let mut dynamo_db_config_builder = aws_sdk_dynamodb::config::Builder::from(&aws_skd_config);
    dynamo_db_config_builder.set_endpoint_url(Some("http://localhost:8111".to_owned()));
    DynamoDB::from_conf(dynamo_db_config_builder.build())
}

const TABLE_NAME: &str = "report_status";

#[async_trait]
impl Database for DynamoDB {
    type Error = aws_sdk_dynamodb::Error;

    async fn insert_report(&self, report: Report) -> Result<(), Self::Error> {
        tracing::info!(
            "storing a new report {} in DynamoDB for user {}",
            report.report_id,
            report.user_id
        );
        let request = self
            .put_item()
            .table_name(TABLE_NAME.to_owned())
            .item("report_id", AttributeValue::S(report.report_id.to_string()))
            .item("user_id", AttributeValue::S(report.user_id.to_string()))
            .item(
                "status",
                AttributeValue::S(report.report_status.as_str().to_owned()),
            );

        let _ = request.send().await?;

        Ok(())
    }

    async fn list_reports(&self, user_id: uuid::Uuid) -> Result<Vec<Report>, Self::Error> {
        tracing::info!("listing reports for user {}", user_id);

        let request = self
            .query()
            .table_name(TABLE_NAME)
            .index_name("UserIdIndex")
            .select(Select::SpecificAttributes)
            .key_condition_expression("#user_id = :user_id")
            .expression_attribute_names("#user_id", "user_id")
            .expression_attribute_names("#s", "status")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .projection_expression("report_id, #s");

        let mut paginator = request.into_paginator().page_size(100).send();

        let mut output = vec![];
        while let Some(value) = paginator.next().await {
            let query_output = value?;

            if query_output.items().is_empty() {
                break;
            }

            if let Ok(partial_reports) = PartialReports::try_from(query_output) {
                output.extend(partial_reports.into_reports(user_id))
            }
        }
        Ok(output)
    }

    async fn update_report_status(
        &self,
        update: &ReportStatusUpdate,
    ) -> Result<Option<Uuid>, Self::Error> {
        tracing::info!(
            "Changing the status of report {:?} in DynamoDB to {:?}",
            update.id,
            update.status,
        );

        let request = self
            .update_item()
            .table_name(TABLE_NAME.to_owned())
            .key("report_id", AttributeValue::S(update.id.to_string()))
            .expression_attribute_values(
                ":report_status",
                AttributeValue::S(update.status.as_str().to_owned()),
            )
            .expression_attribute_names("#s", "status")
            .update_expression("set #s = :report_status")
            .return_values(ReturnValue::AllOld);

        let response = request.send().await?;
        let value = response
            .attributes
            .expect("we asked for values to be returned");

        // TODO(ytmimi) we should have better error handling here
        // instead of calling .expect
        let user_id = value
            .get("user_id")
            .expect("table has a user_id column")
            .as_s()
            .expect("username stored as a string");

        Ok(Some(
            Uuid::from_str(user_id).expect("user_id is a valid uuid"),
        ))
    }

    async fn get_report_status(
        &self,
        report_id: Uuid,
    ) -> Result<Option<ReportStatus>, Self::Error> {
        let request = self
            .get_item()
            .table_name(TABLE_NAME.to_owned())
            .key("report_id", AttributeValue::S(report_id.to_string()))
            .expression_attribute_names("#s", "status")
            .projection_expression("report_id, #s");

        let response = request.send().await?;

        let Some(item) = response.item() else {
            return Ok(None);
        };

        let Some(Ok(status)) = item.get("status").map(|attribute| attribute.as_s()) else {
            return Ok(None);
        };

        Ok(ReportStatus::from_str(status).ok())
    }
}

struct PartialReports(Vec<ReportStatusUpdate>);

impl PartialReports {
    fn into_reports(self, user_id: uuid::Uuid) -> Vec<Report> {
        self.0
            .into_iter()
            .map(|report_status| report_status.into_report(user_id))
            .collect()
    }
}

impl TryFrom<QueryOutput> for PartialReports {
    type Error = String;
    fn try_from(value: QueryOutput) -> Result<Self, Self::Error> {
        let reports = value.items.unwrap_or_default();
        let reports = reports
            .into_iter()
            .filter_map(|output| {
                let report_id = output.get("report_id")?.as_s().ok()?;
                let status = output.get("status")?.as_s().ok()?;
                Some(ReportStatusUpdate {
                    id: uuid::Uuid::from_str(report_id).ok()?,
                    status: ReportStatus::from_str(status).ok()?,
                })
            })
            .collect::<Vec<_>>();

        Ok(PartialReports(reports))
    }
}
