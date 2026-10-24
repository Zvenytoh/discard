# Architecture

Discard is a single Rust binary crate hosting the official Discord website in
one Tauri window. The v0.2.0 source replaces the old direct Win32 event loop;
it remains a migration candidate until platform acceptance checks are recorded.

The [Tauri migration design](superpowers/specs/2026-10-20-tauri-multiplatform-design.md)
supersedes the Windows-only native module extraction proposed in the
[earlier clean architecture design](superpowers/specs/2026-10-19-clean-architecture-design.md).
That historical proposal does not describe the current module layout.

## Dependency boundaries

The dependency rule applies to real host policy: `navigation.rs` depends only on
the URL parser. It imports no Tauri, OS, COM, filesystem or presentation types.
Discord itself owns accounts, conversations and media; duplicating its domain in
local service/repository layers would add no useful boundary.

| Module or file | Responsibility |
| :--- | :--- |
| `src/main.rs` | Entry point, exit status and final error reporting |
| `src/app.rs` | Tauri setup, one external window, security hooks and exit policy |
| `src/navigation.rs` | Home URL and pure main-frame allowlist |
| `src/updater.rs` | Windows installed-app startup check, native consent and verified Inno handoff |
| `src/tray.rs` | Native tray, mod recovery switches and application-audio selection |
| `src/mods.rs`, `mods.js` | Pinned browser Vencord bootstrap and sanitized activity adapter |
| `src/platform/activity.rs` | Windows foreground-event game matching and process-exit notifications |
| `src/platform/rpc.rs` | Bounded local named-pipe Rich Presence receiver |
| `src/platform/audio.rs`, `audio.js`, `audio-worklet.js` | Experimental native-selected WASAPI audio and bounded binary transport |
| `src/profile.rs` | Explicit persistent engine-data locations |
| `src/platform/mod.rs` | Narrow platform configuration/attachment dispatch and native error presentation |
| `src/platform/titlebar.rs`, `titlebar.js` | Windows frame, accessible native caption buttons and Discord header layout |
| `src/platform/windows.rs` | Windows identity, DWM appearance and event-driven WebView2 resource policy |
| `src/platform/macos.rs` | WKWebView persistent store and background scheduling configuration |
| `build.rs`, `tauri.conf.json` | Tauri build context, identity, icons and bundle metadata |

```mermaid
flowchart LR
    Main[main.rs] --> App[app.rs]
    App --> Policy[navigation.rs]
    App --> Profile[profile.rs]
    App --> Platform[platform adapters]
    App --> Tauri[Tauri lifecycle]
    Platform --> Native[OS and engine APIs]
    Policy --> URL[url parser]
```

There is no empty Linux adapter: Linux uses the supported Tauri engine defaults.
There is no browser trait, workspace of extra crates, frontend tree or global
service registry. Add a module only when it owns a concrete responsibility.

## Lifecycle and ownership

`app::run() -> tauri::Result<()>` configures Tauri and creates `main` from Rust
setup. It loads `https://discord.com/app` directly at 1200×800 logical pixels,
with native decorations, a dark theme and opaque `#242429` background.
`tauri.conf.json` declares no initial windows; it grants no remote capabilities.
No frontend development server is involved. Vencord and the native activity/audio adapters now run in the Discord document; this is an explicit extension of the original wrapper-only scope.

`profile::directory(&tauri::AppHandle)` returns the platform's explicit data
location before the window is built. `platform::configure(builder)` applies
builder-only settings, and `platform::attach(&window)` adds native behavior to the
constructed window. Native state stays private to the adapter. Closing `main`
hides it only after a tray icon has been created successfully. The tray owns an
Open/Quit menu; restoring shows, unminimizes and focuses the same window. Quit
and updater handoffs still exit the process directly. Failed tray creation falls
back to close-to-exit, preventing an inaccessible hidden application. On Windows,
hide and restore explicitly refresh the event-driven renderer visibility policy.

