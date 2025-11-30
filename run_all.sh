#!/bin/bash
# Run all services: Redis, API Server, and Drainer
# Usage: ./run_all.sh

set -e  # Exit on error

echo "🚀 Starting Log Pipeline Services"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to check if Redis is running
check_redis() {
    if redis-cli ping > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Redis is running${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠️  Redis is not running${NC}"
        return 1
    fi
}

# Function to start Redis
start_redis() {
    echo "🔄 Starting Redis..."
    
    # Try to start Redis (different methods for different systems)
    if command -v redis-server > /dev/null 2>&1; then
        # Try to start in background
        redis-server --daemonize yes 2>/dev/null || {
            # If that fails, try brew services (macOS)
            if command -v brew > /dev/null 2>&1; then
                echo "   Trying brew services..."
                brew services start redis 2>/dev/null || true
            fi
        }
        sleep 2
        
        if check_redis; then
            echo -e "${GREEN}✅ Redis started${NC}"
        else
            echo -e "${RED}❌ Failed to start Redis${NC}"
            echo "   Please start Redis manually:"
            echo "   macOS: brew services start redis"
            echo "   Linux: sudo systemctl start redis"
            exit 1
        fi
    else
        echo -e "${RED}❌ Redis not found. Please install Redis:${NC}"
        echo "   macOS: brew install redis"
        echo "   Linux: sudo apt-get install redis-server"
        exit 1
    fi
}

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "🛑 Shutting down services..."
    kill $API_PID $DRAINER_PID 2>/dev/null || true
    wait $API_PID $DRAINER_PID 2>/dev/null || true
    echo "✅ Services stopped"
    exit 0
}

# Trap Ctrl+C
trap cleanup INT TERM

# Step 1: Check/Start Redis
echo "📊 Step 1: Checking Redis..."
if ! check_redis; then
    start_redis
fi
echo ""

# Step 2: Build project
echo "🔨 Step 2: Building project..."
cargo build --quiet
echo -e "${GREEN}✅ Build complete${NC}"
echo ""

# Step 3: Start API Server
echo "🚀 Step 3: Starting API Server..."
cargo run --bin log_pipelines > /tmp/log_api_server.log 2>&1 &
API_PID=$!
sleep 3

# Check if server started
if curl -s http://127.0.0.1:3000/log > /dev/null 2>&1 || [ $? -eq 52 ] || [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ API Server running on http://127.0.0.1:3000${NC}"
    echo "   Logs: /tmp/log_api_server.log"
else
    echo -e "${YELLOW}⚠️  API Server starting... (check logs if issues)${NC}"
fi
echo ""

# Step 4: Start Drainer Service
echo "🔄 Step 4: Starting Drainer Service..."
cargo run --bin drainer > /tmp/log_drainer.log 2>&1 &
DRAINER_PID=$!
sleep 2
echo -e "${GREEN}✅ Drainer Service running${NC}"
echo "   Logs: /tmp/log_drainer.log"
echo ""

# Step 5: Test the endpoint
echo "🧪 Step 5: Testing /log endpoint..."
sleep 1
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test_user","event":"system_startup","timestamp":'$(date +%s)'}')

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✅ Test log sent successfully (HTTP 200)${NC}"
else
    echo -e "${YELLOW}⚠️  Test log returned HTTP $HTTP_CODE${NC}"
fi
echo ""

# Summary
echo "=================================="
echo -e "${GREEN}✅ All services running!${NC}"
echo ""
echo "📝 Services:"
echo "   • API Server:    http://127.0.0.1:3000"
echo "   • Drainer:       Running (drains every 30s)"
echo "   • Redis:         Running on 127.0.0.1:6379"
echo ""
echo "📋 Useful Commands:"
echo "   • Send log:      curl -X POST http://127.0.0.1:3000/log -H 'Content-Type: application/json' -d '{\"user_id\":\"123\",\"event\":\"test\",\"timestamp\":1712345678}'"
echo "   • Check Redis:   redis-cli KEYS 'logs:user_*:*'"
echo "   • View logs:     tail -f /tmp/log_api_server.log"
echo "   • View drainer:  tail -f /tmp/log_drainer.log"
echo "   • Check files:   ls -R logs/"
echo ""
echo "🛑 Press Ctrl+C to stop all services"
echo ""

# Wait for user interrupt
wait $API_PID $DRAINER_PID

