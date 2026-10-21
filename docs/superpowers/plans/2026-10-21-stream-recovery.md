# Stream recovery and clock alignment

Execute inline with Ponytail and superpowers:executing-plans.

- [x] Serialize quality changes with a single replaceable pending request.
  Commit accepted capture parameters only after applyConstraints succeeds;
  restore Discord/picker state to the last accepted profile on failure.
- [x] Check stream module shapes when a share starts and when its connection
  appears; keep browser capture usable and honor disabled plugins.
- [x] Carry capture timing into the AudioWorklet, maintain a bounded playout
  timeline, reset after packet gaps and apply bounded fractional resampling
  to compensate source/render clock drift. Do not claim received A/V parity.
- [x] Add focused regression cases without running unit/functional tests, as
  required by AGENTS.md. Run syntax, formatting, lint/build checks.
- [ ] Refresh docs, publish Windows v0.3.9 and stop when release CI launches.

Ruling: No subagents, app launch, account actions or requested diagnostic UI.
The preceding user-approved four priorities define the scope; no new design
questionnaire. Full A/V calibration needs a live call and remains unverified.

Verification record: JS syntax, Rust formatting/Clippy passed; production build
started. Authored rejection/coalescing/module fallback and worklet timing/drift
regressions; unit/functional runs were not requested and were not performed.
