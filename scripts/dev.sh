#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Install: cargo install tauri-cli --version 2.12.3 --locked
# Linux: WebKitGTK 4.1 development headers, GTK 3, build-essential, librsvg2-dev,
# libayatana-appindicator3-dev, patchelf (Ubuntu 22.04+). macOS: Xcode tools,
# Rust stable and macOS 14+. This helper does not install system packages.
task=${1:-check}
if [[ "$task" == build || "$task" == run ]]; then
    if [[ "$(cargo tauri --version)" != 'tauri-cli 2.12.3' ]]; then
        echo 'Install the pinned CLI: cargo install tauri-cli --version 2.12.3 --locked' >&2
        exit 1
    fi
fi
case "$task" in
    check)
        cargo fmt --all -- --check
        cargo clippy --locked --all-targets -- -D warnings
        ;;
    fmt) cargo fmt --all -- --check ;;
    clippy) cargo clippy --locked --all-targets -- -D warnings ;;
    build) cargo tauri build --no-bundle -- --locked ;;
    run) cargo tauri dev -- --locked ;;
    *) echo 'Usage: bash scripts/dev.sh [check|fmt|clippy|build|run]' >&2; exit 2 ;;
esac
