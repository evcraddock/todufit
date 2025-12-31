.PHONY: build test run clean check fmt fmt-check clippy lint release install dev dev-stop dev-logs

# Default target
all: build

# Build debug binary
build:
	cargo build

# Run all tests
test:
	cargo test

# Run the application (pass ARGS for arguments: make run ARGS="dish create Test")
run:
	cargo run -- $(ARGS)

# Clean build artifacts
clean:
	cargo clean

# Fast compile check without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Check formatting without modifying
fmt-check:
	cargo fmt --check

# Run clippy linter (allow dead_code during incremental development)
clippy:
	cargo clippy -- -D warnings -A dead_code

# Run all lints (format check + clippy)
lint: fmt-check clippy

# Build optimized release binary
release:
	cargo build --release

# Install locally
install:
	cargo install --path .

# Run tests and lints (good for CI/pre-commit)
ci: lint test

# ============================================================================
# Development Environment
# ============================================================================

# Start development environment via shoreman
dev:
	@./hack/shoreman.sh

# Stop development environment
dev-stop:
	@if [ -f .shoreman.pid ]; then \
		pid=$$(cat .shoreman.pid); \
		if kill -0 $$pid 2>/dev/null; then \
			echo "Stopping development environment (PID: $$pid)..."; \
			kill $$pid; \
			rm -f .shoreman.pid; \
			echo "Stopped."; \
		else \
			echo "Process not running, cleaning up stale PID file."; \
			rm -f .shoreman.pid; \
		fi \
	else \
		echo "Development environment is not running."; \
	fi

# Tail development logs
dev-logs:
	@if [ -f dev.log ]; then \
		tail -f dev.log; \
	else \
		echo "No dev.log found. Start the dev environment with 'make dev' first."; \
	fi
