<div align="center">
  <img src="assets/logo.png" width="144" height="144" alt="Discard logo">
  <h1>Discard</h1>
  <p><strong>A focused desktop home for Discord Web.</strong></p>
  <p>A native Windows shell built with Rust, Win32, and Microsoft Edge WebView2.</p>
  <p>
    <a href="https://github.com/Zvenytoh/discard/actions/workflows/windows.yml"><img src="https://github.com/Zvenytoh/discard/actions/workflows/windows.yml/badge.svg?branch=main" alt="Windows build status"></a>
    <img src="https://img.shields.io/badge/Windows-10%20%2F%2011-0078D4" alt="Windows 10 and 11">
    <img src="https://img.shields.io/badge/Built_with-Rust-DEA584" alt="Built with Rust">
    <img src="https://img.shields.io/badge/Engine-WebView2-FF573D" alt="Microsoft Edge WebView2">
  </p>
  <p>
    <a href="#overview">Overview</a> ·
    <a href="#getting-started">Getting started</a> ·
    <a href="#development">Development</a> ·
    <a href="https://github.com/Zvenytoh/discard/issues">Report an issue</a>
  </p>
</div>

---

## Overview

Discard brings the official Discord web experience into a dedicated native
Windows window. Rust owns the application lifecycle, Win32 provides the desktop
window, and WebView2 renders the site using the installed Evergreen Runtime.

The host uses an event-driven message loop and shares the system WebView2
installation instead of bundling another browser engine. The goal is a small,
maintainable desktop shell; memory and CPU improvements have not yet been measured.

### What is included

- **Native window** — standard Windows controls, resizing, and per-monitor DPI handling.
- **Persistent sign-in** — a dedicated local WebView2 profile reused between launches.
- **Focused navigation** — main-frame navigation restricted to HTTPS on `discord.com`.
- **Isolated web content** — native host objects and web-to-native messages disabled.
- **Explicit lifecycle** — asynchronous WebView2 startup and controller cleanup on exit.
- **Windows CI** — formatting, Clippy, and a release build checked by GitHub Actions.

### Project status

**Early development · v0.1.0.** The native host foundation is in place. Discord
provides the accounts, conversations, and media through its website.

External links, pop-up windows, and downloads are currently blocked. Voice,
camera, notifications, and screen sharing still need runtime validation. An
installer, system tray integration, and automatic updates are not included yet.

## Getting started

### Requirements

- **Windows 10 or 11, x64.**
- **[Microsoft Edge WebView2 Evergreen Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).**
- **[Rust stable](https://rustup.rs/)** with the `rustfmt` and `clippy` components.
- **Visual Studio Build Tools** with **Desktop development with C++** and a Windows
  SDK for the recommended MSVC toolchain.

### Build from source

```powershell
git clone https://github.com/Zvenytoh/discard.git
cd discard
cargo run --locked
```

For an optimized executable:

```powershell
cargo build --release --locked
.\target\release\discard.exe
```

Successful [Windows workflow runs](https://github.com/Zvenytoh/discard/actions/workflows/windows.yml)
also provide a `discard-windows-x64` build artifact. GitHub sign-in may be required
to download it. These are development builds, not installer releases.

### Session data

Discard stores its WebView2 profile in:

```text
%LOCALAPPDATA%\Discard\WebView2
```

Closing the window exits the application and keeps your session data. Treat this
directory as private: it can contain cookies and other account session information.

## Development

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

The PowerShell helper provides the same checks and handles copying the WebView2
loader for x64 GNU builds:

```powershell
.\scripts\dev.ps1 check
.\scripts\dev.ps1 build
.\scripts\dev.ps1 run
```

MSVC builds produce `target\release\discard.exe`. GNU builds also require
`WebView2Loader.dll` beside the executable. Unit and functional tests are run only
when explicitly requested; the current CI performs static checks and compilation.

## Architecture

The design follows a pure-policy core with native adapters: URL rules remain
independent of Win32 and WebView2, while the host owns platform effects and
resource lifetimes. The current implementation uses one binary crate.

Current source layout:

| Component | Responsibility |
| :--- | :--- |
| `src/main.rs` | Application entry point and fatal errors |
| `src/navigation.rs` | HTTPS URL allowlist |
| `src/native/mod.rs` | Win32 window, COM apartment, message loop, and shutdown |
| `src/native/webview.rs` | WebView2 initialization and navigation events |

See [Architecture](docs/ARCHITECTURE.md) for ownership and security boundaries,
[Tooling](docs/TOOLING.md) for the development tools and skills, and
[Project instructions](AGENTS.md) for contribution rules. The proposed next module
boundaries and their rationale are documented in the
[Clean architecture design](docs/superpowers/specs/2026-10-19-clean-architecture-design.md).

---

Discard is an independent project and is not affiliated with or endorsed by
Discord. Discord and Microsoft Edge WebView2 belong to their respective owners.
