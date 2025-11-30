# Log Pipelines - Rust Log Ingestion Service

A high-performance log ingestion service built with Rust, featuring Redis caching and background file draining.

📊 **See [FLOW.md](./FLOW.md) for detailed architecture and flow diagrams.**

📚 **See [LEARNING_RESOURCES.md](./LEARNING_RESOURCES.md) for comprehensive learning resources.**

## 🏗️ Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ POST /log
       ▼
┌─────────────────┐
│  Axum Server    │  ← Fast API endpoint
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Redis Cache    │  ← In-memory storage (instant writes)
└────────┬────────┘
         │
         │ (Background Drainer)
         │ Every 60 seconds (configurable)
         ▼
┌─────────────────┐
│  File System    │  ← Persistent storage (logs/user_*/)
└─────────────────┘
```

## 🚀 Features

- **Fast Log Ingestion**: Writes go to Redis first (microseconds latency)
- **Background Draining**: Automatic batch writes from Redis to files
- **User-Specific Files**: Logs organized by `user_id` and date
- **Configurable**: TOML-based configuration
- **Type-Safe**: Full Rust type safety with explicit error handling
- **Tested**: Unit tests and integration tests included

## 📦 Installation

### Prerequisites

- Rust (latest stable version)
- Redis installed locally (or use Docker - see below)

### Quick Start

**1. Install Redis (if not already installed):**
```bash
# macOS
brew install redis
brew services start redis

# Linux
sudo apt-get install redis-server
sudo systemctl start redis
```

**2. Verify Redis is running:**
```bash
make redis-check
# or
redis-cli ping  # Should return PONG
```

**3. Start everything:**
```bash
make run
```

This will:
1. Check Redis is running
2. Start the API server
3. Start the drainer service

### Setup

1. Clone the repository:
```bash
git clone <your-repo>
cd logPipelines_rust
```

2. Build the project:
```bash
make build
# or
cargo build
```

3. (Optional) Create a config file:
```bash
# Copy the example config
cp config.toml.example config.toml
# Edit config.toml as needed
```

### Using Make Commands

**Check Redis status:**
```bash
make redis-check
```

**Start API server:**
```bash
make server
```

**Start drainer service:**
```bash
make drainer
```

**Start everything:**
```bash
make run
```

**Stop everything:**
```bash
make stop
```

**View all commands:**
```bash
make help
```

### Using Docker (Optional)

If you prefer Docker instead of local Redis:

**Start Redis in Docker:**
```bash
make redis-up-docker
```

**Use Redis CLI (Docker):**
```bash
make redis-cli-docker
```

**Stop Redis Docker:**
```bash
make redis-down-docker
```

## ⚙️ Configuration

Configuration is managed via `config.toml`. If the file doesn't exist, defaults are used.

### Sample `config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000

[redis]
url = "redis://127.0.0.1:6379"
# How long keys stay in Redis before expiring (in seconds)
key_expiration_seconds = 600  # 10 minutes

[drainer]
# How often to drain Redis cache to files (in seconds)
interval_seconds = 60  # 1 minute
# Redis key pattern to match for draining
log_pattern = "logs:user_*:*"
```

### Default Configuration

- **Server**: `127.0.0.1:3000`
- **Redis**: `redis://127.0.0.1:6379`
- **Drainer Interval**: 60 seconds (1 minute)
- **Key Expiration**: 600 seconds (10 minutes)

## 🎯 Usage

### Quick Start (Recommended)

**Start everything with one command:**
```bash
make run
```

This starts Redis, API server, and drainer automatically!

### Manual Start

**Start the API Server:**
```bash
make server
# or
cargo run
```

**Start the Drainer Service (separate terminal):**
```bash
make drainer
# or
cargo run --bin drainer
```

### Benefits of Separate Services

- **Independent scaling**: Run multiple drainer instances
- **Independent deployment**: Deploy as separate containers/services
- **Fault isolation**: If drainer crashes, API server keeps running
- **Flexible**: Start/stop drainer without affecting the API