Tauri/Wry owns the platform event loop and engine lifecycle. Windows COM access
must remain on the owning UI STA thread through Tauri's native webview callback.
Do not retain borrowed engine pointers, hold an interior borrow across reentrant
native operations, or introduce callback ownership cycles. Optional resource
optimizations must not turn into fatal startup failures.

## Remote-content boundary

Windows Setup creates a per-user Startup shortcut by default, with a visible
opt-out task. Setup remembers prior task choices; deselection removes only this
shortcut. Uninstall removes it as well. Portable builds do not register startup.

- Parse main-frame URLs with `url`: require HTTPS, exact `discord.com`, effective
  port 443, and empty username/password. Prefix matching is insufficient.
- Deny new webviews and requested downloads. On Windows, user-initiated HTTP(S)
  popups and external main-frame links open in the system browser. Parse the URL,
  require a host and no credentials, and pass it as a shell target without command
  arguments. Other schemes, automatic popups and external redirects stay blocked.
  Main-frame links also remain canceled inside the webview. No page IPC is added.
- Main-frame navigation policy is not a subresource firewall. Discord's CDN and
  engine-managed requests retain normal browser security behavior.
- Configure no application invoke handler, privileged remote capability grants
  to Discord. Updater and dialog plugins are called only from Rust on Windows,
  with no remote capability grants. `withGlobalTauri` is false. Tauri still supplies framework-internal
  scripts and IPC; absence of a global object alone is not isolation proof.
- Keep TLS, origin checks and sandboxing intact. Explicitly return the engine's
  default permission response; this requests WKWebView's origin prompt instead
  of Wry's no-handler media auto-grant. Other engines keep their default decision.
  macOS bundles declare microphone/camera purposes and appropriate capture
  entitlements; declarations do not grant permission or establish media support.

The [compatibility report](validation/tauri-compatibility.md) separates source/ACL
review from runtime probes, login, notifications and capture behavior.

## Persistent data and upgrades

| Platform | Storage contract |
| :--- | :--- |
| Windows | Explicit `%LOCALAPPDATA%\Discard\WebView2`, unchanged from v0.1.0 |
| Linux | Tauri app-local data directory for `com.zvenytoh.discard`, plus `webview` |
| macOS | WKWebView store ID `[213,50,140,206,145,20,70,192,190,120,110,183,97,61,185,142]` |

Keep bundle identifier `com.zvenytoh.discard` stable. Never read, export, clear,
rename or copy cookies during migration. Handle directory/window errors inside
the Tauri setup callback, report them and request exit; returning setup Err would
cause Tauri to panic. Windows presents a dialog, other platforms currently log
to stderr. Never silently create an alternative profile after a failure.
Windows uses the existing location, but sign-in preservation across the upgrade
still needs a user-controlled acceptance check. Profiles are not shared across OSes.

Inno Setup retains AppId `{D5328CCE-9114-46C0-BE78-6EB7613DB98E}` and destination
`%LOCALAPPDATA%\Programs\Discard`. The app and shortcuts use AppUserModelID
`Zvenytoh.Discard`. Uninstall preserves the separate profile. No periodic update polling is included. Since v0.2.1, installed Windows builds check
once at startup through the Rust updater plugin and ask before downloading.
The HTTPS GitHub feed may only point to this repository's versioned installer URL.
The plugin verifies Minisign and the signed version before returning installer bytes.
The host writes a unique temporary executable, launches Inno with the existing
install directory and `/DISCARDUPDATE=1`, then exits. Inno restarts Discard after
a successful silent install. Temporary installer files are left to Windows temp cleanup.
Portable Windows, Linux and macOS builds use manual updates. Signing credentials
are stored in GitHub Actions secrets, never in source; only the public key is embedded.

## Appearance and background policy

