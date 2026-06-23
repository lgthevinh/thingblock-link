# thingblock-link — build & dev helpers.
#
# Native dev is the common path (`make dev` / `make build`). Cross-compiling the
# GUI/tray deps (gtk, tray-icon) is target-dependent:
#   - Windows: works from Linux via `cargo-xwin` (used by `build-windows`).
#     Needs `rustup target add x86_64-pc-windows-msvc`, `cargo install cargo-xwin`,
#     and `lld` on PATH. cargo-xwin auto-downloads the MSVC CRT + Windows SDK.
#   - macOS: needs the Apple SDK (osxcross) — license-restricted; build on a Mac
#     or a macOS CI runner, not from here.
#   - Linux aarch64: needs an arm64 GTK/dbus cross sysroot.
# Building each on its native host (or a CI matrix) is the path of least friction.

CARGO ?= cargo
CARGO_XWIN ?= cargo xwin

# Release target triples, keyed to the bundled arduino-cli dirs.
TARGET_LINUX   := x86_64-unknown-linux-gnu
TARGET_MACOS   := aarch64-apple-darwin
TARGET_WINDOWS := x86_64-pc-windows-msvc

.DEFAULT_GOAL := help

.PHONY: help dev run build build-linux build-macos build-windows build-all \
        fmt fmt-check clippy test check clean

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

dev run: ## Run the helper locally (debug)
	$(CARGO) run

build: ## Native release build
	$(CARGO) build --release

build-linux: ## Release build for linux x86_64
	$(CARGO) build --release --target $(TARGET_LINUX)

build-macos: ## Release build for macOS arm64
	$(CARGO) build --release --target $(TARGET_MACOS)

build-windows: ## Release build for Windows x86_64 (cross via cargo-xwin)
	$(CARGO_XWIN) build --release --target $(TARGET_WINDOWS)

build-all: build-linux build-macos build-windows ## Release build for all platforms

fmt: ## Format the code
	$(CARGO) fmt

fmt-check: ## Check formatting (CI)
	$(CARGO) fmt --check

clippy: ## Lint with warnings denied
	$(CARGO) clippy --all-targets -- -D warnings

test: ## Run the test suite
	$(CARGO) test

check: fmt-check clippy test ## Run all pre-submit checks

clean: ## Remove build artifacts
	$(CARGO) clean
