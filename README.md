<div align="center">
  <img src="assets/logo.png" width="144" height="144" alt="Discard logo">
  <h1>Discard</h1>
  <p><strong>A focused desktop home for Discord Web.</strong></p>
  <p>Built with Rust and Tauri. Powered by your platform's webview.</p>
  <p>
    <a href="https://github.com/Zvenytoh/discard/actions/workflows/desktop.yml"><img src="https://github.com/Zvenytoh/discard/actions/workflows/desktop.yml/badge.svg?branch=main" alt="Desktop build status"></a>
    <img src="https://img.shields.io/badge/Built_with-Rust-DEA584" alt="Built with Rust">
    <img src="https://img.shields.io/badge/Desktop-Tauri_2-24C8D8" alt="Tauri 2">
  </p>
  <p><a href="#getting-started">Getting started</a> · <a href="#development">Development</a> · <a href="https://github.com/Zvenytoh/discard/issues">Report an issue</a></p>
</div>

---

## Overview

Discard opens the official Discord website in one dedicated native window.
The application uses a single Rust crate and Tauri's desktop lifecycle, without
React, npm, a local frontend server, or a bundled Chromium distribution.
Discord provides accounts, conversations, and media through its website.

Download published packages from the [latest release](https://github.com/Zvenytoh/discard/releases/latest).
The v0.2.x line uses Tauri; v0.1.0 was the earlier Windows-only Win32 host.
Platform support and runtime validation are tracked separately below.

| Candidate target | Engine | Packaging | Validation status |
| :--- | :--- | :--- | :--- |
| Windows 10/11 x64 | WebView2 Evergreen | Per-user Inno installer, portable EXE | Migration acceptance pending |
| macOS 14+ Intel and Apple Silicon | WKWebView | Separate DMGs | Experimental; native runtime checks pending |
| Ubuntu 22.04+ x64 | WebKitGTK 4.1 | DEB, AppImage | Experimental; native runtime checks pending |

Other Linux distributions are best effort. An AppImage does not supply every
system codec or guarantee compatibility with every graphics stack.

### What the candidate includes

- One native window with the coral Discard icon, dark appearance, and persistent
  engine data. Closing the window exits the app, including on macOS.
- Main-frame navigation restricted to HTTPS on the exact `discord.com` host,
  port 443, without embedded credentials. Pop-ups and downloads are blocked.
- No application-native commands or capability grants for remote
  content. Tauri still uses framework-internal scripts and IPC.
- Event-driven Windows background handling: hide rendering when minimized and
  request lower memory use while inactive, with normal memory priority for
  playing audio and conservative fallbacks. The page is never forcibly suspended.
- Native CI configuration for all four architecture targets, with formatting,
  Clippy, release builds, and packaging.

Voice, silent calls, video, notifications, screen sharing, session upgrades, and
installation behavior need the checks recorded in the
[compatibility report](docs/validation/tauri-compatibility.md). No CPU or memory
saving is claimed before the
[performance measurements](docs/validation/tauri-performance.md).
### App updates

Starting with v0.2.1, the installed Windows application checks once at startup
for a newer release. Choose **Install update** to download, verify and install it,
or **Later** to keep using the current version. Discard restarts after installation.
Offline checks do not prevent startup; there is no periodic background polling.
The installer signature and its signed version must match the embedded public key
and release manifest. The existing installation directory and Discord profile are preserved.

Install v0.2.1 manually once to enable this feature: older releases cannot update
themselves. Portable Windows builds, macOS and Linux still update manually.
Windows updater signatures authenticate releases but are not Authenticode certificates.
Mobile and system tray integration remain outside this release.

## Getting started

Download **Discard-Setup.exe** from the
[latest release](https://github.com/Zvenytoh/discard/releases/latest) for Windows.
Run the installer as your normal user. Other packages are listed on the release
page when available. Development builds are also available from successful
[Desktop workflow runs](https://github.com/Zvenytoh/discard/actions/workflows/desktop.yml);
GitHub sign-in may be required for workflow artifacts.

| CI artifact | Candidate files |
| :--- | :--- |
| `discard-windows-x64` | `Discard-Setup.exe`, `Discard.exe`, `SHA256SUMS.txt` |
| `discard-linux-x64` | `Discard_0.2.1_linux-x64.deb`, `Discard_0.2.1_linux-x64.AppImage` |
| `discard-macos-x64` | `Discard_0.2.1_macos-x64[-unsigned-experimental].dmg` |
| `discard-macos-arm64` | `Discard_0.2.1_macos-arm64[-unsigned-experimental].dmg` |

The bracketed macOS suffix is present when signing credentials are unavailable.
No signing certificate is currently configured for this project. Windows binaries
are unsigned; macOS unsigned/ad-hoc packages are experimental CI artifacts and
are excluded from public release assets. Normal macOS distribution requires
Developer ID signing and notarization.

### Windows installation and session data

Setup installs into `%LOCALAPPDATA%\Programs\Discard` without administrator rights,
with Start menu and optional desktop shortcuts. It retains the original Inno app
identity for upgrades. If WebView2 is missing, setup installs Microsoft's signed
Evergreen bootstrapper; that step needs an Internet connection. The portable
executable requires an existing WebView2 runtime.

The candidate explicitly reuses `%LOCALAPPDATA%\Discard\WebView2`. It does not
copy or reset cookies. Actual v0.1.0-to-candidate sign-in preservation must still
be checked with a user-controlled session. Uninstall leaves this profile intact.
Do not run two versions against the profile simultaneously.

Linux stores webview data under Tauri's app-local data directory for
`com.zvenytoh.discard`, in `webview` (normally
`$XDG_DATA_HOME/com.zvenytoh.discard/webview` or
`~/.local/share/com.zvenytoh.discard/webview`). macOS uses a fixed persistent
WKWebView data-store identifier. There is no cross-platform cookie migration.
Treat engine data as private account/session information.

## Development

### Requirements

All targets need Rust stable with `rustfmt` and `clippy`. The reviewed versions
are Tauri **2.12.4**, tauri-build **2.7.3**, and tauri-cli **2.12.3**; the resolved
Rust dependencies are committed in `Cargo.lock`. Node and npm are unnecessary.

- **Windows:** MSVC Rust, Visual Studio C++ Build Tools with Windows SDK, and
  WebView2 Evergreen. MSVC is the shipped target.
- **macOS:** macOS 14+ and Xcode command-line tools; build for the host architecture.
- **Linux:** Ubuntu 22.04+ build tools and WebKitGTK 4.1 development packages.

On Ubuntu:

```sh
sudo apt-get update
sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf libfuse2
```

Clone and install the pinned Cargo CLI:

```sh
git clone https://github.com/Zvenytoh/discard.git
cd discard
cargo install tauri-cli --version 2.12.3 --locked
```

Use `./scripts/dev.ps1 run` on Windows or `bash scripts/dev.sh run` on macOS/Linux.
The helpers provide `check`, `fmt`, `clippy`, `build`, and `run` tasks and invoke
Cargo from the repository root. `build` produces the optimized application
without an installer. The Windows helper also copies the WebView2 loader for
local x64 GNU builds; MSVC remains the distribution baseline.

Required Rust checks:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

Unit and functional tests run only on explicit request. CI performs the static
checks and compilation above; compilation does not establish login or media support.

### Packaging and releases

Windows retains Inno Setup 6.3+:

```powershell
.\scripts\dev.ps1 build
.\scripts\build-installer.ps1
```

The script verifies the Microsoft bootstrapper's Authenticode signature and
writes packages to `target/distribution`. MSVC links the C runtime statically.
On Linux use `cargo tauri build --bundles deb,appimage -- --locked`; on macOS
use `cargo tauri build --bundles dmg -- --locked`. Native SDKs and packaging
tools are required; macOS release signing also requires private CI credentials.

The Desktop workflow builds branch/PR artifacts for Windows x64, Linux x64,
macOS x64, and macOS arm64. Only a matching `v<package-version>` tag enables its
publication job, after every build succeeds. That job normalizes filenames,
excludes unsigned macOS packages, and generates `SHA256SUMS.txt` across public
assets. A branch run never publishes a release. Review the compatibility report
and candidate artifacts before creating a release tag.

## Architecture

| Component | Responsibility |
| :--- | :--- |
| `src/main.rs` | Entry point and final failure reporting |
| `src/app.rs` | Single Tauri window, web hooks, and exit lifecycle |
| `src/navigation.rs` | Pure HTTPS URL allowlist |
| `src/profile.rs` | Persistent platform data locations |
| `src/platform/` | Native appearance, identity, and engine-specific background handling |

See [Architecture](docs/ARCHITECTURE.md), [Tooling](docs/TOOLING.md), and
[Project instructions](AGENTS.md). The
[Tauri design](docs/superpowers/specs/2026-10-20-tauri-multiplatform-design.md)
supersedes the earlier Windows-only module proposal.

---

Discard is independent and is not affiliated with or endorsed by Discord.
Discord, Tauri, and the platform web engines belong to their respective owners.
