SHELL := /bin/bash
.DEFAULT_GOAL := help

# ====== Nastavení ======
APP_NAME       := snapdash
BUNDLE_NAME    := Snapdash
RELEASE_BIN    := target/release/$(APP_NAME)

# Log soubor (macOS — ProjectDirs mapping pro ("dev", "snapdash", "Snapdash"))
LOG_DIR        := $(HOME)/Library/Application Support/dev.snapdash.Snapdash
LOG_FILE       := $(LOG_DIR)/debug.log

# RUST_LOG šablony
LOG_DEFAULT    := $(APP_NAME)=info,warn
LOG_DEBUG      := $(APP_NAME)=debug
LOG_TRACE      := $(APP_NAME)=trace
LOG_WS         := $(APP_NAME)::ha::ws=trace
LOG_STATUS     := $(APP_NAME)::status=info,$(APP_NAME)=warn

# .app bundle location (po `make bundle`)
APP_BUNDLE     := dist/$(BUNDLE_NAME).app
APP_BUNDLE_BIN := $(APP_BUNDLE)/Contents/MacOS/$(BUNDLE_NAME)

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  HOST_OS := mac
else ifeq ($(UNAME_S),Linux)
  HOST_OS := linux
else
  HOST_OS := windows
endif

DIST_DIR       := dist
MAC_BUNDLE     := $(DIST_DIR)/$(BUNDLE_NAME).app
MAC_BUNDLE_BIN := $(MAC_BUNDLE)/Contents/MacOS/$(BUNDLE_NAME)
LINUX_DIR      := $(DIST_DIR)/linux
LINUX_TARBALL  := $(DIST_DIR)/$(APP_NAME)-linux-x86_64.tar.gz
WIN_DIR        := $(DIST_DIR)/windows
WIN_ZIP        := $(DIST_DIR)/$(APP_NAME)-windows-x86_64.zip
WIN_EXE        := $(APP_NAME).exe

# ====== macOS ======
.PHONY: build-mac install-mac run-mac

build-mac:
	@./mac_build.sh
	@echo "📦 macOS bundle: $(MAC_BUNDLE)"

install-mac: build-mac
	@rm -rf /Applications/$(BUNDLE_NAME).app
	@cp -R "$(MAC_BUNDLE)" /Applications/
	@echo "✅ Installed: /Applications/$(BUNDLE_NAME).app"

run-mac: build-mac
	@RUST_LOG="$(LOG_DEBUG)" "$(MAC_BUNDLE_BIN)"
	
# ====== Linux ======
.PHONY: build-linux install-linux run-linux

build-linux:
	@cargo build --release
	@mkdir -p "$(LINUX_DIR)"
	@cp "$(RELEASE_BIN)" "$(LINUX_DIR)/$(APP_NAME)"
	@[ -f README.md ] && cp README.md "$(LINUX_DIR)/" || true
	@[ -f LICENSE ]   && cp LICENSE   "$(LINUX_DIR)/" || true
	@cd "$(DIST_DIR)" && tar -czf "$(notdir $(LINUX_TARBALL))" linux
	@echo "📦 Linux tarball: $(LINUX_TARBALL)"

install-linux: build-linux
	@mkdir -p "$(HOME)/.local/bin"
	@cp "$(RELEASE_BIN)" "$(HOME)/.local/bin/$(APP_NAME)"
	@echo "✅ Installed: $(HOME)/.local/bin/$(APP_NAME)"
	@echo "   Zkontroluj, že ~/.local/bin je v PATH."

run-linux:
	@RUST_LOG="$(LOG_DEBUG)" cargo run --release
	
# ====== Windows (native build Windows Git-Bash/MSYS) ======
.PHONY: build-windows install-windows run-windows

build-windows:
		@cargo build --release
		@mkdir -p "$(WIN_DIR)"
		@cp "target/release/$(WIN_EXE)" "$(WIN_DIR)/"
		@[ -f README.md ] && cp README.md "$(WIN_DIR)/" || true
		@[ -f LICENSE ]   && cp LICENSE   "$(WIN_DIR)/" || true
		@cd "$(DIST_DIR)" && (command -v zip >/dev/null 2>&1 \
		  && zip -r "$(notdir $(WIN_ZIP))" windows \
		  || (command -v 7z >/dev/null && 7z a "$(notdir $(WIN_ZIP))" windows))
		@echo "📦 Windows zip: $(WIN_ZIP)"

install-windows: build-windows
		@mkdir -p "$(USERPROFILE)/bin" 2>/dev/null || mkdir -p "$(HOME)/bin"
		@cp "target/release/$(WIN_EXE)" "$(USERPROFILE)/bin/" 2>/dev/null \
		  || cp "target/release/$(WIN_EXE)" "$(HOME)/bin/"
		@echo "✅ Installed to %USERPROFILE%\\bin"

run-windows:
		@RUST_LOG="$(LOG_DEBUG)" cargo run --release

# ====== Host-aware shortcuty ======
# `make dist` → automaticky vybere správný build podle host OS
.PHONY: dist install-host run-host

dist: build-$(HOST_OS)
		@echo "🎯 Built for host: $(HOST_OS)"

install-host: install-$(HOST_OS)

run-host: run-$(HOST_OS)

.PHONY: help \
        check build build-release \
        fmt fmt-check clippy clippy-fix lint \
        test test-ws test-verbose docs \
        run run-debug run-trace run-ws run-status run-release run-bundle \
        log log-clear \
        bundle install-local \
        ci all pre-commit \
        clean distclean \
        bump-patch bump-minor bump-major \
        audit outdated

