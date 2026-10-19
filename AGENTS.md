# Discard — Project instructions

Read `README.md` and the relevant section of `docs/ARCHITECTURE.md` before editing.
Read `~/.codex/RTK.md` when present; its location is machine-specific.

## Principles

- Prefer simple, maintainable, idiomatic Rust.
- Avoid unnecessary abstractions, dependencies and files.
- Follow existing conventions and make minimal, targeted changes.
- Do not modify unrelated code or another repository.
- Target Windows, native Win32 and Microsoft Edge WebView2.

## Context efficiency

- Search relevant symbols with `rg` before reading whole files.
- Prefer scoped searches and concise output; do not repeatedly read unchanged files.
- Use RTK for supported Git, Cargo and search commands when available. Fall back to the original command if unavailable; never hide failures.
- Use the project Graphify skill for architecture questions when `graphify-out/graph.json` exists. Keep generated output local and rebuild only when needed.
- Use Context7 only when current external documentation is needed; use official documentation if the MCP is unavailable.
- Use ast-grep for structural searches and refactoring when appropriate.

## Development

- Use global Ponytail for simple, minimal implementations.
- Invoke the relevant Superpowers planning/debugging skill for complex work. User instructions take precedence over skill defaults.
- Do not use subagents unless the user explicitly asks for them.
- Keep `unsafe` inside `src/native/`; document FFI invariants.
- Keep COM and WebView2 on the owning STA thread. Never hold a `RefCell` borrow across a native call that can reenter.
- Use the blocking Win32 message loop; avoid polling and permanent page scripts.
- Preserve navigation allowlists, WebView2 security defaults and existing user data.
- Never expose native host objects, credentials or privileged IPC to remote pages.
- Do not run unit or functional tests unless the user explicitly requests them. When requested, run focused tests before full suites.
- For Rust changes, run formatting, Clippy and a build when the toolchain is available. CI runs these checks without tests.
- Use Semgrep for relevant security reviews, not automatically on every edit.
- Select specialized OpenAI skills only when relevant to the requested task.

## Safety and reporting

- Never suppress errors to make validation pass or skip required validation silently.
- Ask before destructive or irreversible operations; ordinary local edits and checks are authorized by the task.
- Never commit tokens, cookies, local profiles, tool binaries or generated graphs.
- Summarize changes, exact checks, remaining issues and limitations concisely.
- Do not claim runtime, login, voice, video or performance validation unless performed.
