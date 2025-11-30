#!/bin/bash
# Quick test script for /log endpoint

echo "🧪 Testing /log endpoint"
echo "========================"
echo ""

# Test 1: Send a log
echo "📝 Sending log to /log endpoint..."
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "123",
    "event": "clicked_button",
    "timestamp": 1712345678
  }'

echo ""
echo ""

# Test 2: Send another log (same user, different event)
echo "📝 Sending another log..."
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "123",
    "event": "login",
    "timestamp": 1712345680
  }'

echo ""
echo ""

# Test 3: Send log for different user
echo "📝 Sending log for different user..."
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "456",
    "event": "viewed_page",
    "timestamp": 1712345690
  }'

echo ""
echo ""
echo "✅ Test complete!"
echo ""
echo "🔍 Check Redis:"
echo "   redis-cli"
echo "   > KEYS logs:user_*:*"
echo "   > LRANGE logs:user_123:19847 0 -1"
echo ""
echo "📁 Check files (after drainer runs):"
echo "   ls -R logs/"
echo "   cat logs/user_123/19847.jsonl"