help:
	@echo "Snapdash — Makefile"
	@echo ""
	@echo "🦀 Build & check"
	@echo "  make check              - cargo check"
	@echo "  make build              - cargo build (dev)"
	@echo "  make build-release      - cargo build --release"
	@echo "  make docs               - cargo doc --no-deps --open"
	@echo ""
	@echo "🧹 Format & lint"
	@echo "  make fmt                - cargo fmt"
	@echo "  make fmt-check          - cargo fmt --check"
	@echo "  make clippy             - clippy --all-targets -- -D warnings"
	@echo "  make clippy-fix         - clippy --fix (!! code change)"
	@echo "  make lint               - fmt-check + clippy"
	@echo ""
	@echo "🧪 Test"
	@echo "  make test               - cargo test (all)"
	@echo "  make test-ws            - only ha::ws tests"
	@echo "  make test-verbose       - cargo test with stdout"
	@echo ""
	@echo "🏃 Run (dev build, env vars)"
	@echo "  make run                - default (snapdash=info,warn)"
	@echo "  make run-debug          - snapdash=debug"
	@echo "  make run-trace          - snapdash=trace (all)"
	@echo "  make run-ws             - snapdash::ha::ws=trace (per-frame WS)"
	@echo "  make run-status         - only status bar events"
	@echo "  make run-release        - optimized build"
	@echo "  make run-bundle         - run .app binary with debug logs"
	@echo ""
	@echo "📦 Distribuce (per platform)"
	@echo "  make build-mac          - .app bundle (macOS)"
	@echo "  make build-linux        - release binary + tarball (Linux)"
	@echo "  make build-windows      - release .exe + zip (Windows)"
	@echo "  make dist               - vybere podle host OS ($(HOST_OS))"
	@echo ""
	@echo "📥 Install"
	@echo "  make install-mac        - copy .app do /Applications"
	@echo "  make install-linux      - copy binary do ~/.local/bin"
	@echo "  make install-windows    - copy .exe do %USERPROFILE%\\bin"
	@echo "  make install-host       - vybere podle host OS"
	@echo ""
	@echo "🏃 Run platform builds"
	@echo "  make run-mac            - spustí .app binary (po build-mac)"
	@echo "  make run-linux          - cargo run --release"
	@echo "  make run-windows        - cargo run --release (.exe)"
	@echo "  make run-host           - vybere podle host OS"
	@echo ""
	@echo "📜 Log"
	@echo "  make log                - tail -f debug.log"
	@echo "  make log-clear          - delete debug.log"
	@echo ""
	@echo "📦 Distribuce (macOS)"
	@echo "  make bundle             - create dist/Snapdash.app"
	@echo "  make install-local      - copy .app to /Applications"
	@echo ""
	@echo "🎯 Meta"
	@echo "  make ci                 - lint + test"
	@echo "  make all                - fmt + lint + test + build-release"
	@echo "  make pre-commit         - check + lint + test"
	@echo ""
	@echo "🧹 Clean"
	@echo "  make clean              - cargo clean"
	@echo "  make distclean          - clean + delete Cargo.lock and dist/"

# ====== Build & check ======
check:
	@cargo check

build:
	@cargo build

build-release:
	@cargo build --release

docs:
	@cargo doc --no-deps --open

# ====== Format & lint ======
fmt:
	@cargo fmt -all

fmt-check:
	@cargo fmt --all --check

clippy:
	@cargo clippy --all-targets --all-features -- -D warnings

clippy-fix:
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features

lint: fmt-check clippy

# ====== Test ======
test:
	@cargo test

test-ws:
	@cargo test ha::ws

test-verbose:
	@cargo test -- --show-output --nocapture

# ====== Run (dev) ======
run:
	@RUST_LOG="$(LOG_DEFAULT)" cargo run

run-debug:
	@RUST_LOG="$(LOG_DEBUG)" cargo run

run-trace:
	@RUST_LOG="$(LOG_TRACE)" cargo run

run-ws:
	@RUST_LOG="$(LOG_WS)" cargo run

run-status:
	@RUST_LOG="$(LOG_STATUS)" cargo run

run-release:
	@cargo run --release

run-bundle:
	@if [ ! -x "$(APP_BUNDLE_BIN)" ]; then \
	  echo "❌ $(APP_BUNDLE_BIN) does not exists. Run 'make bundle' first."; \
	  exit 1; \
	fi
	@RUST_LOG="$(LOG_DEBUG)" "$(APP_BUNDLE_BIN)"

# ====== Log ======
log:
	@if [ ! -f "$(LOG_FILE)" ]; then \
	  echo "⚠️  Log does not exists: $(LOG_FILE)"; \
	  echo "    Run Snapdash once."; \
	  exit 1; \
	fi
	@tail -f "$(LOG_FILE)"

log-clear:
	@rm -f "$(LOG_FILE)"
	@echo "🧹 Log smazán: $(LOG_FILE)"

# ====== Bundle (macOS) ======
bundle:
	@./mac_build.sh
	@echo "📦 Bundle: $(APP_BUNDLE)"

install-local: bundle
	@rm -rf /Applications/$(BUNDLE_NAME).app
	@cp -R "$(APP_BUNDLE)" /Applications/
	@echo "✅ Installed: /Applications/$(BUNDLE_NAME).app"

# ====== Meta ======
ci: lint test

all: fmt lint test build-release
	@echo "✅ All passed."

pre-commit: check fmt-check clippy test
	@echo "✅ Pre-commit checks passed."

# ====== Clean ======
clean:
	@cargo clean
	@echo "🧹 target/ cleaned."

distclean: clean
	@rm -f Cargo.lock
	@rm -rf dist
	@echo "🧨 Cargo.lock a dist/ erased."
