# Native integration feasibility

Status: initial implementation in v0.3.0; browser/native runtime acceptance pending. Application audio is experimental, explicitly armed through the tray.

## Target

Keep Rust/Tauri and the existing Discord profile. Start on Windows x64. Add
Vencord, game activity and screen sharing with application audio. Other platforms
need their own native adapters; Windows success does not establish portability.

## Smallest useful implementation

1. Embed a pinned browser build of Vencord with its license and source provenance.
   Update it with Discard releases and provide a disable/restart recovery option.
   Do not download executable scripts on each launch.
2. Implement game detection and a minimal Discord RPC receiver in Rust. Process
   detection and receiving Rich Presence from participating games are separate
   features. Match known executables; do not label every process as a game.
   Publish sanitized activity through a small Vencord adapter. arRPC documents
   the protocol and existing browser integration, but its server requires Node;
   bundling that server would conflict with the chosen Rust-only host.
3. First validate existing WebView2 calls, camera and display capture in an actual
   Discord call. Do not replace functioning browser media capture unnecessarily.
4. If application audio is missing, prototype WASAPI process-loopback capture.
   Windows Graphics Capture is an option if native video capture is also needed.
   Capturing samples locally is not sufficient: the prototype must deliver a live
   browser MediaStreamTrack into Discord's existing outgoing stream. WebView2's
   screen-capture permission event does not provide that media transport.

## Critical feasibility gate

Before building a full capture subsystem, prove application audio reaches another
Discord participant, starts only after explicit selection, excludes incoming call
audio, and stops when sharing ends. Select the media transport only after this
experiment. A local WebRTC peer is a candidate, not a validated solution. Do not
send continuous raw video frames as JSON/base64 over Tauri IPC.

The Microsoft process-loopback sample requires Windows build 20348 or newer.
Detect availability; do not silently claim support on all Windows 10 machines or
substitute recording every application's audio.

## Boundaries

Keep lifecycle composition in app.rs and native operations in platform modules.
Introduce modules only when implementing the corresponding feature. The current
host disables WebMessages and remote capabilities. Do not broadly enable Tauri
IPC, shell, filesystem or arbitrary process queries for discord.com.

Prefer native-owned controls for capture selection and native-to-page activity
updates. If a bridge is necessary, separately design its exact methods, main-frame
origin checks, input limits, consent and teardown. Origin checks cannot distinguish
Vencord from other scripts executing in the same Discord document. A loopback
service is also a trust boundary and needs authentication and origin validation.

Existing voice and camera remain browser-managed. Show native settings only for
implemented functions; spoofing desktop detection does not implement Discord's
private modules or justify displaying nonfunctional desktop options.

## Validation before shipping

- Game start/exit and disabled detection; participating-game RPC and non-RPC games.
- Voice, camera, screen/window selection, cancellation, stop and tray lifecycle.
- Remote participant confirms shared video and selected application audio, without
  call echo; capture resources released on stop, navigation and application exit.
- Measure idle CPU/memory and streaming latency before claiming optimization.

Implementation: bundled Vencord, native foreground-game detection, local IPC Rich Presence, and the experimental WASAPI/binary-WebSocket/AudioWorklet track prototype are present. Existing browser video capture remains. No real call, UI or performance validation has been performed.

## Primary references

- [arRPC protocol and Vencord integration](https://github.com/OpenAsar/arrpc)
- [Windows Graphics Capture](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture)
- [Application loopback audio sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/)
- [WebView2 screen capture event](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2screencapturestartingeventargs)
- [Tauri capability boundaries](https://v2.tauri.app/security/capabilities/)