### Send Logs

```bash
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "123",
    "event": "clicked_button",
    "timestamp": 1712345678
  }'
```

### Log File Structure

Logs are organized by user and date:
```
logs/
  user_123/
    19847.jsonl  # Day 19847 since epoch
  user_456/
    19847.jsonl
```

Each file contains JSONL (JSON Lines) format:
```json
{"user_id":"123","event":"clicked_button","timestamp":1712345678}
{"user_id":"123","event":"login","timestamp":1712345680}
```

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Run Unit Tests Only

```bash
cargo test --lib
```

### Run Integration Tests

```bash
# Tests that don't require Redis
cargo test --test integration_test

# Tests that require Redis (ignored by default)
cargo test --test api_test -- --ignored
```

### Test Coverage

- **Unit Tests**: 
  - Key/path generation
  - JSON serialization/deserialization
  - Error handling
  - Configuration loading

- **Integration Tests**:
  - API endpoint validation
  - Redis operations
  - File operations

## 📁 Project Structure

```
src/
  ├── main.rs              # API server and entry point
  ├── lib.rs               # Library exports (for testing)
  ├── types.rs             # Shared types (LogEvent, AppError, AppState)
  ├── file_redis_layer.rs  # Redis cache operations
  ├── drainer.rs           # Background drainer service
  └── config.rs            # Configuration management

tests/
  ├── integration_test.rs  # Integration tests
  └── api_test.rs          # API tests (requires Redis)

config.toml                # Configuration file (optional)
```

## 🔧 Module Details

### `types.rs`
- `LogEvent`: Log entry structure
- `AppError`: Custom error types
- `AppState`: Shared application state

### `file_redis_layer.rs`
- `write_to_cache()`: Write logs to Redis
- `read_from_cache()`: Read logs from Redis (for future use)
- `get_redis_key()`: Generate Redis keys
- `get_log_file_path()`: Generate file paths

### `drainer.rs`
- `start_drainer()`: Background service that drains Redis to files
- `drain_key_to_file()`: Drain a single Redis key to file

### `config.rs`
- `Config`: Application configuration
- `load()`: Load config from `config.toml` or use defaults
- `create_sample_config()`: Generate sample config file

## 🐛 Debugging

### Check Redis Connection

```bash
redis-cli ping
# Should return: PONG
```

### View Redis Keys

```bash
redis-cli
> KEYS logs:user_*
> LRANGE logs:user_123:19847 0 -1
```

### Check Log Files

```bash
# List all log directories
ls -R logs/

# View a specific log file
cat logs/user_123/19847.jsonl
```

### Enable Debug Logging

Add to your `config.toml`:
```toml
[logging]
level = "debug"
```

## 🚧 Future Enhancements

- [ ] `/stats` endpoint (read from Redis + files)
- [ ] Manual drain trigger endpoint
- [ ] Log filtering by event type
- [ ] User-specific log queries
- [ ] Metrics and monitoring
- [ ] Docker support
- [ ] Kubernetes deployment configs

## 📝 API Reference

### `POST /log`

Ingest a log event.

**Request Body:**
```json
{
  "user_id": "string",
  "event": "string",
  "timestamp": 1234567890
}
```

**Response:**
- `200 OK`: Log successfully cached
- `400 Bad Request`: Invalid JSON or missing fields
- `500 Internal Server Error`: Redis or file system error

**Example:**
```bash
curl -X POST http://127.0.0.1:3000/log \
  -H "Content-Type: application/json" \
  -d '{"user_id":"123","event":"clicked_button","timestamp":1712345678}'
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Ensure all tests pass
5. Submit a pull request

## 📄 License

[Your License Here]

## 🙏 Acknowledgments

Built as a learning project to understand:
- Rust async/await patterns
- Redis caching strategies
- Background task processing
- Type-safe API design