The approved Blue D vector artwork supplies native executable/bundle, taskbar/Dock,
shortcut and installer icons. Windows retains Discord's actual 32-pixel top bar:
the live title, SVG icons, inbox/help actions and their native page interactions.
Static CSS centers the title and reserves 132 logical pixels at the right for
three native owner-drawn BUTTON controls, each 44 by 32 logical pixels. Buttons
retain Windows input/capture/accessibility semantics, with hover, pressed,
keyboard-focus and red-close feedback. System commands minimize, maximize/restore
and close to tray; restoring focus after maximize returns typing to Discord.

Windows disables Tauri decorations and shadows. The Wry host has a native window
region excluding the controls, updated on resize, so Chromium cannot cover them. WebView2 Settings9 enables CSS app-region dragging before
the first Discord navigation. Windows first creates an empty browser; native
security/drag settings are applied before Navigate(HOME). Discord receives no
native command bridge, web messages or host objects. Other platforms navigate
directly to HOME and keep native decorations.

The one-shot document initialization installs static layout CSS. No observer,
recurring script, synthetic inbox click or additional WebView is used. Native
children are owned/destroyed by their parent; hooks remain on the UI thread and
are removed at destruction. Brushes/pens/DCs are released synchronously. Failed
control setup errors are logged. Discord class-prefix changes may need
CSS updates. Visual fidelity, mixed DPI, zoom, dragging and real input remain
subject to runtime acceptance.

Windows resource adjustments are event-driven: minimized or tray-hidden rendering is hidden,
then restored with the window. Memory target remains NORMAL, including while
inactive or silent: playback silence cannot establish that no outgoing call or
stream is active. Automatic LOW targeting and its playback observer are removed.
The engine defaults remain in place if the optional memory interface is unavailable.
The memory target is a hint, not a private-memory limit. Document-audio silence
cannot establish that no microphone/video call exists.

macOS disables supported WKWebView background throttling to favor ongoing
messages/calls, which can cost background resources. Linux retains engine defaults.
No target gets forced suspension, timer polling, injected keepalives, automatic
reloads, cache purges or GPU-disable flags. Preserve the release LTO, single
codegen unit, stripping and panic-abort settings until measurements justify changes.

Actual CPU, private memory and working set must be measured separately for only
Discard's process tree. Compile success and a smaller working set do not prove
lower allocation, better responsiveness or reliable calls. See the
[performance report](validation/tauri-performance.md).

## Build and release boundary

Runtime remains one crate; packaging belongs to scripts and native CI.
Tauri 2.12.4 and tauri-build 2.7.3 are pinned. Tauri CLI 2.12.3 is installed from
a checksum-verified upstream archive, with dependencies resolved in `Cargo.lock`.
No Node toolchain is required.

The Desktop release workflow targets Windows MSVC x64 only. Windows retains Inno
and its Microsoft-signed WebView2 bootstrapper verification. Other target modules
remain in source but are not packaged by the current workflow. Branch/PR builds
run formatting and Clippy. Tags and manual runs use one Tauri
production build and package Windows artifacts. Cargo dependencies and build
artifacts are cached by OS, target, compiler fingerprint and lockfile; fallback
can reuse older lockfiles for the same compiler/target, with Cargo checking
dependency fingerprints. Release signing remains tag-only.

A matching version tag permits publication after the Windows build succeeds.
It signs the updater installer, creates `latest.json`, and computes public asset
checksums. The updater signing key belongs only in CI. The executable is not
Authenticode-signed. CI compilation does not establish engine/media compatibility.

## Rust and validation conventions

Keep application `unsafe` inside `src/platform/`, with local safety comments on
thread affinity, ownership, pointer validity and allocation. Deny unsafe operations
inside unsafe functions without explicit blocks. Favor `Result` and `?`; avoid
panic paths in callbacks. Preserve original error sources until final reporting.

Follow `AGENTS.md`: formatting, Clippy and release compilation for Rust changes;
unit/functional runs only when explicitly requested. Runtime, account, media and
installation cases are independent checks. Record unperformed checks rather than
presenting all desktop targets as validated support.

