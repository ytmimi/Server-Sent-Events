use std::ops::Deref;

use async_trait::async_trait;
use uuid::Uuid;

use super::{
    app_events::{Report, ReportStatusUpdate},
    report_status::ReportStatus,
};

#[async_trait]
pub trait Database {
    type Error;
    async fn list_reports(&self, use_id: Uuid) -> Result<Vec<Report>, Self::Error>;
    async fn insert_report(&self, report: Report) -> Result<(), Self::Error>;
    async fn update_report_status(
        &self,
        update: &ReportStatusUpdate,
    ) -> Result<Option<Uuid>, Self::Error>;
    async fn get_report_status(&self, report_id: Uuid)
        -> Result<Option<ReportStatus>, Self::Error>;
}

#[async_trait]
impl<T> Database for std::sync::Arc<T>
where
    T: Database + Sync + Send,
{
    type Error = T::Error;

    async fn list_reports(&self, user_id: Uuid) -> Result<Vec<Report>, Self::Error> {
        self.deref().list_reports(user_id).await
    }

    async fn insert_report(&self, report: Report) -> Result<(), Self::Error> {
        self.deref().insert_report(report).await
    }

    async fn update_report_status(
        &self,
        update: &ReportStatusUpdate,
    ) -> Result<Option<Uuid>, Self::Error> {
        self.deref().update_report_status(update).await
    }

    async fn get_report_status(
        &self,
        report_id: Uuid,
    ) -> Result<Option<ReportStatus>, Self::Error> {
        self.deref().get_report_status(report_id).await
    }
}
