import { ReportStatus } from "../report";

export function StatusCircle({ status }: { status: ReportStatus }) {
  let color: string;

  if (status == ReportStatus.Pending) {
    color = "yellow";
  } else if (status == ReportStatus.Queued) {
    color = "orange";
  } else if (status == ReportStatus.Processing) {
    color = "#93C47D"; // light green
  } else if (status == ReportStatus.Canceled) {
    color = "#E06666"; // light red
  } else if (status == ReportStatus.Failed) {
    color = "red";
  } else if (status == ReportStatus.Completed) {
    color = "green";
  } else {
    color = "blue";
  }
  return (
    <svg height="10" width="10" overflow="visible" className="status-circl">
      <circle r="10" stroke="black" strokeWidth="1" fill={color} />
    </svg>
  );
}
