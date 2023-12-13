USER_ID=$1
REPORT_ID=$2
STATUS=$3

URL="http://localhost:3000/v4/report?user_id=${USER_ID}"

DATA="{\"id\":\"${REPORT_ID}\", \"status\":\"${STATUS}\"}"
echo sending: $DATA

curl -X PUT $URL \
    -H "Content-Type: application/json" \
    -d "$DATA"
