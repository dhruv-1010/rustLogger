.PHONY: help redis-check redis-up-docker redis-down-docker redis-cli-docker build run server drainer test clean

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

redis-check: ## Check if Redis is running (local)
	@echo "📊 Checking Redis status..."
	@redis-cli ping > /dev/null 2>&1 && echo "✅ Redis is running" || echo "❌ Redis is not running. Install and start it:"
	@echo "   macOS: brew install redis && brew services start redis"
	@echo "   Linux: sudo apt-get install redis-server && sudo systemctl start redis"
	@echo "   Or use Docker: make redis-up-docker"

redis-up-docker: ## Start Redis using Docker (optional)
	@echo "🚀 Starting Redis in Docker..."
	docker-compose up -d redis
	@echo "✅ Redis is running on port 6379"
	@echo "   Check status: make redis-cli-docker"

redis-down-docker: ## Stop Redis Docker container
	@echo "🛑 Stopping Redis Docker container..."
	docker-compose down
	@echo "✅ Redis stopped"

redis-cli-docker: ## Open Redis CLI (Docker)
	@echo "🔧 Opening Redis CLI (Docker)..."
	docker-compose exec redis redis-cli

redis-cli: ## Open Redis CLI (local)
	@echo "🔧 Opening Redis CLI..."
	@redis-cli || echo "❌ Redis CLI not found. Install Redis or use: make redis-cli-docker"

build: ## Build the project
	@echo "🔨 Building project..."
	cargo build

run: redis-check ## Start everything (API server + Drainer) - requires local Redis
	@echo "🚀 Starting full stack..."
	@./run_all.sh

run-simple: redis-check ## Start everything in background (simpler)
	@echo "🚀 Starting full stack in background..."
	@echo "   Starting API server..."
	@cargo run --bin log_pipelines > /tmp/log_api_server.log 2>&1 &
	@sleep 2
	@echo "   Starting drainer service..."
	@cargo run --bin drainer > /tmp/log_drainer.log 2>&1 &
	@echo "✅ Full stack running!"
	@echo "   API Server: http://127.0.0.1:3000"
	@echo "   View logs: tail -f /tmp/log_api_server.log"
	@echo "   To stop: make stop"

server: redis-check ## Start only the API server (requires local Redis)
	@echo "🚀 Starting API server..."
	cargo run --bin log_pipelines

drainer: redis-check ## Start only the drainer service (requires local Redis)
	@echo "🔄 Starting drainer service..."
	cargo run --bin drainer

test: ## Run all tests
	@echo "🧪 Running tests..."
	cargo test

test-integration: ## Run integration tests
	@echo "🧪 Running integration tests..."
	cargo test --test integration_test

test-api: redis-check ## Run API tests (requires Redis)
	@echo "🧪 Running API tests..."
	cargo test --test api_test -- --ignored

clean: ## Clean build artifacts
	@echo "🧹 Cleaning..."
	cargo clean
	@docker-compose down -v 2>/dev/null || true

stop: ## Stop all services
	@echo "🛑 Stopping all services..."
	@pkill -f "cargo run" || true
	@docker-compose down 2>/dev/null || true
	@echo "✅ All services stopped"

setup: ## Initial setup (build + check Redis)
	@echo "⚙️  Setting up project..."
	cargo build
	@make redis-check
	@echo "✅ Setup complete!"
	@echo "   Start server: make server"
	@echo "   Start drainer: make drainer"
	@echo ""
	@echo "   If Redis is not installed, use Docker: make redis-up-docker"

