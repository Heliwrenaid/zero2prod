curl -X POST http://localhost:8000/subscriptions \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "email=$1&name=$2"