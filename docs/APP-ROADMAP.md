# Discard: upgrade priorities

Review baseline: v0.3.5, 2026-10-20. Static code/documentation review; no application
launch, real call, UI interaction, unit/functional suite or performance benchmark.
Effort below includes implementation and validation, not guaranteed elapsed time.

## Assessment

Keep the single Rust crate, Tauri, existing profile and platform boundaries.
The highest uncertainty is media behavior, not the number of architectural layers.
The native audio path is still experimental and should remain opt-in. Existing
navigation restrictions, updater signatures and remote capability denial stay intact.

## Priority and effort

| Order | Work | Effort | Completion criteria |
| --- | --- | --- | --- |
| 1 | Stabilize browser video and native audio separately | High; roughly 3–7 development/validation days, subject to WebView2 feasibility | Moving-content calls remain usable across focus changes, minimize/restore and stop/restart; no audio-triggered video cancellation or growing audio delay |
| 2 | Harden audio cancellation and transport | Medium; roughly 1–2 days, overlapping media work | Every network write is bounded/cancellable; native failure releases the worker; socket and worklet queues cannot accumulate old audio indefinitely |
| 3 | Validate lifecycle and recovery | Medium; roughly 1–2 days | Rapid launches, repeated tray transitions, mod restarts and interrupted updates preserve one usable instance and the same user data |
| 4 | Simplify and speed up CI | Low; roughly half a day to one day | Cargo artifacts cached appropriately; avoid duplicate branch/tag builds for one change; retain one production build, lint and signed release packaging |
| 5 | Define focused regression checks and refresh acceptance records | Medium; roughly 1–2 days | Navigation, RPC framing, cancellation and lifecycle risks have meaningful checks; compatibility/performance docs describe the actual current policy |
| 6 | Consolidate native settings and UI behavior | Medium; roughly 1–3 days | Persistent preferences, clear optional-audio selection/recovery and working caption controls at 100/125/150% DPI; unsupported options stay hidden |

These estimates overlap. Do not sum them into a promised delivery date. Full
official Discord parity or a replacement native media engine is outside this estimate.
Linux/macOS validation and packaging are a later, separately estimated phase.

## Baseline findings (implementation status below)

### Media transport

- `src/platform/audio.rs`: the `Frame::Failed` path awaits `writer.send` directly,
  unlike normal PCM writes which have a timeout and cancellation branch. A stalled
  peer can delay service teardown on that error path. Route errors through the
  same bounded write path.
- `src/platform/audio.js` and `audio-worklet.js`: the worklet ring and Rust channel
  are bounded, but posting every incoming packet does not bound the browser's
  pending MessagePort tasks. Introduce explicit backpressure/coalescing if the
  browser cannot consume packets promptly; do not build a second large queue.
- PCM packets carry samples without capture timestamps. Independent WASAPI and
  browser audio clocks currently have no explicit drift correction or video
  synchronization. Verify long-call drift before changing queue sizes or claiming
  low latency. Prefer the existing browser audio track where it meets the need.
- The user-reported 6 encoded FPS was measured on static content. It is not proof
  of an encoder bottleneck, nor evidence that moving-content sharing is stable.
  No resolution/codec/bitrate change should be justified by that snapshot alone.

### Build and acceptance

- `.github/workflows/desktop.yml` starts builds for both main and release tags,
  and invokes a plain release build before the Tauri production build. Review
  duplicate work; retain one complete packaging path and its integrity checks.
- `docs/validation/tauri-performance.md` still describes automatic LOW memory
  targeting, removed in v0.3.4. Update the stated policy without inventing results.
- Compatibility tables still leave media, tray and installation acceptance
  unverified. Compile success cannot close those entries.

## Recommended sequence

1. Fix bounded audio teardown and browser backpressure; keep optional native audio
   separate from the normal video path. Validate realistic moving-content sharing.
2. Validate lifecycle and updater recovery, then establish focused regressions.
   The repository currently prohibits unit/functional runs without an explicit
   user request; this roadmap does not change that instruction.
3. Improve CI and native settings after the media/lifecycle acceptance gate.
   Batch related fixes into reviewable releases rather than adding speculative
   media overrides on each report.

The removed diagnostic UI stays removed. No extra runtime logging of account,
message, process-path or capture content is proposed.

## Implementation update: v0.3.6

- Capture failure notifications now share the cancellable, two-second write path.
- Browser-to-worklet delivery has one unacknowledged packet and one replaceable
  pending packet; the existing bounded ring still drops oldest samples on overflow.
- Processor failure and interrupted startup release native output tracks without
  stopping browser video. No new polling or media override is introduced.
- Branch/PR CI keeps format/Clippy checks. Tags/manual runs perform the single
  production build and packaging; the extra plain release build is removed.
  Compatible Cargo artifacts are cached. No build-time saving has been measured.
- The performance policy document now reflects NORMAL memory targeting.

Moving-content sharing, independent clock drift, real audio quality, lifecycle
acceptance and install/update acceptance remain unverified. These changes do not
establish that the reported stream-quality problem is resolved. No diagnostic UI
is restored. Unit/functional runs still require an explicit request.

## Implementation update: v0.3.7

- Rich Presence frame assembly is limited to five seconds after its first byte;
  responses also have a five-second deadline. Idle established game connections
  remain open, and existing eight-client/frame-size/rate/cancellation limits stay.
- Repeated identical activities and empty disconnects no longer republish state.
- RPC publication now snapshots current state on the UI thread, with its lock
  released before window operations, avoiding stale queued activity updates.
- v0.3.6 Windows release build and publication succeeded. Installer, portable,
  update signature, manifest and checksums are published; no install/call/runtime
  validation is implied. Native audio and moving-content acceptance remain open.

## Implementation update: v0.3.8

Stream-audit fixes and picker/Discord profile integration are implemented; see
STREAM-AUDIT.md for source findings and remediation. The transport now uses an
optional dedicated Worker and timestamps. Real-call moving-content quality,
Worker/CSP support, A/V sync and clock drift remain acceptance gates. No account
feature, private capability or Electron-only plugin support is fabricated.

## Implementation update: v0.3.9

- Rejected quality changes restore the last accepted profile. Rapid changes are
  serialized and coalesced; stopped/replaced tracks cannot publish old results.
- Discord and picker preferences converge on the accepted capture profile.
- Compatibility checks run per share/connection, with lazy-store retry and normal
  browser fallback; checks cannot prove every upstream regex patch still matches.
- Audio playout now uses capture timing and bounded sample-clock compensation.
  Receiver-side synchronization and perceived quality remain runtime acceptance
  gates, not compile results. Regression cases were extended but not run under
  the project's explicit-unit/functional-test-request policy.
