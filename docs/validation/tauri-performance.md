# Background resource validation

No CPU/RAM saving is claimed for the Tauri migration without a measured result.
Tauri still renders Discord through a browser engine; executable size and host
language do not establish total process-tree memory or call reliability.

## Implemented policy

- One native event loop, one WebView, persistent profile and default GPU path.
- Windows updates renderer visibility from native minimize/restore events and
  retains NORMAL memory priority, including silent calls and streams. Automatic
  LOW targeting was removed in v0.3.4; no playback observer changes this policy.
  Missing optional engine interfaces leave engine defaults in place.
- macOS 14+ disables background suspension policy to favor ongoing activity.
  This can increase background CPU/memory; no equivalent savings are promised.
- Linux retains supported engine scheduling. No injected keepalive, periodic
  reload, polling, cache deletion or forced JavaScript suspension is introduced.

## Required measurement protocol

Compare the retained v0.1.0 executable with the candidate sequentially, using
the same hardware, Discord content and profile. Close each app before opening
the other; do not run compilation while collecting comparable measurements.

For each of three repetitions: warm up 60 seconds, active idle 60 seconds,
visible inactive 60 seconds, minimized 180 seconds. Attribute only the host and
its browser descendants, including dynamically spawned descendants. Report CPU
time/duration and private memory separately from working set; shared working-set
sums can count shared pages more than once. Record engine and OS versions.

Also verify ten minutes minimized with user-controlled incoming messages and
calls, including silent audio/video and restore responsiveness. No account
content, cookie or message text belongs in measurement artifacts.

## Results

The full paired three-repeat protocol and live-media checks have not yet been
performed. No comparative percentage or memory ceiling is established.
Any shorter process smoke check must be labeled as such and must not be used
as evidence of the requested before/after resource improvement.
