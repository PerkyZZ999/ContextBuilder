.PHONY: build test lint fmt clean release check

build:           ## Build all (Rust + TS)
	cargo build --workspace
	bun install
	bun run build

test:            ## Run all tests
	cargo test --workspace
	bun test

lint:            ## Lint all
	cargo clippy --workspace -- -D warnings
	bunx biome check .

fmt:             ## Format all
	cargo fmt --all
	bunx biome format --write .

check:           ## Quick check (no full build)
	cargo check --workspace --all-targets
	bunx biome check .

clean:           ## Clean build artifacts
	cargo clean
	rm -rf node_modules dist
	find . -name 'node_modules' -type d -prune -exec rm -rf {} +

release:         ## Build release binaries
	cargo build --workspace --release
	bun install
	bun run build

help:            ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-16s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
