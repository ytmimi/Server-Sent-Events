import { Report, ReportStatusUpdate } from "./report";

// FIXME(ytmimi) I don't know how to type the event type from the server sent event so I'm just using `any`
export function reportStatusUpdateEventListener(
  e: any,
  reportList: Report[],
  reportUpdateFunction: (r: Report[]) => void,
) {
  console.log(`reportStatusUpdateEventListener got data: ${e.data}`);
  const statusUpdate: ReportStatusUpdate = JSON.parse(e.data);

  const reportIndex = reportList.findIndex(
    (value) => value.reportId === statusUpdate.id,
  );

  if (reportIndex === -1) {
    console.log(
      `Could not find a reportId '${statusUpdate.id}' in ${JSON.stringify(
        reportList,
      )}`,
    );
    return;
  }

  const newReportList = [...reportList];
  newReportList[reportIndex].reportStatus = statusUpdate.status;
  reportUpdateFunction(newReportList);
}

export class ServerSentEventClient {
  baseUrl: string;
  userId: string;
  eventSource: EventSource;

  constructor(baseUrl: string, userId: string) {
    this.baseUrl = baseUrl;
    this.userId = userId;

    // URL for server sent events
    const url = `${this.baseUrl}/sse?user_id=${this.userId}`;
    this.eventSource = new EventSource(url);

    // Close the connection if we experience an error
    // FIXME(ytmimi) in a production app we'd want to retry the reconnect
    this.eventSource.onerror = (e: Event) => {
        console.error(e)
        this.eventSource.close();
    }
  }

  // FIXME(ytmimi) I don't know how to type the event type from the server sent event so I'm just using `any`
  addReportStatusUpdateEventListener(listener: (e: any) => void) {
    console.log("setting event listener!!!");
    this.eventSource.addEventListener("report_status_update", listener);
  }

  async listReports(): Promise<Report[]> {
    const url = `${this.baseUrl}/reports?user_id=${this.userId}`;

    const response = await fetch(url);

    if (!response.ok) {
      console.log(response.statusText);
      return [];
    }

    const data = await response.json();
    console.log(`Listing reports ${JSON.stringify(data)}`);

    // The API should return a response of `Report[]`
    // [
    //   {
    //      "userId":"046fe7f4-c0e6-4d51-81ab-572deddc8142",
    //      "reportId":"9c676d07-98b8-4e10-bf9e-dfad292676ef",
    //      "reportStatus":"pending"
    //   }
    // ]
    return data;
  }

  async newReport(): Promise<Report | null> {
    const url = `${this.baseUrl}/new/report?user_id=${this.userId}`;

    const response = await fetch(url, { method: "POST" });

    if (!response.ok) {
      console.log(response.statusText);
      return null;
    }

    const data = await response.json();
    console.log(`New report ${JSON.stringify(data)}`);

    // The API should return a response of `Report`
    // {
    //    "userId":"046fe7f4-c0e6-4d51-81ab-572deddc8142",
    //    "reportId":"9c676d07-98b8-4e10-bf9e-dfad292676ef",
    //    "reportStatus":"pending"
    // }
    return data;
  }
}
