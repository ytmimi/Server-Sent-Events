Server Sent Events (SSE)

To quote developer.mozilla.org's [Server-sent-events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events) page:

> Traditionally, a web page has to send a request to the server to receive new data; that is, the page requests data from the server. With server-sent events, it's possible for a server to send new data to a web page at any time, by pushing messages to the web page

[This article](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events) gives a very good overview of how to use
Server Sent Events, and what the message format looks like.

## V1

This is the most basic server sent event example. This creates a single `/v1/sse/` route that the user can
make a request to.

### Start the App

```
cargo run
```

### Receive a stream of messags

```
curl "http://localhost:3000/v1/sse?username=demo"
```

The output should look something like this:

```
data: hi!

: keep-alive-text

: keep-alive-text

: keep-alive-text

: keep-alive-text

data: hi!
```

## V2

Internally the app has been structured to pass messages around via channels.
When a user connects to the app a new multiple producer single consume (mpsc) channel is opened.
The receiving end of the channel is used to stream data to the client, while the transmitting end of the channel is stored in a map.
When messages come in for a given user the app looks up the transmitter in the map and sends the intend user a message.
When a client disconnects their transmitter is removed from the map.

### Start the App

```
cargo run
```

### Receive a stream of messags V2

Notice the `v2` in the URL instead of `v1`.

```
curl "http://localhost:3000/v2/sse?username=demo"
```

### Sending custom messages

The `v1` app only sent a steady stream of `hi!` messages to the user.
The `v2` app allows you to send custom messages to users by making POST requests to the `/v2/message/` route.

```
curl -X POST "http://localhost:3000/v2/message?username=demo" -d "Howdy 🤠 from version 2️⃣"
```


## V3

The v3 app is the same as the v2 app. The only difference is that support is added to listen for kafka messages on the `v3_messages` topic.

### Run the Required Docker Containers

```
docker-compose up -d --build
```

### Start the App

```
cargo run
```

### Receive a stream of messags V3

```
curl "http://localhost:3000/v3/sse?username=demo"
```

### Send messages via the API endpoint

```
curl -X POST "http://localhost:3000/v3/message?username=demo" -d "Howdy 🤠 from version 3️⃣"
```

### Send messages via kafka

kafka messages are expected to be in the form `username:message`

```
# execute a bash shell inside the docker container
docker-compose exec kafka bash

# run the kafka-console-producer script
kafka-console-producer --topic v3_messages --broker-list localhost:9092
>demo:🦋 howdy from kafka!
```

## V4

The v4 app adds a user interface and models a report status update UI. Users's can create new reports, and in a separate
terminal window you can run the `update_report_status.sh` script to change the status of a given report, which will cause
the UI to update.

### Run the Required Docker Containers

In case docker isn't up run:

```
docker-compose up -d --build
```

### Create the necessary DynamoDB Tables

This app uses DynamoDB to persist reports between session. To create the required tables run:

```
./dynamodb_local_init.sh
```

### Start the backend web server

```
cargo run
```

### Serve the frontend react app

You may need to install a newer version of node if you run into errors running the app.

```
cd server_sent_events_frontend
npm install
npm run dev
```

### Generate a UUID to login

on linux and macOS you can use the `uuidgen` command line tool to generate a new UUID to use
as your userID.

```
uuidgen
```

### Create New Reports

After logging in with a new UUID click the `Create New Report` button to create as many new reports as you'd like

### Start sending events

All new reports start off in a state of `pending`. The report status update app models a finite state machine and only accepts
valid state transitions. You can run the `update_report_status.sh` to more easily update reports state.

```
update_report_status.sh <userID> <reportID> <new-status>
```

Here is the complete list of valid state transitions:

* `pending`    -> `queued`
* `pending`    -> `canceled`
* `queued`     -> `processing`
* `queued`     -> `canceled`
* `processing` -> `completed`
* `processing` -> `failed`
* `failed`     -> `pending` (to model retries)
* `canceled`   -> `pending` (to model retries)
