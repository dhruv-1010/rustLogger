# Quick Start Guide

## 🚀 Run Everything (One Command)

```bash
./run_all.sh
```

This script will:
1. ✅ Check/Start Redis
2. ✅ Build the project
3. ✅ Start API Server (port 3000)
4. ✅ Start Drainer Service
5. ✅ Test the endpoint

Press `Ctrl+C` to stop all services.

## 📋 Manual Commands

### Start Services Individually

**1. Start Redis:**
```bash
# macOS
brew services start redis

# Linux
sudo systemctl start redis

# Or use Docker
make redis-up-docker
```

**2. Start API Server:**
```bash
cargo run --bin log_pipelines
# or
make server
```

**3. Start Drainer:**
```bash
cargo run --bin drainer
# or
make drainer
```

### Test the /log Endpoint

```bash
# Send a log
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "123",
    "event": "clicked_button",
    "timestamp": 1712345678
  }'

# Or use the test script
./test_log.sh
```

### Check Where Logs Are Stored

**1. Check Redis (immediately after POST):**
```bash
redis-cli
> KEYS logs:user_*:*
> LRANGE logs:user_123:19847 0 -1
```

**2. Check Files (after drainer runs, ~30 seconds):**
```bash
ls -R logs/
cat logs/user_123/19847.jsonl
```

## 🔍 Debugging

**View API Server logs:**
```bash
tail -f /tmp/log_api_server.log
```

**View Drainer logs:**
```bash
tail -f /tmp/log_drainer.log
```

**Check if services are running:**
```bash
# Check API server
curl http://127.0.0.1:3000/log  # Should return 405 (Method Not Allowed) or 400

# Check Redis
redis-cli ping  # Should return PONG
```

## 🛑 Stop Services

**If using run_all.sh:**
- Press `Ctrl+C`

**If running manually:**
```bash
# Stop API server and drainer
pkill -f "cargo run"

# Stop Redis (if started manually)
# macOS
brew services stop redis

# Linux
sudo systemctl stop redis
```

## 📊 Log Flow

```
1. POST /log → API Server (main.rs:30)
   ↓
2. write_to_cache() → Redis (file_redis_layer.rs:21)
   ↓
3. Redis Key: logs:user_123:19847
   ↓
4. Drainer (every 30s) → Reads from Redis
   ↓
5. File: logs/user_123/19847.jsonl
```

## 🧪 Testing

**Run all tests:**
```bash
cargo test
```

**Run tests that require Redis:**
```bash
cargo test --test api_test -- --ignored
```