The restored Windows window disables native decorations and shadows. A native region reserves the caption controls independently of Chromium rendering order.

## Native integration (v0.3.0)

Vencord initializes at document start only in the main HTTPS discord.com frame.
The official browser bundle and CSS are embedded, with source and hashes pinned
in vendor/vencord. The browser-native stub receives local CSS before initStyles.
Preferences use presence markers in Tauri's app config directory, separately
from the existing WebView2 profile. A tray toggle writes the marker and restarts.

Windows game detection uses one WinEvent/message-wait thread. It opens only the
foreground process for image matching, keeps the game until its process handle
signals exit, and blocks on events while idle. It does not detect never-focused
background games. The filtered catalog excludes launchers and argument-dependent
or ambiguous executable matches. Activity updates go from Rust to the document;
no process paths or process enumeration API are exposed to remote content.

RPC uses discord-ipc-0 through discord-ipc-9 only when no existing owner holds a
name. Local clients are bounded to eight, 64 KiB frames and 60 frames/second per
connection. Idle established connections have no read deadline; after the first
byte, a frame must complete within five seconds. Responses have the same write
deadline, with outer service cancellation covering both directions. Identical
activities do not trigger page updates, and queued publications read current state
on the UI thread so an older snapshot cannot overwrite a later one.
It accepts handshake, ping and SET_ACTIVITY only; no shell, invite,
filesystem, credential or capture command is implemented. Disconnect and page
reload update the Vencord adapter, with serialized metadata lookups.

Audio is armed by a native tray choice, never by a page-provided PID. Selection
records process creation time and excludes Discard and its ancestors. Only then
is an ephemeral loopback listener created. Handshakes require a fresh 256-bit
path token and exact Discord Origin. The service is read-only: no inbound page
commands start arbitrary native operations. All scripts in the Discord origin
share this trust boundary; the token does not isolate Vencord from Discord code.

A connected share initializes WASAPI process-loopback on a worker MTA, verifies
PID identity and captures 48 kHz stereo float PCM. It blocks on audio, target-exit
and cancellation handles. A four-packet channel limits native delivery. Each
1024-frame packet has an eight-byte little-endian f64 capture timestamp in epoch
milliseconds, correlated from WASAPI QPC; timestamp errors fall back to receipt
time. DATA_DISCONTINUITY clears partial queued samples. The server rejects packets
queued longer than 250 ms and bounds every write, including errors and intentional
close notifications. No privileged inbound audio command is added.

The optional audio wrapper honors the per-share audio request and active Vencord
mute preference. An explicitly armed share creates one disposable blob Worker and
transfers the AudioWorklet MessagePort to it. Socket reception, timestamp age checks
and bounded delivery no longer pass through Discord's page main thread. The Worker
rejects capture data older than 250 ms and retains one unacknowledged packet plus
only the newest pending packet. The Worker carries capture time and packet age
into the worklet, which retains its bounded stereo ring. A 40 ms capture-to-playout
target absorbs packet jitter; it is a scheduling policy, not measured total latency.
Gaps above 5 ms reset the timeline, older out-of-order packets are ignored, and
large late offsets discard obsolete samples. A source-rate estimate over intervals
of at least one second feeds fractional linear interpolation capped at +/-0.5%.
Small clock offsets use resampling, not repeated silent block prefixes. No new
main-thread sampling, polling, browser capture permission or dependency is added.
The 250 ms packet age threshold remains a delivery policy, not an end-to-end bound.

Startup has one shared ten-second deadline and preserves live browser capture on
failure. Successful native audio replaces the browser audio track. Unexpected
post-start transport/processor failure warns once and keeps video alive; the
stopped browser track is not falsely claimed to recover automatically. A deliberate
native stop sends a normal close reason so it stays quiet. Stop, pagehide and
failure terminate the Worker, disconnect output and close the context. No Worker,
listener or WASAPI recording is created without an armed audio request.

