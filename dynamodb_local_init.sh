#!/bin/bash

# Check if aws command exists
aws --version >/dev/null 2>&1 || {
    echo >&2 "I require aws but it's not installed. Aborting..."
    exit 1
}

# This port was just picked at random
DYNAMO_DB_LOCAL_PORT=8111
IS_PORT_ACTIVE=$(lsof -i:$DYNAMO_DB_LOCAL_PORT)
DYNAMO_DB_LOCAL_URL=http://localhost:$DYNAMO_DB_LOCAL_PORT

if [ -z "$IS_PORT_ACTIVE" ]; then
    echo Make sure that DynamoDB Local is running on $DYNAMO_DB_LOCAL_URL
    exit 1
fi

echo DynamoDB is running at $DYNAMO_DB_LOCAL_URL

PROFILE=default
TABLE_NAME=report_status

DOES_TABLE_EXIST=$(aws dynamodb list-tables --profile $PROFILE --endpoint-url $DYNAMO_DB_LOCAL_URL | grep $TABLE_NAME)

# -n checks if the string length is greater than 0
if [ -n "$DOES_TABLE_EXIST" ]; then
    echo Table $TABLE_NAME already exists. Deleting and then recreating $TABLE_NAME
    aws --profile $PROFILE --endpoint-url $DYNAMO_DB_LOCAL_URL dynamodb delete-table --table-name $TABLE_NAME 2>&1 >/dev/null
    aws --profile $PROFILE --endpoint-url $DYNAMO_DB_LOCAL_URL dynamodb wait table-not-exists --table-name $TABLE_NAME
fi

READ_CAPACITY=10
WRITE_CAPACITY=10

# Create the report_status table with a primary key consists of report_id (partition key)
# We add a secondary index on the `user_id` column so that we can lookup a users reports
aws --profile $PROFILE \
    --endpoint-url $DYNAMO_DB_LOCAL_URL \
    dynamodb create-table \
    --table-name $TABLE_NAME \
    --attribute-definitions \
        AttributeName=report_id,AttributeType=S \
        AttributeName=user_id,AttributeType=S \
    --key-schema \
        AttributeName=report_id,KeyType=HASH \
    --provisioned-throughput ReadCapacityUnits=$READ_CAPACITY,WriteCapacityUnits=$WRITE_CAPACITY \
    --global-secondary-indexes \
        "[
            {
                \"IndexName\": \"UserIdIndex\",
                \"KeySchema\": [
                    {\"AttributeName\":\"user_id\",\"KeyType\":\"HASH\"}
                ],
                \"Projection\": {
                    \"ProjectionType\":\"ALL\"
                },
                \"ProvisionedThroughput\": {
                    \"ReadCapacityUnits\": 10,
                    \"WriteCapacityUnits\": 5
                }
            }
        ]"
