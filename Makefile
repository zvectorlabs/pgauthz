# pgauthz dev Makefile
#
# Most targets accept PG_VERSION to select a Postgres version:
#   make test-pgauthz PG_VERSION=17
#   make test-pgauthz PG_VERSION=18
#   make package      PG_VERSION=18
#
# Supported values: 16 (default), 17, 18

.PHONY: help setup setup-pgrx setup-pgrx-pg17 setup-pgrx-pg18 \
        build build-pgauthz fmt fmt-check check clippy \
        test test-pgauthz-unit test-e2e test-pgauthz test-all \
        run ci package install-hooks clean bump

PG_VERSION ?= 16
PGAUTHZ_BENCH_DSN ?= host=127.0.0.1 port=28816 user=$(shell whoami) dbname=postgres
PGAUTHZ_PG_CONFIG ?= $(shell cargo pgrx info pg-config pg$(PG_VERSION) 2>/dev/null)

# On macOS, Homebrew's icu4c is keg-only (not linked into /usr/local).
# Detect and add it to PKG_CONFIG_PATH so pgrx can compile Postgres from source.
ICU_PREFIX := $(shell brew --prefix icu4c@78 2>/dev/null || brew --prefix icu4c 2>/dev/null)
ifneq ($(ICU_PREFIX),)
  export PKG_CONFIG_PATH := $(ICU_PREFIX)/lib/pkgconfig:$(PKG_CONFIG_PATH)
endif

help:
	@echo "pgauthz dev commands"
	@echo ""
	@echo "Most targets accept PG_VERSION=<version> (default: 16)"
	@echo "  Supported versions: 16, 17, 18"
	@echo "  Example: make test-pgauthz PG_VERSION=17"
	@echo ""
	@echo "Setup:"
	@echo "  make setup             - Install cargo-pgrx, rustfmt, clippy"
	@echo "  make setup-pgrx        - Init pgrx with PG_VERSION (default: pg16, downloads if needed)"
	@echo "  make setup-pgrx-pg17   - Download and init pg17 via pgrx"
	@echo "  make setup-pgrx-pg18   - Download and init pg18 via pgrx"
	@echo "  make install-hooks     - Install git hooks"
	@echo ""
	@echo "Build:"
	@echo "  make build             - Build all crates (no pgrx)"
	@echo "  make build-pgauthz     - Build pgauthz extension"
	@echo ""
	@echo "Test:"
	@echo "  make test                         - Run non-pg tests"
	@echo "  make test-pgauthz-unit            - Run pgauthz unit tests (no Postgres)"
	@echo "  make test-e2e                     - Run workspace E2E tests"
	@echo "  make test-pgauthz [PG_VERSION=NN] - Run pgauthz pgrx tests against given PG"
	@echo "  make test-all     [PG_VERSION=NN] - Run all tests against given PG"
	@echo ""
	@echo "Release:"
	@echo "  make bump VERSION=x.y.z       - Bump version in all files + create git tag"
	@echo "  make package [PG_VERSION=NN]  - Build release package locally"
	@echo "  git push --tags               - Push tag to trigger GitHub release workflow"
	@echo ""
	@echo "CI:"
	@echo "  make ci [PG_VERSION=NN] - fmt-check + check + test + clippy"

# --- Setup ---

setup:
	cargo install cargo-pgrx
	rustup component add rustfmt clippy

setup-pgrx:
	cargo pgrx init --pg$(PG_VERSION) download

setup-pgrx-pg17:
	cargo pgrx init --pg17 download

setup-pgrx-pg18:
	cargo pgrx init --pg18 download

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
	cargo check -p authz-datastore-pgx -p pgauthz \
		--features pg$(PG_VERSION) --no-default-features

clippy:
	cargo clippy -p authz-datastore-pgx -p pgauthz \
		--features pg$(PG_VERSION) --no-default-features -- -D warnings

# --- Test ---

test-pgauthz-unit:
	cargo test -p pgauthz --lib

test-e2e:
	cargo test --test integration_e2e

test:
	cargo test -p pgauthz-workspace -p pgauthz \
		--test integration_pgauthz --test integration_e2e \
		-- --skip test_pgauthz_extension_e2e

test-pgauthz:
	cargo pgrx test -p pgauthz pg$(PG_VERSION)

test-all:
	@echo "Running all tests (PG_VERSION=$(PG_VERSION))..."
	@if ! $(MAKE) test; then echo "❌ Unit tests failed"; exit 1; fi
	@echo "✅ Unit tests passed"
	@if ! $(MAKE) test-pgauthz PG_VERSION=$(PG_VERSION); then echo "❌ pgauthz tests failed"; exit 1; fi
	@echo "✅ pgauthz tests passed"
	@echo "🎉 All tests passed!"

