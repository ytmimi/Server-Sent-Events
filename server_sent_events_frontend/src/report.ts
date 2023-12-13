export enum ReportStatus {
  Pending = "pending",
  Queued = "queued",
  Processing = "processing",
  Canceled = "canceled",
  Failed = "failed",
  Completed = "completed",
}

export type Report = {
  userId: string;
  reportId: string;
  reportStatus: ReportStatus;
};

export type ReportStatusUpdate = {
  id: string;
  status: ReportStatus;
};
