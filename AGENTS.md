# Discard — Project instructions

Read `README.md` and the relevant section of `docs/ARCHITECTURE.md` before editing.
Read `~/.codex/RTK.md` when present; its location is machine-specific.

## Principles

- Prefer simple, maintainable, idiomatic Rust and minimal, targeted changes.
- Avoid unnecessary abstractions, dependencies and files; do not modify unrelated code or another repository.
- Keep one root Rust binary crate and one Tauri external-content window. No npm/frontend toolchain.
- Target Windows 10/11 x64, macOS 14+ Intel/Apple Silicon, and Ubuntu 22.04+ x64. Treat unvalidated targets as experimental.

## Architecture boundaries

- Follow the implemented structure in `docs/ARCHITECTURE.md`; historical proposals are not current code.
- Keep navigation policy pure, independent of Tauri, native engines and filesystem effects.
- Keep lifecycle composition in `src/app.rs`, path resolution in `src/profile.rs`, and native effects inside `src/platform/`.
- Prefer concrete types and narrow visibility. Add traits, layers or crates only for a demonstrated need.
- Preserve structured error sources until presentation; never silently replace a failed profile with a fresh one.
- Keep architectural refactoring separate from unrelated features and dependency changes.

## Context efficiency

- Search relevant symbols with `rg` before reading whole files; use scoped output and avoid rereading unchanged files.
- Use RTK for supported Git/Cargo/search commands when available; fall back normally without hiding failures.
- Use the project Graphify skill for architecture questions when `graphify-out/graph.json` exists. Keep generated output local.
- Use Context7 when current external documentation is needed; fall back to official documentation.
- Use ast-grep for structural searches and refactoring when appropriate.

## Development

- Use Ponytail for minimal implementations and relevant Superpowers skills for complex planning/debugging.
- User instructions override skill defaults. Subagents need user authorization; the current migration permits simple peripheral tasks. Keep core migration and final integration with the primary agent.
- Localize application `unsafe` to `src/platform/` and document thread affinity, ownership and pointer invariants.
- Keep WebView2 COM operations on the owning UI STA thread via Tauri's native access. End interior borrows before potentially reentrant native calls.
- Use native events and Tauri's lifecycle. Add no polling, permanent app page scripts, forced suspension, cache clearing or GPU-disable flags.
- Preserve exact navigation rules, engine security defaults and existing user data.
- Grant no privileged remote capabilities, native commands or plugins to Discord. Framework-internal JS/IPC exists; do not describe the framework as injection-free.
- Keep Windows profile `%LOCALAPPDATA%\Discard\WebView2`, Inno app identity and `Zvenytoh.Discard` AppUserModelID stable.
- Do not run unit or functional tests unless the user explicitly requests them. When requested, run focused tests before full suites.
- For Rust changes run formatting, Clippy and a release build when available. CI runs these checks without unit/functional suites.
- Use Semgrep for relevant security reviews and specialized skills only when useful.

## Safety and reporting

- Never suppress errors or silently skip required validation.
- Ask before destructive or irreversible operations. Ordinary task-scoped local edits and checks are authorized.
- Never commit tokens, cookies, local profiles, signing credentials, tool binaries or generated graphs.
- Keep unsigned macOS artifacts explicitly experimental and out of normal public release assets.
- Summarize changes, exact checks, remaining issues and limitations concisely.
- Never infer runtime, login, media, install/upgrade or performance success from compilation. Record unavailable cases explicitly.