# --- Run ---

run:
	cargo pgrx run -p pgauthz pg$(PG_VERSION)

# --- CI ---

ci: fmt-check check test clippy

# --- Package ---

package:
	cargo pgrx package -p pgauthz \
		--pg-config $(PGAUTHZ_PG_CONFIG) \
		--features pg$(PG_VERSION) \
		--no-default-features \
		--profile release

# --- Release / Version bump ---
#
# VERSION must be set explicitly, e.g.:  make bump VERSION=0.2.0
#
# What this updates (all must stay in sync for a correct release):
#   1. crates/pgauthz/Cargo.toml          — Rust crate version (shown in Cargo.lock)
#   2. crates/authz-datastore-pgx/Cargo.toml — datastore crate version
#   3. crates/pgauthz/pgauthz.control     — default_version shown by PostgreSQL
#                                            (SELECT extversion FROM pg_extension)
#   4. META.json                           — PGXN registry metadata
#
# After bumping, the git tag (v$(VERSION)) drives the release workflow:
#   - Artifact filenames embed the tag:  pgauthz-0.2.0-pg18-ubuntu2404-amd64.tar.gz
#   - GitHub Release title = tag name
#   - Docker images are pushed to ghcr.io/zvectorlabs/pgauthz
#
# Release workflow:
#   make bump VERSION=0.2.0   # edits files, commits, tags
#   git push && git push --tags  # triggers CI + release workflow

bump:
ifndef VERSION
	$(error VERSION is not set. Usage: make bump VERSION=x.y.z)
endif
	# Require a clean working tree so the bump commit contains only version changes.
	# Commit or stash any pending work before running this target.
	@if ! git diff --quiet || ! git diff --cached --quiet; then \
		echo "Error: working tree has uncommitted changes."; \
		echo "Commit or stash them first, then re-run: make bump VERSION=$(VERSION)"; \
		git status --short; \
		exit 1; \
	fi
	@echo "Bumping version to $(VERSION)..."
	# 1. Cargo.toml — pgauthz crate
	#    Matches only the top-level `version = "..."` line (starts with "version =")
	sed -i.bak 's/^version = "[0-9]*\.[0-9]*\.[0-9]*"/version = "$(VERSION)"/' \
		crates/pgauthz/Cargo.toml && rm crates/pgauthz/Cargo.toml.bak
	# 2. Cargo.toml — authz-datastore-pgx crate (same pattern)
	sed -i.bak 's/^version = "[0-9]*\.[0-9]*\.[0-9]*"/version = "$(VERSION)"/' \
		crates/authz-datastore-pgx/Cargo.toml && rm crates/authz-datastore-pgx/Cargo.toml.bak
	# 3. pgauthz.control — default_version is what PostgreSQL shows in:
	#      SELECT extversion FROM pg_extension WHERE extname = 'pgauthz';
	sed -i.bak "s/default_version = '[0-9]*\.[0-9]*\.[0-9]*'/default_version = '$(VERSION)'/" \
		crates/pgauthz/pgauthz.control && rm crates/pgauthz/pgauthz.control.bak
	# 4. META.json — use python3 for safe JSON editing so we only touch
	#    the extension version fields and never the meta-spec version ("1.0.0")
	python3 -c "\
import json; \
f = open('META.json'); data = json.load(f); f.close(); \
data['version'] = '$(VERSION)'; \
data['provides']['pgauthz']['version'] = '$(VERSION)'; \
f = open('META.json', 'w'); json.dump(data, f, indent=2); f.write('\n'); f.close()"
	# Update Cargo.lock so lockfile version matches crate versions
	cargo generate-lockfile
	# Run fmt before staging so the pre-commit hook finds nothing left to reformat.
	# Using git add -u scopes staging to already-tracked files only, picking up
	# any files rustfmt touched beyond the ones we explicitly edited above.
	cargo fmt --all
	git add -u -- crates/ META.json Cargo.lock
	@echo ""
	@echo "Updated files:"
	@grep '^version' crates/pgauthz/Cargo.toml crates/authz-datastore-pgx/Cargo.toml
	@grep 'default_version' crates/pgauthz/pgauthz.control
	@python3 -c "import json; d=json.load(open('META.json')); print('META.json: version =', d['version'])"
	@echo ""
	# Commit — pre-commit hook will re-run check+clippy as a final guard
	git commit -m "chore: bump version to $(VERSION)"
	git tag v$(VERSION)
	@echo ""
	@echo "✅ Version bumped to $(VERSION) and tagged v$(VERSION)."
	@echo "   Run: git push && git push --tags"
	@echo ""
	@echo "✅ Version bumped to $(VERSION) and tagged v$(VERSION)."
	@echo "   Run: git push && git push --tags"

# --- Clean ---

clean:
	cargo clean
