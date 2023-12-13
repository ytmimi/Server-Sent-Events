import { useState, useEffect } from "react";
import { Report } from "../report";
import {
  ServerSentEventClient,
  reportStatusUpdateEventListener,
} from "../client";
import { StatusCircle } from "./StatusCircle";

export function ReportStatusTable({
  client,
}: {
  client: ServerSentEventClient;
}) {
  const [reports, setReports] = useState<Report[]>([]);

  useEffect(() => {
    client
      .listReports()
      .then((data) => setReports(data))
      .catch((e) => console.error(`Getting data failed: ${e.message}`));
  }, [client]);

  client.addReportStatusUpdateEventListener((e) =>
    reportStatusUpdateEventListener(e, reports, setReports),
  );

  return (
    <>
      <h1>Report Status</h1>
      <p>User ID: {client.userId}</p>
      <button
        className="new-report"
        onClick={() => {
          client.newReport().then((data) => {
            if (!data) {
              console.log("Didn't get a new report");
              return;
            }
            const newReports = [...reports];
            newReports.push(data);
            console.log("calling setReports");
            setReports(newReports);
          });
        }}
      >
        Create New Report
      </button>

      <table style={{ width: "100%" }}>
        <thead>
          <tr>
            <th style={{ width: "10%" }}></th>
            <th style={{ width: "45%" }}>Report Status</th>
            <th style={{ width: "45%" }}>Report ID</th>
          </tr>
        </thead>
        <tbody>
          {reports.map((report) => (
            <tr key={report.reportId}>
              <td>
                <StatusCircle status={report.reportStatus} />
              </td>
              <td>{report.reportStatus}</td>
              <td>{report.reportId}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}
