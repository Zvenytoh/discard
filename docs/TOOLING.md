# Tooling and skills

## Development baseline

Discard is a root Rust crate with Tauri 2.12.4 and tauri-build 2.7.3, pinned in
`Cargo.toml` and resolved in `Cargo.lock`. Install the reviewed Rust CLI:

```sh
cargo install tauri-cli --version 2.12.3 --locked
```

There is no npm, React or frontend build pipeline. Use `scripts/dev.ps1` on
Windows and `bash scripts/dev.sh` on macOS/Linux, with tasks `check`, `fmt`,
`clippy`, `build` and `run`. The build/run helpers require CLI 2.12.3. Both use
the repository root. `build` invokes Tauri without bundling; packaging is separate.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

Unit and functional test runs require explicit user instruction. CI checks
formatting, Clippy and compilation without adding an automatic test suite.

Windows distribution uses the MSVC toolchain, C++ Build Tools, Windows SDK and
WebView2 Evergreen. Local x64 GNU builds additionally need `WebView2Loader.dll`
beside the executable; the PowerShell helper copies it. Machine-specific setup
may live in ignored `.tools/env.ps1`; tracked scripts must contain no developer's
absolute toolchain paths.

macOS development requires Xcode command-line tools and macOS 14+. Linux targets
Ubuntu 22.04+ with WebKitGTK 4.1, GTK 3, build-essential, OpenSSL, librsvg and
packaging dependencies; see the README's package command and CI's exact list.
Do not infer native runtime compatibility from cross-compilation.

## Tool scope

| Tool | Scope | Use |
| :--- | :--- | :--- |
| `AGENTS.md` | Repository | Contribution rules and validation limits |
| RTK | Global | Compact supported Git/Cargo/search output |
| Ponytail | Global skill | Simple, minimal implementations |
| Superpowers | Global skills | Targeted planning, execution and debugging |
| Graphify | Project skill | Architecture queries when an index exists |
| ast-grep | Global | Structural Rust search/refactoring |
| Context7 | Global MCP | Current external documentation when needed |
| Semgrep | Global | Relevant security audits |
| OpenAI skills | Selected global skills | Specialized tasks as needed |

User instructions override skill defaults. For this migration the user authorized
simple peripheral subagent tasks; core changes and final integration stay with
the primary agent. Do not broaden delegation without an applicable instruction.

Example scoped commands:

```powershell
rtk git status --short
rtk cargo build --release --locked
ast-grep run --lang rust --pattern 'unsafe { $$$BODY }' src
semgrep scan --config p/rust --metrics=off --error src
```

No Graphify index is required for this small codebase. Query an existing
`graphify-out/graph.json` before architecture exploration when present, and keep
generated output ignored. Do not regenerate unchanged graphs for routine edits.

## Local configuration

User binaries may be installed under `%USERPROFILE%\.local\bin`. Reload the
terminal after PATH changes. Existing Ponytail/OpenAI skills are reused rather
than copied into each repository. The Superpowers skills junction on this machine
points from `%USERPROFILE%\.agents\skills\superpowers` to
`%USERPROFILE%\.codex\superpowers\skills`; updates should be deliberate.

Context7 is configured globally in `%USERPROFILE%\.codex\config.toml` with
`https://mcp.context7.com/mcp`. Authentication belongs in the user's MCP settings,
never a tracked API key. Use official documentation if the service is unavailable.

The current machine may need an ignored `.tools/env.ps1` to select its installed
Rust/linker environment. CI uses native targets and does not rely on that file.
Report the exact compiler/linker and commands actually used; do not describe a
GNU-only local build as MSVC validation.

## Packaging

Windows keeps `scripts/build-installer.ps1` and Inno Setup 6.3+, with bootstrapper
signature verification. Linux packaging uses `cargo tauri build --bundles
deb,appimage -- --locked`; macOS uses `cargo tauri build --bundles dmg -- --locked`.
The Desktop workflow pins the CLI, names target artifacts distinctly, and allows
publication only from a version-matched tag after all builds pass. Apple signing
and notarization credentials belong in CI secrets, never local tracked files.

See [compatibility](validation/tauri-compatibility.md) and
[performance](validation/tauri-performance.md) for recorded evidence and blockers.

## References

- [Tauri prerequisites](https://tauri.app/start/prerequisites/)
- [RTK](https://github.com/rtk-ai/rtk)
- [Superpowers](https://github.com/obra/superpowers)
- [ast-grep](https://github.com/ast-grep/ast-grep)
- [Context7](https://github.com/upstash/context7)
- [Semgrep](https://semgrep.dev/docs/getting-started/)
