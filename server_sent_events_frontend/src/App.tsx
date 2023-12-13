import "./App.css";
import { useState } from "react";
import { ServerSentEventClient } from "./client";
import { Login } from "./components/Login";
import { ReportStatusTable } from "./components/ReportStatusTable";

function App() {
  // userId for testing = "046fe7f4-c0e6-4d51-81ab-572deddc8142"
  const [userId, setUserId] = useState<string>("");
  const baseUrl = "http://localhost:3000/v4";

  if (!userId) {
    return <Login userId={userId} setUserId={setUserId} />;
  } else {
    const client = new ServerSentEventClient(baseUrl, userId);
    return <ReportStatusTable client={client} />;
  }
}

export default App;
