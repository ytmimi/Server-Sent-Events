use super::report_status::ReportStatus;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub(crate) enum AppEvent {
    UserConnected {
        user_id: Uuid,
        sender: Sender<ServerSentEventMessage>,
    },
    UserMessage(ServerSentEventMessage),
    UserDisconnected {
        user_id: Uuid,
    },
    CacheReports(Vec<Report>),
}

impl AppEvent {
    pub(super) fn report_status_update_message(report_stats: ReportStatusUpdate) -> Self {
        AppEvent::UserMessage(ServerSentEventMessage::ReportStatusUpdate(report_stats))
    }

    pub(super) fn new_report(new_report: Report) -> Self {
        AppEvent::UserMessage(ServerSentEventMessage::NewReport(new_report))
    }

    pub(super) fn cache_reports(reports: Vec<Report>) -> Self {
        AppEvent::CacheReports(reports)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ReportStatusUpdate {
    pub(super) id: Uuid,
    pub(super) status: ReportStatus,
}

impl ReportStatusUpdate {
    pub(crate) fn into_report(self, user_id: Uuid) -> Report {
        Report {
            user_id,
            report_id: self.id,
            report_status: self.status,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub(crate) user_id: Uuid,
    pub(crate) report_id: Uuid,
    pub(crate) report_status: ReportStatus,
}

impl Report {
    pub(crate) fn new(user_id: Uuid) -> Self {
        Self::with_all_details(user_id, Uuid::new_v4(), ReportStatus::Pending)
    }
    pub(crate) fn with_all_details(
        user_id: Uuid,
        report_id: Uuid,
        report_status: ReportStatus,
    ) -> Self {
        Self {
            user_id,
            report_id,
            report_status,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ServerSentEventMessage {
    ReportStatusUpdate(ReportStatusUpdate),
    NewReport(Report),
}
