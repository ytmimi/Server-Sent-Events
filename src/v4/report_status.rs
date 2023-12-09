use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Pending,
    Queued,
    Processing,
    Canceled,
    Failed,
    Completed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReportStatusError {
    /// Failed to write the new status to the database
    DatabaseUpdateFailed,
    /// Could not find the report based on the UUId
    ReportNotFound(Uuid, ReportStatus),
    /// Tried to construct a status that doesn't exist
    InvalidStatus(String),
    /// Failure when trying to update the status of a report
    InvalidStatusTransition {
        current: ReportStatus,
        next_status: ReportStatus,
    },
}

impl FromStr for ReportStatus {
    type Err = ReportStatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let status = match s {
            "pending" | "PENDING" => ReportStatus::Pending,
            "queued" | "QUEUED" => ReportStatus::Queued,
            "processing" | "PROCESSING" => ReportStatus::Processing,
            "canceled" | "CANCELED" => ReportStatus::Canceled,
            "failed" | "FAILED" => ReportStatus::Failed,
            "completed" | "COMPLETED" => ReportStatus::Completed,
            _ => return Err(ReportStatusError::InvalidStatus(s.to_owned())),
        };

        Ok(status)
    }
}

impl ReportStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ReportStatus::Pending => "pending",
            ReportStatus::Queued => "queued",
            ReportStatus::Processing => "processing",
            ReportStatus::Canceled => "canceled",
            ReportStatus::Failed => "failed",
            ReportStatus::Completed => "completed",
        }
    }
    /// Define the state machine that outlines valid report status transitions
    pub(super) fn transition(self, next_status: ReportStatus) -> Result<Self, ReportStatusError> {
        let next_status = match (self, next_status) {
            // When a report is pending it hasn't been created yet.
            // It can either be queued to be worked on or canceled by the user
            (Self::Pending, Self::Queued) => Self::Queued,
            (Self::Pending, Self::Canceled) => Self::Canceled,
            // When a report is queued it will be picked up by a worker.
            // Processing can start or a user can cancel the report before processing begins
            (Self::Queued, Self::Processing) => Self::Processing,
            (Self::Queued, Self::Canceled) => Self::Canceled,
            // Once the report has started it can either fail or succeed
            (Self::Processing, Self::Failed) => Self::Failed,
            (Self::Processing, Self::Completed) => Self::Completed,
            // Allow uers to retry reports that have failed or been canceled
            (Self::Canceled, Self::Pending) => Self::Pending,
            (Self::Failed, Self::Pending) => Self::Pending,
            _ => {
                return Err(ReportStatusError::InvalidStatusTransition {
                    current: self,
                    next_status,
                })
            }
        };

        Ok(next_status)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Helper macro to quickly write tests for valid status transitions
    macro_rules! valid_status_transition {
        ($current_status:expr, $next_status:expr) => {
            assert_eq!($current_status.transition($next_status), Ok($next_status));
        };
    }

    /// Helper macro to quickly write tests for invalid status transitions
    macro_rules! invalid_status_transition {
        ($current_status:expr, $next_status:expr) => {
            assert_eq!(
                $current_status.transition($next_status),
                Err(ReportStatusError::InvalidStatusTransition {
                    current: $current_status,
                    next_status: $next_status
                })
            );
        };
    }

    #[test]
    fn test_pending_transitions() {
        invalid_status_transition!(ReportStatus::Pending, ReportStatus::Pending);
        valid_status_transition!(ReportStatus::Pending, ReportStatus::Queued);
        invalid_status_transition!(ReportStatus::Pending, ReportStatus::Processing);
        valid_status_transition!(ReportStatus::Pending, ReportStatus::Canceled);
        invalid_status_transition!(ReportStatus::Pending, ReportStatus::Failed);
        invalid_status_transition!(ReportStatus::Pending, ReportStatus::Completed);
    }

    #[test]
    fn test_queued_transition() {
        invalid_status_transition!(ReportStatus::Queued, ReportStatus::Pending);
        invalid_status_transition!(ReportStatus::Queued, ReportStatus::Queued);
        valid_status_transition!(ReportStatus::Queued, ReportStatus::Processing);
        valid_status_transition!(ReportStatus::Queued, ReportStatus::Canceled);
        invalid_status_transition!(ReportStatus::Queued, ReportStatus::Failed);
        invalid_status_transition!(ReportStatus::Queued, ReportStatus::Completed);
    }

    #[test]
    fn test_processing_transition() {
        invalid_status_transition!(ReportStatus::Processing, ReportStatus::Pending);
        invalid_status_transition!(ReportStatus::Processing, ReportStatus::Queued);
        invalid_status_transition!(ReportStatus::Processing, ReportStatus::Processing);
        invalid_status_transition!(ReportStatus::Processing, ReportStatus::Canceled);
        valid_status_transition!(ReportStatus::Processing, ReportStatus::Failed);
        valid_status_transition!(ReportStatus::Processing, ReportStatus::Completed);
    }

    #[test]
    fn test_canceled_transition() {
        valid_status_transition!(ReportStatus::Canceled, ReportStatus::Pending);
        invalid_status_transition!(ReportStatus::Canceled, ReportStatus::Queued);
        invalid_status_transition!(ReportStatus::Canceled, ReportStatus::Processing);
        invalid_status_transition!(ReportStatus::Canceled, ReportStatus::Canceled);
        invalid_status_transition!(ReportStatus::Canceled, ReportStatus::Failed);
        invalid_status_transition!(ReportStatus::Canceled, ReportStatus::Completed);
    }

    #[test]
    fn test_failed_transition() {
        valid_status_transition!(ReportStatus::Failed, ReportStatus::Pending);
        invalid_status_transition!(ReportStatus::Failed, ReportStatus::Queued);
        invalid_status_transition!(ReportStatus::Failed, ReportStatus::Processing);
        invalid_status_transition!(ReportStatus::Failed, ReportStatus::Canceled);
        invalid_status_transition!(ReportStatus::Failed, ReportStatus::Failed);
        invalid_status_transition!(ReportStatus::Failed, ReportStatus::Completed);
    }

    #[test]
    fn test_completed_transition() {
        // completed reports cannot transition to any status
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Pending);
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Queued);
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Processing);
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Canceled);
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Failed);
        invalid_status_transition!(ReportStatus::Completed, ReportStatus::Completed);
    }
}
