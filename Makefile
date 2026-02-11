# mudd.one - Veterinary Ultrasound Processing Pipeline
# Rust workspace: mudd-core + mudd-ffi
# Created by M&K (c)2026 VetCoders

.PHONY: all build release check fmt fmt-check lint test test-quick ci fix clean help \
        hooks-install hooks-uninstall pre-commit pre-push \
        version bump bump-patch bump-minor bump-major \
        bindings app xcode

SHELL := /bin/bash
VERSION_FILE := Cargo.toml

# ============================================================================
# Build
# ============================================================================

all: check

build:
	@echo "Building (debug)..."
	@cargo build --workspace

release:
	@echo "Building (release)..."
	@cargo build --workspace --release

# ============================================================================
# Quality
# ============================================================================

fmt:
	@cargo fmt --all

fmt-check:
	@cargo fmt --all -- --check

lint:
	@echo "=== Format Check ==="
	@cargo fmt --all -- --check
	@echo "=== Clippy ==="
	@cargo clippy --workspace --all-targets -- -D warnings

check:
	@echo "=== Format Check ==="
	@cargo fmt --all -- --check
	@echo "=== Clippy (workspace, all targets) ==="
	@cargo clippy --workspace --all-targets -- -D warnings
	@echo "Quality gate passed"

test:
	@echo "=== Tests (workspace) ==="
	@cargo test --workspace

test-quick:
	@echo "=== Tests (quick, lib only) ==="
	@cargo test --workspace --lib

ci: fmt-check lint test
	@echo "CI passed"

fix:
	@echo "=== Auto-fix ==="
	@cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged
	@cargo fmt --all
	@echo "Fixed"

# ============================================================================
# Version Bump
# ============================================================================

version:
	@grep '^version' $(VERSION_FILE) | head -1 | sed 's/.*"\(.*\)"/v\1/'

bump:
	@if [ -z "$(TYPE)" ]; then \
		echo "Usage: make bump TYPE=patch|minor|major"; \
		echo "Current: $$(grep '^version' $(VERSION_FILE) | head -1 | sed 's/.*\"\(.*\)\"/v\1/')"; \
		exit 1; \
	fi
	@current=$$(grep '^version' $(VERSION_FILE) | head -1 | sed 's/.*"\(.*\)"/\1/'); \
	IFS='.' read -r major minor patch <<< "$$current"; \
	case "$(TYPE)" in \
		patch) patch=$$((patch + 1)) ;; \
		minor) minor=$$((minor + 1)); patch=0 ;; \
		major) major=$$((major + 1)); minor=0; patch=0 ;; \
		*) echo "Invalid TYPE: $(TYPE)"; exit 1 ;; \
	esac; \
	new="$$major.$$minor.$$patch"; \
	sed -i '' "s/^version = \"$$current\"/version = \"$$new\"/" $(VERSION_FILE); \
	echo "Bumped: v$$current -> v$$new"

bump-patch:
	@$(MAKE) bump TYPE=patch

bump-minor:
	@$(MAKE) bump TYPE=minor

bump-major:
	@$(MAKE) bump TYPE=major

# ============================================================================
# UniFFI Bindings
# ============================================================================

bindings:
	@echo "Building mudd-ffi (release)..."
	@cargo build -p mudd-ffi --release
	@echo "Generating Swift bindings..."
	@cargo run -p uniffi-bindgen -- generate --library target/release/libmudd_ffi.dylib --language swift --out-dir app/mudd/Bridge/
	@echo "Bindings ready: app/mudd/Bridge/"

# ============================================================================
# macOS App
# ============================================================================

xcode:
	@cd app && xcodegen generate
	@echo "Xcode project generated: app/mudd.xcodeproj"

app: bindings xcode
	@echo "Building mudd.app..."
	@xcodebuild -project app/mudd.xcodeproj -scheme mudd -configuration Debug build 2>&1 | tail -3
	@echo "App built"

# ============================================================================
# Git Hooks
# ============================================================================

hooks-install:
	@echo "Installing git hooks..."
	@cp .githooks/pre-commit .git/hooks/pre-commit
	@cp .githooks/pre-push .git/hooks/pre-push
	@chmod +x .git/hooks/pre-commit .git/hooks/pre-push
	@echo "Hooks installed: pre-commit + pre-push"

hooks-uninstall:
	@echo "Removing git hooks..."
	@rm -f .git/hooks/pre-commit .git/hooks/pre-push
	@echo "Hooks removed"

pre-commit: fmt-check
	@cargo check --workspace
	@echo "Pre-commit passed"

pre-push: ci

# ============================================================================
# Cleanup
# ============================================================================

clean:
	@cargo clean
	@rm -rf .loctree
	@echo "Cleaned"

# ============================================================================
# Help
# ============================================================================

help:
	@echo "mudd.one - Veterinary Ultrasound Processing"
	@echo ""
	@echo "Build:"
	@echo "  make build           Build debug (workspace)"
	@echo "  make release         Build release (workspace)"
	@echo ""
	@echo "Quality:"
	@echo "  make fmt             Format all code"
	@echo "  make fmt-check       Check formatting (no changes)"
	@echo "  make lint            Format check + clippy -D warnings"
	@echo "  make check           Full quality gate (fmt + clippy)"
	@echo "  make test            Run all tests"
	@echo "  make test-quick      Run lib tests only (fast)"
	@echo "  make ci              Full CI: fmt-check + lint + test"
	@echo "  make fix             Auto-fix clippy + format"
	@echo ""
	@echo "Version:"
	@echo "  make version         Show current version"
	@echo "  make bump-patch      Bump patch (0.1.0 -> 0.1.1)"
	@echo "  make bump-minor      Bump minor (0.1.0 -> 0.2.0)"
	@echo "  make bump-major      Bump major (0.1.0 -> 1.0.0)"
	@echo ""
	@echo "Hooks:"
	@echo "  make hooks-install   Install pre-commit + pre-push hooks"
	@echo "  make hooks-uninstall Remove hooks"
	@echo ""
	@echo "App:"
	@echo "  make bindings        Build FFI + generate Swift bindings"
	@echo "  make xcode           Regenerate Xcode project (xcodegen)"
	@echo "  make app             Full app build (bindings + xcode + build)"
	@echo ""
	@echo "Other:"
	@echo "  make clean           cargo clean + remove caches"
