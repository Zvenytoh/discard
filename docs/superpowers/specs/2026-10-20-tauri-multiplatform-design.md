# Discard — Tauri desktop migration design

Status: proposed for review; no application migration implemented.

## Goal and scope

Move the existing Discord Web host from direct Win32/WebView2 integration to
Tauri 2 so the same Rust application can ship on Windows, macOS and Linux.
Keep Discord's official website, Discard's existing coral logo, a dark native
window and low background overhead while preserving live messages and calls.
This supersedes the Windows-only target for future implementation. Existing
AGENTS.md and architecture instructions remain descriptive of the current code
until the migration task updates them.

Initial release: 0.2.0. Desktop only; mobile, tray mode, automatic application
updates, Discord API reimplementation and additional app settings are excluded.

## Platforms and honest support

| Target | Engine | Initial package |
| --- | --- | --- |
| Windows 10/11 x64 | WebView2 Evergreen | Existing per-user Inno installer and portable executable |
| macOS 14+ Intel and Apple Silicon | WKWebView | Separate architecture-specific DMGs |
| Ubuntu 22.04+ x64 baseline | WebKitGTK 4.1 | DEB and AppImage |

Other Linux distributions are best effort. An AppImage is not a universal
browser runtime or a guarantee that every codec is present. The macOS 14 floor
enables the documented persistent data-store identifier and background policy.
Windows is the existing baseline; macOS/Linux are experimental until verified
on those systems. CI compilation alone does not establish Discord compatibility.

## Architecture decisions

- Keep one Rust binary crate at the repository root and the current package name.
- Use Tauri 2.12.4 or a later reviewed stable 2.x patch, with resolved versions
  committed in Cargo.lock. Use compatible tauri-build and tauri-cli 2.x versions;
  record their exact resolved versions in the implementation.
- Create one `main` WebviewWindow from Rust setup, loading
  `https://discord.com/app` directly. No React, Vite, npm, custom frontend server,
  iframe wrapper or secondary WebView is needed.
- Keep pure navigation decisions in `src/navigation.rs`; Tauri owns the native
  lifecycle, and small platform modules own actual engine-specific operations.
- Keep concrete interfaces and scoped visibility. No generic browser trait,
  repository layer, global service locator or additional crates in a workspace.
- Tauri's framework runtime may be required by its internals; add no independent
  async runtime, polling worker or periodic page script to application code.
- Preserve release LTO, one codegen unit, stripping and panic-abort initially.

## Remote-content boundary

Keep the existing exact HTTPS `discord.com`, effective port 443, no-credentials
navigation rule. Deny new windows and downloads. A main-frame allowlist remains
distinct from subresource requests: Discord's CDN requests must continue working.
Native protocol URLs, `file:`, `data:`, `javascript:`, localhost and lookalike
domains must not become allowed navigation destinations.

Configure no capabilities for the remote window, no remote capability grants,
no invoke handler, no application plugins, and `withGlobalTauri: false`.
Do not claim that this removes every framework-internal injected script or
transport: the acceptance criterion is that remote content cannot successfully
invoke privileged native operations. Review effective ACLs, not only globals.
Keep engine security defaults; do not weaken CSP, sandboxing, TLS or origin checks.
Retain normal user permission prompts for microphone/camera; add macOS usage
descriptions and required signed-app entitlements without automatic grants.
Notifications and screen sharing are separate per-engine acceptance cases.

## Profiles, installation and upgrades

Windows must explicitly reuse `%LOCALAPPDATA%\Discard\WebView2` through the
WebView builder's data directory. Never copy, clear, rename or inspect cookies.
Detect profile-in-use startup failures and present a useful error without
silently creating a fresh profile. Validate a v0.1.0-to-0.2.0 upgrade using a
user-controlled session before claiming persistent sign-in compatibility.

Use a persistent dedicated Linux data directory under Tauri's app-local data
directory. macOS uses the fixed data-store identifier bytes
`[213,50,140,206,145,20,70,192,190,120,110,183,97,61,185,142]`.
Set bundle identifier `com.zvenytoh.discard` and keep it stable thereafter.
Profiles are local to each platform; there is no cross-system cookie migration.

