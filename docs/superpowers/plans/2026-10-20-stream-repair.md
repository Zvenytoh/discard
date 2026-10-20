# Stream repair implementation plan

Use superpowers:executing-plans inline. No agents or app launch. Preserve the
profile, navigation allowlist, updater signatures and pinned upstream assets.

- [x] Gate native audio on per-share consent and the active Vencord mute choice.
- [x] Move socket/ack transport to a disposable Worker; report active failures once.
- [x] Add PCM capture timestamps, discontinuity reset and age rejection in Rust/Worker.
- [x] Sync current picker selection to Discord streaming settings; update local
  stream quality after capture, preserve actual aspect ratio and plugin toggles.
- [x] Verify syntax, formatting and Clippy; release builds started; record runtime checks as pending.
- [x] Refresh docs and prepare v0.3.8 publication; stop when CI starts.

Ruling: Unit/functional runs require explicit user authorization per AGENTS.md;
this request does not explicitly request them. Do not run them or open Discord.
Ruling: Audit already specifies the four defects. Implement directly without
another design questionnaire. No claim of measured FPS or universal plugin parity.
Ruling: Public Discord bootstrap was inaccessible (403); use guarded compatibility
with published plugin APIs, not fabricated current Discord internals.

Implementation notes: regression cases added in tests/stream.test.mjs but not run.
No private Discord bundle could be inspected (403); guarded store initialization
and local video parameters follow published browser-plugin integration patterns.
Compatibility still requires a real user-controlled share, not a compile claim.
