# Stream audit — v0.3.7

Reviewed on 2026-10-20 at commit dfc93e8. This is a static audit of the complete
Discard-owned stream path and the pinned Vencord media code. It does not establish
received video quality, encoder throughput, audio synchronization or runtime behavior.
No application launch, account access, call, unit/functional suite or benchmark ran.

## Scope and data flow

- `src/app.rs`, `mods.rs`, `mods.js`: initialization order and Vencord integration.
- `vendor/vencord/browser.js`: actual shipped WebScreenShare/WebScreenShareFixes code,
  including capture options, quality limits, preview cleanup and audio mute handling.
- `src/platform/audio.rs`: native selection, process identity, authorization,
  WASAPI capture, packet delivery, cancellation and resource ownership.
- `src/platform/audio.js`, `audio-worklet.js`: browser integration, startup,
  buffering, rendering, error paths, track replacement and teardown.
- `src/platform/windows.rs`, `mod.rs`, `macos.rs`, `titlebar.js`, and tray/lifecycle
  callers: visibility policy, native controls and background behavior.
- README, architecture and acceptance records: documented guarantees and gaps.

Video: Discord/Vencord picker -> browser getDisplayMedia -> Discord WebRTC encoder
and transport. Discard does not implement a native video encoder or stream receiver.

Optional application audio: selected process tree -> WASAPI -> four-packet Rust
channel -> loopback WebSocket -> Discord page main thread -> acknowledged MessagePort
-> bounded AudioWorklet ring -> MediaStreamDestination track -> Discord WebRTC.
Without a native selection, the browser media path is returned unchanged.

## Findings requiring correction

### 1. P1 — Native audio bypasses the requested audio-off state

Locations: `src/platform/audio.js:8-11,75-77`; shipped Vencord code at
`vendor/vencord/browser.js:436`.

**Case:** Arm a native source, then request a video-only share (`audio: false`)
or select Vencord's mute option. The wrapper checks the endpoint and video tracks,
but never checks the audio request or Vencord's mute choice before adding native
sound. The shipped picker maps its `systemAudio` mute setting to `exclude` and
shows “Stream Muted”, while this wrapper still adds the native audio track.

**Impact:** The returned share can contain application audio despite an audio-off
request or muted picker summary. This is a consent/behavior defect, not an FPS issue.

**Smallest correction:** Explicitly gate the native path on requested audio and
honor the pinned picker's mute intent. Keep default/omitted `audio` semantics;
do not treat every generic browser `systemAudio: exclude` hint as a universal
ban on window audio outside this known Vencord integration.

**Evidence:** The Screen Capture specification defines omitted audio as false
and excludes audio tracks for false. Native source selection is not a substitute
for the per-share request: https://www.w3.org/TR/screen-capture/#dom-displaymediastreamoptions-audio

### 2. P2 — Audio failure after startup silently leaves a share without sound

Locations: `src/platform/audio.js:21-30,54,64-72,75-81,84-90`.

**Case:** Native startup succeeds and the browser audio track is removed/stopped.
Later the capture fails, the selected process closes or the worklet errors. The
handlers reject an already-settled promise and clean up the output track. The
startup catch/warning is no longer active, and the stopped browser track cannot
be reused.

**Impact:** Video keeps working but participants lose stream audio; there is no
new warning or recovery path for this post-start failure.

**Smallest correction:** Distinguish startup, active sharing and intentional stop.
Report unexpected active failures once and tell the user to restart sharing.
Only add automatic recovery after proving Discord replaces the outgoing sender;
adding another track to a stream is not a proven sender replacement.

### 3. P2 — Main-thread stalls directly interrupt native audio delivery

Locations: `src/platform/audio.js:48-52,60-69` and
`src/platform/audio-worklet.js:21-29`.

**Case:** Discord's page main thread stalls while a native share is active. Both
WebSocket reception and port acknowledgements execute there. The worklet keeps
rendering but receives no new samples; after its current samples drain, it emits
zeros. Its ring capacity is approximately 171 ms, not a guaranteed/pre-filled
171 ms of protection. Bounded queues prevent indefinite growth but do not remove
this thread dependency.

**Impact:** Page/UI load can cause audio gaps even when native capture is timely.
The actual frequency is unmeasured; this is not proof of the reported video issue.

**Correction:** Move socket reception and bounded delivery into a dedicated Worker,
transferring a MessagePort to the worklet. Keep WASAPI on its existing worker.
Verify existing CSP/worker support without weakening page security. This is a
measured optimization candidate, not a reason to rewrite the video engine.

### 4. P2 — Overload retains older PCM and has no end-to-end age check

Locations: `src/platform/audio.rs:287,305,401-410`; worklet ring at
`src/platform/audio-worklet.js:11-17`.

**Case:** The socket writer stalls while capture continues. The four-packet channel
fills; `try_send` drops each new chunk and retains the older queued chunks. The
writer can wait up to two seconds for one send. After recovery, retained PCM is
sent without capture timestamps, so neither receiver nor worklet can reject it
by age. The worklet's drop-oldest policy only applies after delivery there.

**Impact:** A transient stall can play stale audio and discontinuities on recovery.
Four 1024-frame chunks at 48 kHz represent about 85 ms of samples, but that is
not a bound on their wall-clock age or total pipeline latency.

