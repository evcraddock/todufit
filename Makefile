.PHONY: build test run clean check fmt fmt-check clippy lint release install

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
