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
| macOS 14+ Intel and Apple Silicon | WKWebView | Not built in current release workflow | Not packaged |
| Ubuntu 22.04+ x64 | WebKitGTK 4.1 | Not built in current release workflow | Not packaged |

Other Linux distributions are best effort. An AppImage does not supply every
system codec or guarantee compatibility with every graphics stack.

### What the candidate includes

- One native window with the coral Discard icon, dark appearance, and persistent
  engine data. Windows retains Discord's own header, SVG icons and actions,
  with a centered title and responsive native window controls. Closing the window hides it to the system tray. Use **Ouvrir
  Discard** to restore it or **Quitter** to exit completely.
- Main-frame navigation restricted to HTTPS on the exact `discord.com` host,
  port 443, without embedded credentials. Pop-ups and downloads are blocked.
- No application-native commands or capability grants for remote
  content. Tauri still uses framework-internal scripts and IPC.
- Event-driven Windows background handling: hide rendering when minimized or hidden and
  request lower memory use while inactive, with normal memory priority for
  playing audio and conservative fallbacks. The page is never forcibly suspended.
- Native release CI for Windows x64, with formatting, Clippy, production builds,
  the signed updater installer, and per-user Inno packaging.

Voice, silent calls, video, notifications, screen sharing, session upgrades, and
installation behavior need the checks recorded in the
[compatibility report](docs/validation/tauri-compatibility.md). No CPU or memory
saving is claimed before the
[performance measurements](docs/validation/tauri-performance.md).
### Vencord and native features (v0.3.0)

Vencord's browser build is bundled at a fixed upstream commit. Its settings and
plugins appear inside Discord. The tray menu can disable Vencord or game
activity and restart Discard without clearing your Discord profile.

On Windows, foreground games are matched against a bundled executable catalog;
activity is retained until the process exits. There is no periodic process scan.
A minimal local Discord IPC receiver also accepts Rich Presence from compatible
games. It implements handshake, ping and SET_ACTIVITY, not the full Discord SDK.
Existing Discord IPC pipe owners are not replaced.

**Application audio sharing is experimental and off until you select a source.**
Right-click the tray, open **Audio du stream (experimental)**, refresh the
application list if needed, and select the application whose sound you want.
Then share a screen/window normally in Discord. Video stays browser-managed;
WASAPI captures only the selected process tree and supplies a stereo audio track.
The video choice and audio source are separate: select the same application in
both places. Stop sharing to stop capture; **Desactiver l'audio natif** also
removes the selection. Without a source, normal Discord Web capture is unchanged.

Process audio requires Windows build 20348+ (including supported Windows 11
versions). Failure does not fall back to capturing all system audio. This path
still needs a real call acceptance check, including browser CSP/worklet support,
remote audio, echo, synchronization, stop, and tray behavior. No performance or
full official-client feature parity is claimed. Native settings remain in the
tray; unsupported official desktop settings are not artificially exposed.

Discard is distributed under GPL-3.0-or-later with bundled Vencord. Upstream
commit, asset hashes, licenses and source links are in [vendor/vencord](vendor/vencord).
The game catalog attribution is in [vendor/GAMES-SOURCE.txt](vendor/GAMES-SOURCE.txt).

### Stream quality diagnostics

The tray's **Diagnostic du stream** reports live local capture dimensions/FPS
and outgoing WebRTC encoding dimensions/FPS when exposed by the browser. It also
shows the encoder's quality limitation reason (for example bandwidth or CPU).
These values are separate from Discord's stream-profile label; they do not measure
what a remote participant receives. The diagnostic runs only when requested.

Discard corrects the pinned Vencord WebScreenShare module's 0.6 Mbit/s target to
6 Mbit/s, retaining its existing 8 Mbit/s ceiling and browser bandwidth adaptation.
This is a target, not a guaranteed bitrate or a forced resolution/FPS setting.

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
The tray uses the Discard icon and an Open/Quit menu. A left click restores the
window on supported desktops; Linux tray interaction depends on the desktop's
status-icon support. If tray creation fails, closing the window exits instead.
Calls and notifications while hidden still require runtime validation.
Mobile remains outside this release.

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
Other platform packages are not built by the current release workflow.

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

The Desktop workflow builds Windows x64 only. A matching
`v<package-version>` tag creates a signed updater manifest and publishes the
installer, portable executable, signature and `SHA256SUMS.txt`. A branch run
never publishes a release.

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