**Correction:** Carry capture position/time and discard expired packets at delivery
boundaries, or use an explicit latest-data queue. Preserve bounded storage and
cancellation. Avoid reducing buffers blindly before validating jitter tolerance.

## Design and runtime gaps, not confirmed root causes

- **Synchronization:** `GetBuffer` receives null device/QPC-position outputs
  (`audio.rs:435`), and only the SILENT flag is acted on. DATA_DISCONTINUITY and
  capture timestamps are not conveyed. WASAPI and AudioContext have no explicit
  clock-drift or A/V correction. Actual drift has not been established. Preserve
  timing before adding resampling; do not claim merely adding both tracks to one
  stream synchronizes independent sources.
- **Startup response:** optional audio setup holds the returned video promise for
  up to the shared ten-second deadline. Slow worklet loading or suspended audio
  startup therefore delays publishing otherwise captured video. This is bounded,
  not a confirmed ongoing-stream FPS fault. Parallelize safe preparation only if
  startup is measured as the issue; retain browser capture consent ordering.
- **Background video:** Windows sets WebView2 visibility false when hidden or
  minimized (`windows.rs:195-199`). Microsoft recommends this and documents page
  throttling. That does not prove outgoing capture/encoding stops or stays smooth.
  Keep minimized-call acceptance open; do not change the policy on inference alone.
- **Quality/pinned patches:** Vencord supplies the picker, 4K/120 upper bounds,
  `motion` default hint and preview cleanup. Discard changes the target from
  0.6 to 6 Mbit/s, retaining the 8 Mbit/s quality ceiling. The separate upstream
  SDP patch advertises a different ceiling; effective encoder negotiation and
  regex compatibility with today's Discord modules are not established by static
  review. No hardcoded FPS, codec or bitrate change is justified by the earlier
  6 FPS sample taken on static content. A high-resolution/high-motion profile may
  need a different bandwidth policy, but no ideal bitrate is established here.
- **Platform coverage:** Native audio is Windows-only. macOS/Linux media behavior
  remains experimental; the current release pipeline builds Windows only.

## Safeguards confirmed in source

- The native capture wrapper is installed before Vencord captures getDisplayMedia.
- No native source means no WASAPI recording/listener in the normal share path.
- Native selection rechecks process identity and excludes Discard/ancestor trees.
- Loopback authorization requires an exact Discord Origin and random path token.
- Audio stays 48 kHz stereo float; silent WASAPI packets safely handle null data.
- Packet acquisition is paired with release; native handles and COM scope have owners.
- Server writes, including failure notifications, have timeout/cancellation paths.
- Native queues, main-to-worklet outstanding packets and worklet storage are bounded.
- Startup failure preserves live browser video. Stop, pagehide and errors release
  native output/context/socket resources; startup has one shared deadline.
- NORMAL memory priority, default GPU path and no forced suspension are retained.

## Checks actually performed

- Read the connected media code and shipped Vencord media plugin implementation.
- Verified SHA-256 of browser.js, browser.css and browser.js.LEGAL.txt against the
  committed asset manifest; all match.
- `node --check`: audio.js, audio-worklet.js and mods.js passed.
- `cargo fmt --all -- --check` passed. No Rust change, rebuild or redundant Clippy run.
- No unit/functional runs, UI actions, streams or performance measurements.
- Runtime application code, release configuration and local tauri.conf.json were not
  changed. This report adds no diagnostic UI, logging or background observer.

## References and verdict

- Screen Capture audio request semantics:
  https://www.w3.org/TR/screen-capture/#dom-displaymediastreamoptions-audio
- Track synchronization and stop behavior:
  https://www.w3.org/TR/mediacapture-streams/
- WASAPI discontinuities and capture timestamps:
  https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudiocaptureclient-getbuffer
- WebView2 visibility policy (checked with Context7):
  https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2controller
- AudioContext startup permission behavior:
  https://www.w3.org/TR/webaudio/#dom-audiocontext-resume

Fix 1 first, then active-failure handling and stale-audio recovery. Isolating audio
from the page main thread is the most direct architectural candidate for resilience
to UI stalls. Keep video changes evidence-driven. None of the audio findings alone
establishes the cause of low received video quality or proves a smooth stream.

## v0.3.8 remediation (source changes; runtime acceptance pending)

1. Native audio now requires a per-share audio request and honors Vencord mute.
2. Active audio failures warn once; normal native-menu close has a quiet reason.
3. Socket and acknowledged audio delivery moved into a disposable Worker.
4. Capture timestamps, discontinuity reset and stale-packet rejection were added
   at native/Worker boundaries. Fixed queues remain bounded. This does not claim
   sample-clock correction or an end-to-end latency guarantee.

The selected profile is additionally bridged to Discord stream settings, current
capture constraints and local encoding limits. Lower-to-higher selection preserves
source aspect ratio. Plugin disable flags are respected; private-API fallback
warns instead of pretending every plugin works. Windows real-call and Worker CSP
acceptance remain pending. Regression cases were added, but not run because the
project requires an explicit request for unit/functional runs.

## v0.3.9 follow-up

Quality updates now retain a last accepted profile, serialize applyConstraints,
and coalesce pending choices. Module shapes are checked per share and connection.
The worklet receives capture timing, resets at gaps and estimates source-clock
rate for bounded interpolation. This addresses the missing local drift handling,
without claiming receiver A/V calibration, validated pitch/quality or fixed latency.
The remaining real-call/Worker-CSP/platform acceptance gaps above remain open.