Retain the Windows Inno AppId `{D5328CCE-9114-46C0-BE78-6EB7613DB98E}`,
install path, executable name, shortcut targets and preserved user profile.
Do not introduce NSIS/MSI alongside it in this release. Windows AppUserModelID
is `Zvenytoh.Discard`, applied consistently to the app and Inno shortcuts.
Generate platform icon variants from `assets/logo.png`; keep the master artwork.

## Appearance and lifecycle

Start at 1200 by 800 logical pixels with native decorations and Discard title.
Use the existing coral Discard logo in executable, taskbar/Dock and packages.
Use a dark native theme and opaque `#1E1F22` background. On Windows 11 set caption
color `#1E1F22` and text `#F2F3F5`; Windows 10 uses supported native dark styling.
macOS/Linux retain native title bars; exact caption color is compositor-dependent.
Do not add a custom HTML title bar or grant native window commands to Discord.
Closing the only window exits the application on every supported desktop OS.

## Background behavior and measurement

Favor live messages/calls over page suspension. Add no forced suspension,
automatic reload, cache clearing, GPU-disable flag or JavaScript keepalive.
Use native event callbacks and avoid redundant state changes.

Windows: hide only the WebView rendering while minimized and restore it on
restore. Request LOW memory for inactive/minimized views, NORMAL for active views
or document audio. Audio-playing is an optimization hint, not proof of an active
or inactive call; validate silent microphone/video cases too. Optional-interface
failures must retain a usable application and conservative NORMAL memory mode.

macOS: use `BackgroundThrottlingPolicy::Disabled` on the supported OS floor to
favor ongoing Discord activity; document that this can cost background resources.
Linux: retain supported engine scheduling, with a mandatory long-background
compatibility check. Do not promise identical scheduling across engines. If
background calls/messages fail, mark the affected functionality unsupported;
do not hide the failure with injected keepalive scripts.

Before/after Windows measurements use identical content/profile/hardware and
sequential launches: 60 seconds warmup, then 60 seconds active idle, 60 seconds
inactive visible and 180 seconds minimized, repeated three times. Attribute only
the host and its engine process tree; report CPU, private memory and working set
separately. Also check a minimized session for at least ten minutes on every OS.
No numeric resource ceiling or improvement is promised before measurements.

## Delivery and validation

Use native CI runners for Windows x64, Linux x64 and each macOS architecture.
Build Linux on Ubuntu 22.04 to constrain the glibc baseline. Run formatting,
Clippy and locked release builds; unit/functional test execution remains subject
to the project's explicit-request rule. Runtime/manual measurements already
authorized for Windows remain in scope; other-platform login/media checks require
an available machine and user-controlled account actions.

On all engines check navigation denial, popups/download denial, native-IPC
denial, launch/close, relaunch session persistence, minimized startup, repeated
minimize/restore, clipboard and file drag/drop. Check login, permission denial,
messages, calls, silent calls, camera, notifications and screen sharing separately.
User initiates communications; automation never sends messages to other people.
Record blocked/unavailable cases instead of fabricating passes.

Use one release publication job after all packaging jobs succeed, exact version
and tag agreement, unique package names and SHA256SUMS.txt. Windows keeps
`Discard-Setup.exe` and `Discard.exe`. macOS packages require signing and
notarization credentials for normal public distribution; without credentials,
retain unsigned experimental CI artifacts and explicitly omit them from public
installable-release claims. Keep affected targets experimental when runtime
validation is unavailable. No release is published merely by writing this plan.

## References checked on 2026-10-20

- [Tauri engines](https://tauri.app/reference/webview-versions/)
- [Tauri Rust window API](https://docs.rs/tauri/2.12.4/tauri/webview/struct.WebviewWindowBuilder.html)
- [Capabilities](https://tauri.app/security/capabilities/)
- [CI packaging](https://tauri.app/distribute/pipelines/github/)
- [Linux baseline](https://tauri.app/distribute/appimage/)
- [macOS distribution](https://tauri.app/distribute/)

## Acceptance and review

Approve this direction and the linked implementation plan before migration code
is written. The plan must include early engine-compatibility checks, session and
installer compatibility, remote-content isolation, visual fixes, background
measurements and honest per-platform release status. New macOS/Linux support
must never be inferred solely from the existence of a downloadable package.
