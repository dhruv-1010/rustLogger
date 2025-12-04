.PHONY: help redis-check redis-up-docker redis-down-docker redis-cli-docker build run server drainer test clean docker-build docker-run-server docker-run-drainer docker-stop

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
	@pkill -f "log_pipelines" || true
	@pkill -f "drainer" || true
	@lsof -ti:3000 | xargs kill -9 2>/dev/null || true
	@docker-compose down 2>/dev/null || true
	@echo "✅ All services stopped"

kill: stop ## Alias for stop command

setup: ## Initial setup (build + check Redis)
	@echo "⚙️  Setting up project..."
	cargo build
	@make redis-check
	@echo "✅ Setup complete!"
	@echo "   Start server: make server"
	@echo "   Start drainer: make drainer"
	@echo ""
	@echo "   If Redis is not installed, use Docker: make redis-up-docker"

docker-build: ## Build Docker image
	@echo "🐳 Building Docker image..."
	docker build -t log-pipelines .
	@echo "✅ Docker image built: log-pipelines"

docker-run-server: redis-up-docker ## Run API server in Docker
	@echo "🚀 Starting API server in Docker..."
	@if [ ! -f config.toml ]; then \
		echo "⚠️  config.toml not found, using defaults"; \
	fi
	@docker stop log-pipelines-server 2>/dev/null || true
	@docker rm log-pipelines-server 2>/dev/null || true
	@NETWORK=$$(docker inspect log_pipelines_redis --format '{{range $$k, $$v := .NetworkSettings.Networks}}{{$$k}}{{end}}' 2>/dev/null | head -1); \
	if [ -z "$$NETWORK" ]; then \
		echo "⚠️  Redis container not found, starting it..."; \
		make redis-up-docker; \
		sleep 2; \
		NETWORK=$$(docker inspect log_pipelines_redis --format '{{range $$k, $$v := .NetworkSettings.Networks}}{{$$k}}{{end}}' 2>/dev/null | head -1); \
	fi; \
	docker run -d --name log-pipelines-server \
		-p 3000:3000 \
		-v $$(pwd)/config.toml:/app/config.toml:ro \
		-v $$(pwd)/logs:/app/logs \
		--network $$NETWORK \
		log-pipelines /app/log_pipelines
	@echo "✅ API server running on http://localhost:3000"
	@echo "   View logs: docker logs -f log-pipelines-server"

docker-run-drainer: redis-up-docker ## Run drainer service in Docker
	@echo "🔄 Starting drainer service in Docker..."
	@if [ ! -f config.toml ]; then \
		echo "⚠️  config.toml not found, using defaults"; \
	fi
	@docker stop log-pipelines-drainer 2>/dev/null || true
	@docker rm log-pipelines-drainer 2>/dev/null || true
	@NETWORK=$$(docker inspect log_pipelines_redis --format '{{range $$k, $$v := .NetworkSettings.Networks}}{{$$k}}{{end}}' 2>/dev/null | head -1); \
	if [ -z "$$NETWORK" ]; then \
		echo "⚠️  Redis container not found, starting it..."; \
		make redis-up-docker; \
		sleep 2; \
		NETWORK=$$(docker inspect log_pipelines_redis --format '{{range $$k, $$v := .NetworkSettings.Networks}}{{$$k}}{{end}}' 2>/dev/null | head -1); \
	fi; \
	docker run -d --name log-pipelines-drainer \
		-v $$(pwd)/config.toml:/app/config.toml:ro \
		-v $$(pwd)/logs:/app/logs \
		--network $$NETWORK \
		log-pipelines /app/drainer
	@echo "✅ Drainer service running"
	@echo "   View logs: docker logs -f log-pipelines-drainer"

docker-stop: ## Stop Docker containers
	@echo "🛑 Stopping Docker containers..."
	@docker stop log-pipelines-server log-pipelines-drainer 2>/dev/null || true
	@docker rm log-pipelines-server log-pipelines-drainer 2>/dev/null || true
	@echo "✅ Docker containers stopped"

