.PHONY: help build test lint clean dev dev-stop dev-status svc-start svc-stop

help: ## Display this help message
	@echo "Available targets:"
	@echo ""
	@echo "🔨 Build & Test:"
	@grep -E '^(build|test|lint|clean):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "🚀 Development:"
	@grep -E '^dev.*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "🐳 Docker Services:"
	@grep -E '^svc-.*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build all crates
	@echo "🔨 Building..."
	@cargo build
	@echo "✅ Build complete!"

test: ## Run all tests
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ Tests complete!"

lint: ## Run clippy and format check
	@echo "🔍 Running linters..."
	@cargo fmt --check
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "✅ Linting complete!"

clean: ## Clean build artifacts
	@echo "🧹 Cleaning..."
	@cargo clean
	@echo "✅ Clean complete!"

dev: ## Start development environment (sync server via shoreman)
	@echo "🚀 Starting development environment..."
	@./hack/shoreman.sh

dev-stop: ## Stop development environment
	@if [ -f .shoreman.pid ]; then \
		echo "🛑 Stopping development environment..."; \
		kill $$(cat .shoreman.pid) 2>/dev/null || true; \
		rm -f .shoreman.pid; \
		docker compose down; \
		echo "✅ Development environment stopped!"; \
	else \
		echo "⚠️  Development environment is not running"; \
	fi

dev-status: ## Check development environment status
	@if [ -f .shoreman.pid ]; then \
		PID=$$(cat .shoreman.pid); \
		if ps -p $$PID > /dev/null 2>&1; then \
			echo "✅ Development environment is running (PID: $$PID)"; \
		else \
			echo "⚠️  Development environment not running (stale PID file)"; \
			rm -f .shoreman.pid; \
		fi \
	else \
		echo "⚠️  Development environment not running"; \
	fi

svc-start: ## Start sync server (Docker)
	@echo "🐳 Starting sync server..."
	@docker compose up -d
	@echo "✅ Sync server started on ws://localhost:3030"

svc-stop: ## Stop sync server
	@echo "🛑 Stopping sync server..."
	@docker compose down
	@echo "✅ Sync server stopped!"
