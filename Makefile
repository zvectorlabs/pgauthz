# pgauthz dev Makefile

.PHONY: help setup setup-pgrx build build-pgauthz fmt fmt-check check clippy test test-core test-pgauthz-unit test-e2e test-pgauthz test-all ci clean install-hooks

PGAUTHZ_BENCH_DSN ?= host=127.0.0.1 port=28816 user=$(shell whoami) dbname=postgres
PGAUTHZ_PG_CONFIG ?= $(shell cargo pgrx info pg-config pg16 2>/dev/null)

help:
	@echo "pgauthz dev commands"
	@echo ""
	@echo "Setup:"
	@echo "  make setup        - Install cargo-pgrx, rustfmt, clippy"
	@echo "  make setup-pgrx   - Run cargo pgrx init (downloads Postgres 16)"
	@echo "  make install-hooks - Install git hooks"
	@echo ""
	@echo "Build:"
	@echo "  make build        - Build all crates (no pgrx)"
	@echo "  make build-pgauthz - Build pgauthz extension"
	@echo ""
	@echo "Test:"
	@echo "  make test              - Run all non-pg tests"
	@echo "  make test-pgauthz-unit - Run pgauthz unit tests (no Postgres)"
	@echo "  make test-e2e          - Run workspace E2E tests"
	@echo "  make test-pgauthz      - Run pgauthz extension tests (requires setup-pgrx)"
	@echo "  make test-all          - Run all tests"
	@echo ""
	@echo "CI:"
	@echo "  make ci - fmt-check + check + test + clippy"

# --- Setup ---

setup:
	cargo install cargo-pgrx
	rustup component add rustfmt clippy

setup-pgrx:
	cargo pgrx init --pg16 download

install-hooks:
	chmod +x .githooks/pre-commit .githooks/commit-msg
	git config core.hooksPath .githooks

# --- Build ---

build:
	cargo build -p authz-datastore-pgx -p pgauthz

build-pgauthz:
	cargo build -p pgauthz

# --- Check ---

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo check -p authz-datastore-pgx -p pgauthz

clippy:
	cargo clippy -p authz-datastore-pgx -p pgauthz -- -D warnings

# --- Test ---

test-pgauthz-unit:
	cargo test -p pgauthz --lib

test-e2e:
	cargo test --test integration_e2e

test:
	cargo test -p pgauthz-workspace -p pgauthz --test integration_pgauthz --test integration_e2e -- --skip test_pgauthz_extension_e2e

test-pgauthz:
	cargo pgrx test -p pgauthz pg16

test-all:
	@echo "Running all tests..."
	@if ! $(MAKE) test; then echo "❌ Unit tests failed"; exit 1; fi
	@echo "✅ Unit tests passed"
	@if ! $(MAKE) test-pgauthz; then echo "❌ pgauthz tests failed"; exit 1; fi
	@echo "✅ pgauthz tests passed"
	@echo "🎉 All tests passed!"

# --- Run ---

run:
	cargo pgrx run -p pgauthz pg16

# --- CI ---

ci: fmt-check check test clippy

# --- Clean ---

clean:
	cargo clean