No CSP, remote capability or host-object restriction is weakened. Worker/blob CSP
support, synchronization, clock drift and received media still require real-call
acceptance. Timestamped playout and bounded source/render clock compensation are
implemented, but video-frame timestamps are not available to this audio adapter.
They do not establish calibrated audio/video synchronization at the receiver.
See the AudioWorklet timing model: https://www.w3.org/TR/webaudio/#AudioWorkletGlobalScope

## Screen-share profile handling

`src/stream.js` is installed before the pinned Vencord bundle captures
getDisplayMedia. It receives the actual submitted picker dimensions/FPS, replacing
inherited capture limits without forcing a 16:9 source. Only successful live
capture publishes a profile; normal browser shares and disabled plugins bypass it.
A stopped share clears its active profile, leaving upstream pre-picker bounds.

`src/mods.js` guards the ApplicationStreamingSettingsStore integration and syncs
resolution/FPS/custom preset without changing sound preference or account features.
It updates only the local stream's videoStreamParameters and asks the connection
to refresh quality. The getGoliveMaxQuality adapter applies the live profile's
actual width, capped source height and selected FPS, with resolution/FPS-dependent
bitrate limits (target 2–24 Mbit/s, ceiling up to 32 Mbit/s, minimum 0.5 Mbit/s).
Dimensions are read from the live track at quality evaluation, including after a
source resize or switch. Bitrate scales with the actual encoded pixel area rather
than assuming every source is 16:9. Network/codec adaptation remains browser-managed.
This is not a fixed encoder rate.

Discord's own stream settings changes apply matching capture constraints through
an event listener, not polling. One application is active at a time and one
replaceable pending request retains the latest choice. A profile is committed
only after successful capture constraint application. Intermediate successes are
retained as rollback points but are not republished over a newer pending choice.
A rejection restores the last accepted Discord/picker profile without stopping
capture; completion from an ended/replaced track is ignored. State publications
suppress their own change listener to avoid feedback loops.

One deferred refresh and STREAM_CREATE handle connection startup. Private module
shapes are checked per share and on connection events. A not-yet-loaded store at
initial selection is retried after capture without a premature compatibility toast.
Replacing a store removes its old listener when supported. Unsupported shapes
produce a compatibility warning; browser video is not discarded. The UI reflects the requested profile,
not sampled receive-side FPS. The pinned upstream browser assets are unchanged;
Electron-only plugins do not gain compatibility from these adapters.

## Single-instance and tray lifecycle (v0.3.3)

The official single-instance plugin is registered before all other plugins and
before app setup, using the stable application identifier without semver-specific
locks. Duplicate arguments are ignored: reopening only restores the existing
window. The shared restoration action runs on the UI thread, shows/unminimizes,
refreshes renderer visibility and then requests focus; failure to take foreground
focus cannot leave the renderer logically hidden. No second profile is created.

Mod preference changes use request_restart instead of restart on the UI thread,
so ExitRequested/Exit events are delivered. RunEvent::Exit explicitly cancels
activity detection, local RPC and native audio. The plugin also releases its
instance lock on Exit, before Tauri launches the replacement process. Destroyed-window cleanup remains idempotent.
Older binaries that do not implement a single-instance lock must be closed once
when upgrading. Runtime rapid-launch/tray/restart acceptance remains pending.

## Media stability corrections (v0.3.4)

RPC and game-activity replay clone the current state under the mutex, then release
it before window.url/eval can dispatch to the UI thread. Page-load callbacks can
therefore read state without waiting on a worker that is itself waiting on the UI.

Native audio setup shares a monotonic 10-second deadline across worklet loading,
AudioContext.resume and the WebSocket handshake. Track-end cancellation also
rejects each pending stage. Timeout closes the optional audio path and returns
working browser video. No periodic timeout or background polling is introduced.

The automatic LOW memory target was removed rather than inferring media idleness
from audible playback. This favors call/stream stability and may increase memory
retention while the window is in the background. Renderer visibility handling,
engine security settings, profile paths and updater verification are preserved.
