# Discard Tauri Multiplatform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Project instructions prohibit subagents unless the user explicitly requests them.

**Goal:** Ship a minimal Tauri Discord Web client on Windows, with macOS and Linux builds progressing through explicit compatibility gates.

**Architecture:** Keep one root Rust binary crate and the pure navigation policy. Replace the direct Win32 event loop with a Rust-created Tauri WebviewWindow and isolate the few native appearance/background operations in platform modules. Load Discord remotely without a frontend toolchain or privileged remote IPC.

**Tech Stack:** Rust stable, Tauri 2, tauri-build/tauri-cli 2, url, platform webviews, existing Inno Setup, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-20-tauri-multiplatform-design.md` (read alongside this plan).

## Global Constraints

- Initial release: 0.2.0. Desktop only.
- Windows 10/11 x64; macOS 14+ Intel and Apple Silicon; Ubuntu 22.04+ x64 baseline.
- Tauri 2.12.4 or a later reviewed stable 2.x patch; commit resolved Cargo.lock and record exact compatible build/CLI versions.
- One Rust binary crate at the repository root; one `main` WebviewWindow; `https://discord.com/app`; 1200 by 800 logical pixels.
- Stable bundle identifier `com.zvenytoh.discard`; Windows AppUserModelID `Zvenytoh.Discard`.
- Windows profile `%LOCALAPPDATA%\Discard\WebView2`; Inno AppId `{D5328CCE-9114-46C0-BE78-6EB7613DB98E}`.
- macOS data-store identifier `[213,50,140,206,145,20,70,192,190,120,110,183,97,61,185,142]`.
- Dark background/caption `#1E1F22`; Windows caption text `#F2F3F5`; existing coral Discard artwork.
- Exact HTTPS `discord.com`, effective port 443, no credentials; deny new windows/downloads; no privileged remote capability/invoke handler/plugins.
- No React/Vite/npm, extra WebView, polling, permanent app-injected scripts, forced suspension, reload/cache clearing, GPU-disable flags or security weakening.
- Preserve live messages/calls and user data; closing the only window exits on every OS.
- Unit/functional test runs require an explicit user request under AGENTS.md. This plan uses focused static checks and the authorized Windows manual checks; other-machine/account actions remain user-controlled.
- No subagents, installation into the user's live app directory, publication or signing claims during planning.

## Review Focus

1. Existing Windows profile locked or reused after upgrade: keep original data and explain failure, never silently reset (Tasks 1, 5).
2. Silent audio/video calls after ten minutes minimized: audio silence must not be mistaken for proof that no call exists (Tasks 1, 3).
3. Remote frames, redirected URLs and encoded lookalike hosts: deny native privileges and unsafe main-frame navigation (Task 2).
4. App launched minimized or closed during asynchronous startup: no flash/reopened window/orphan engine process (Tasks 1, 3).
5. WebKit codec, permission and Linux graphics differences: label actual supported features per target; never infer support from CI compilation (Tasks 1, 4, 5).

## File structure and ownership

| File | Responsibility |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, `build.rs`, `tauri.conf.json` | Root crate, Tauri build metadata, security and bundle configuration |
| `src/main.rs` | Entry point and final failure reporting |
| `src/navigation.rs` | Existing pure URL rule, kept platform independent |
| `src/app.rs` | Tauri setup, single window, event wiring and explicit exit |
| `src/profile.rs` | Platform profile resolution, no credential access |
| `src/platform/mod.rs` | Narrow common native entry point and cfg dispatch |
| `src/platform/windows.rs` | DWM identity/theme and WebView2 background adaptation |
| `src/platform/macos.rs` | WKWebView data-store and supported background settings |
| `assets/icons/` | Generated PNG/ICO/ICNS brand variants |
| `Info.plist`, `Entitlements.plist` | macOS bundle privacy descriptions and signed distribution requirements |
| `scripts/dev.ps1`, `scripts/dev.sh` | Matching Cargo/Tauri development commands |
| `scripts/build-installer.ps1`, `installer/Discard.iss` | Preserve existing Windows installer/upgrade contract |
| `.github/workflows/desktop.yml` | Native target checks, packaging and a single release job |
| `docs/validation/tauri-compatibility.md` | Engine-specific results and unresolved cases, no account content |
| `docs/validation/tauri-performance.md` | Reproducible measurements and observed tradeoffs |

Delete `src/native/mod.rs` and `src/native/webview.rs` only after Task 1's host
replacement builds. Do not create an empty Linux adapter, frontend tree,
capability grant file or speculative service layer.

## Task 1: Minimal Tauri host and engine compatibility gate

**Files:** Modify `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `.cargo/config.toml`; create `build.rs`, `tauri.conf.json`, `src/app.rs`, `src/profile.rs`, `src/platform/mod.rs`, `src/platform/macos.rs`, `docs/validation/tauri-compatibility.md`; replace the old `src/native/` integration after build parity.

**Interfaces:**
- Consumes: `navigation::HOME: &str`, `navigation::allowed(input: &str) -> bool`.
- Produces: `app::run() -> tauri::Result<()>`; `profile::directory(app: &tauri::AppHandle) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>>` (Windows/Linux return a directory, macOS returns None); `platform::configure(builder: tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App>) -> tauri::webview::WebviewWindowBuilder<'_, tauri::Wry, tauri::App>`.

- [x] **Step 1:** Record the v0.1.0 executable location, git revision and old profile path in the compatibility report. Preserve the old executable outside tracked source for sequential comparisons; never read profile cookies or overwrite the installed copy.
- [x] **Step 2:** Add reviewed stable Tauri/build dependencies, generate a locked dependency graph and record exact tauri-cli version plus installation command. Keep the root crate layout and release settings; constrain Windows-only bindings to Windows. Use the Cargo CLI, no Node toolchain. Set version 0.2.0 in Cargo and derive bundle version from Cargo rather than maintaining duplicate version strings.
- [x] **Step 3:** Add `build.rs` calling `tauri_build::build()` and a minimal Tauri configuration with empty initial windows, no remote capabilities, `withGlobalTauri: false`, stable identifier and Rust-created external main window. Omit a frontend build path/server; if the pinned CLI requires an asset directory, use one inert local fallback asset without introducing a second WebView or build system.
- [x] **Step 4:** Implement the produced interfaces. Resolve Windows LOCALAPPDATA explicitly, Linux through `app.path().app_local_data_dir()` plus `webview`, and macOS through the fixed WKWebView store identifier in platform configuration. Do not silently substitute a fresh profile on errors. Configure strict navigation/new-window/download policies immediately, before loading Discord, even though Task 2 audits them in depth.
- [x] **Step 5:** Wire setup to create `main` once; title Discard, 1200x800, native dark theme, persistent data and no invoke handler/plugins. Closing main exits explicitly on macOS too. Replace entry-point Windows compile_error and old native ownership with Tauri lifecycle. Remove direct native modules only once the replacement compiles.
- [x] **Step 6:** Run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo build --release --locked` on an available native toolchain. Expected: successful exit. Local GNU/MSVC availability must be reported precisely; use MSVC CI for the Windows shipped target.
- [ ] **Step 7:** On available native systems inspect launch, Discord login page/rendering, restart persistence, clipboard, file drag/drop, profile-lock error, minimize at startup and close during startup. A user-controlled account checks login and a ten-minute minimized session with incoming messages plus silent microphone/video calls. Record observed result or unavailable; do not claim all engines passed on a Windows machine.
- [x] **Step 8:** Stop expansion of any target whose basic login/rendering/native isolation fails. Record whether a small supported engine adjustment is sufficient; keep that target experimental or blocked instead of adding a browser engine, spoofing user agents or bypassing security.
- [x] **Step 9:** Commit the working host and report: `git commit -m "feat: migrate Discord host to Tauri"` with explicit task-owned paths staged.

## Task 2: Remote-content security audit and parity

**Files:** Modify `src/app.rs`, `src/navigation.rs` only if needed, `tauri.conf.json`, `docs/validation/tauri-compatibility.md`.

**Interfaces:** Consumes Task 1's run/setup and existing `allowed(&str) -> bool`; produces the same interfaces with reviewed deny-by-default web hooks. No native command API is introduced.

- [x] **Step 1:** Pin a navigation case table in the report: allow `https://discord.com/app`, `https://discord.com/channels/@me`, `https://discord.com:443/app`; deny HTTP, `https://discord.com.evil.test`, `https://discord.com@evil.test`, `https://user@discord.com`, port 444, `file:`, `data:`, `javascript:`, localhost and encoded-host lookalikes. Document URL-parser normalization expectations; paths/query/fragment do not expand the host rule.
- [x] **Step 2:** Connect `on_navigation` directly to `allowed(url.as_str())`; deny popup creation using `NewWindowResponse::Deny`; return false for requested downloads. Verify main-frame denial does not block legitimate Discord subresources. Keep native browser security defaults and add no opener/shell/filesystem plugin.
- [ ] **Step 3:** Audit effective compiled capability configuration: no remote origin/window grants, no implicit default capability accidentally bundled, no invoke handler and no exposed app-native command. Verify a controlled remote-frame/invoke probe cannot cause a native side effect. `withGlobalTauri: false` alone is not proof. If a runnable security harness would be needed, obtain the project's explicit test-run authorization rather than silently adding it to CI.
- [ ] **Step 4:** On available engines manually verify denied main-frame redirects, popup/download attempts and preserved Discord resources. Never send messages to create a test fixture; use user-provided actions or controlled non-account pages. Record unavailable cases and security blockers.
- [x] **Step 5:** Repeat focused formatting/Clippy/build for modified Rust and commit `fix: enforce remote webview isolation` with explicit paths. If no code correction was needed, commit only the audit report with `docs: record Tauri isolation review`.

## Task 3: Brand, native theme and conservative background behavior

**Files:** Modify `src/app.rs`, `src/platform/mod.rs`, `src/platform/macos.rs`, `Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `assets/LOGO.md`, `installer/Discard.iss`; create `src/platform/windows.rs`, `assets/icons/`, `docs/validation/tauri-performance.md`.

**Interfaces:** Consumes `platform::configure(...)` from Task 1; produces `platform::attach(window: &tauri::WebviewWindow) -> tauri::Result<()>`, called once after construction. Native event wiring and native state stay private inside the adapter; Linux's attach is a successful no-op.

- [x] **Step 1:** Generate ICO/ICNS/PNG variants from `assets/logo.png` using the pinned Tauri CLI icon command. Check 16/32px recognition and platform bundle icons. Configure bundle/default window icon and preserve the coral master. No logo redesign.
- [x] **Step 2:** Implement dark native theme/background. Windows adapter sets AppUserModelID `Zvenytoh.Discard`, matching Inno shortcut entries; apply Windows 11 caption/text colors and supported Windows 10 fallback, including theme changes. Leave macOS/Linux native title-bar conventions intact.
- [x] **Step 3:** In Windows `attach`, use `with_webview` on its owning UI thread and the binding types exposed by the locked Tauri/Wry dependency graph; avoid incompatible duplicate COM bindings. Apply current minimized state immediately. Cache applied render/memory state; recompute only on native window/audio events. Minimized rendering is hidden; active/audio NORMAL, other inactive/minimized LOW. Feature-detect optional memory/audio interfaces; if activity cannot be observed or an optimization fails, retain NORMAL and a usable app. Release callback references on shutdown and never hold a Rust interior borrow across reentrant calls.
- [x] **Step 4:** Set macOS background throttling to the supported Disabled policy in builder configuration. Add no cross-platform polling emulation for Linux and no inferred call detection from audio silence. Keep no-suspension/no-reload policy on all targets.
- [ ] **Step 5:** Run focused static/build checks. Manually inspect taskbar/Alt+Tab/Dock icons, Windows theme/DPI changes, native caption, startup minimized, repeated restore and exit with no orphan Discard engine processes.
- [ ] **Step 6:** Execute the spec's sequential three-repeat Windows baseline/candidate measurements, attributing only each app's process tree. Record versions, hardware, profile/content state, sample method, CPU, private memory, working set and variability. Do not conflate reduced working set from paging with reduced private allocation or improved responsiveness.
- [ ] **Step 7:** Record ten-minute minimized message/call checks per available OS, including silent call, capture permission denied, restore responsiveness and camera. User controls calls/messages; no content is captured. If LOW memory causes call glitches, retain NORMAL in the shipped default and document measured tradeoffs rather than ship the regression.
- [x] **Step 8:** Commit `feat: add native branding and background resource policy` with task-owned files.

## Task 4: Native builds, packages and controlled release publication

**Files:** Create `.github/workflows/desktop.yml`, `Info.plist`, `Entitlements.plist`, `scripts/dev.sh`; modify `scripts/dev.ps1`, `scripts/build-installer.ps1`, `installer/Discard.iss`, `tauri.conf.json`, `.gitignore`; retire `.github/workflows/windows.yml` when the replacement provides Windows parity.

**Interfaces:** Consumes the root crate and Task 3 icons. Produces four uniquely named CI artifact groups: Windows x64, Linux x64, macOS x64, macOS arm64; one release job consumes normalized package files and creates SHA256SUMS.txt.

- [x] **Step 1:** Align developer helpers on root Cargo/Tauri commands and pinned CLI version. Preserve ignored machine-specific toolchain setup; avoid adding absolute paths to tracked scripts. Document Linux WebKitGTK 4.1/native prerequisites and macOS build tools in helper usage.
- [x] **Step 2:** Keep Windows packaging on Inno with the existing install directory/AppId/executable/profile-preservation behavior. Feed the Tauri-built root `target/release/discard.exe` to the current package script. Retain signed Microsoft bootstrapper verification and stable portable/setup filenames; do not add a second Windows installer technology.
- [x] **Step 3:** Configure macOS 14 minimum, stable bundle ID, microphone/camera usage descriptions and signing entitlements appropriate to the app's actual capture behavior. Build separate Intel/Apple Silicon DMGs. Keep signing/notarization credentials in CI secrets only. Missing credentials yield explicitly unsigned experimental CI artifacts, not an ordinary public macOS release.
- [x] **Step 4:** Build Linux DEB/AppImage on Ubuntu 22.04 with documented WebKitGTK 4.1 dependencies. Keep default rendering/GPU behavior; record codec/Wayland/X11 differences in validation, adding a graphics workaround only for a reproduced issue.
- [x] **Step 5:** Add native matrix jobs: Windows MSVC x64, Ubuntu 22.04 x64, macOS arm64 and macOS x64 via an available compatible runner/target. Each runs locked formatting/Clippy/release checks and platform packaging. Install exact CLI, verify Cargo version matches tag; upload artifacts with unique names and error on missing expected files. Keep unit test commands out unless separately authorized.
- [x] **Step 6:** Add a single tag-only publication job after all required jobs succeed; restrict contents-write to this job. Normalize unique target filenames, compute SHA256SUMS.txt, and publish once. Exclude unsigned macOS packages from public installable assets; identify experimental/blocked runtime cases in notes. PR builds cannot access signing secrets and cannot publish. Workflow dispatch on a branch produces artifacts only.
- [ ] **Step 7:** Validate workflow/config syntax, run a branch artifact build, and inspect every expected package and checksum. Exercise tag-version mismatch as a packaging check only with explicit test-run authorization; otherwise inspect the guard statically and mark it unexecuted. Record missing runners/secrets as blockers.
- [x] **Step 8:** Commit `build: package Tauri desktops with native CI`. Do not push a release tag as part of a documentation-only planning request.

## Task 5: Upgrade acceptance, support matrix and documentation

**Files:** Modify `README.md`, `AGENTS.md`, `docs/ARCHITECTURE.md`, `docs/TOOLING.md`, `docs/validation/tauri-compatibility.md`, `docs/validation/tauri-performance.md`; annotate the earlier Windows-only design as superseded after implementation.

**Interfaces:** Consumes Task 4 artifact names/checksums and earlier validation reports; produces an English README with accurate downloads, prerequisites and feature support by OS.

- [ ] **Step 1:** On a controlled Windows installation upgrade v0.1.0 to the candidate. Verify one uninstall entry, working shortcuts/icon, unchanged profile/sign-in where the user has provided a session, clean process exit and preserved profile after uninstall. Record installer actions not performed; do not install into the user's live application without authorization.
- [ ] **Step 2:** On available macOS/Linux systems verify install/launch/relaunch/uninstall and record codecs, clipboard, file drag/drop, microphone/camera denial, messages, audio, silent calls, video, notification and screen-sharing results independently. Missing hardware or account interaction means unverified, not passed.
- [x] **Step 3:** Rewrite architecture around implemented Tauri boundaries; update AGENTS Windows-only/direct-Win32 constraints and unsafe location to `src/platform/`. Retain pure policy, scoped native effects, no subagents and explicit-request test rules. Document framework-internal IPC versus actually granted remote permissions precisely.
- [x] **Step 4:** Update English README with available OS artifacts, exact build commands, session locations, per-engine caveats, unsigned experimental macOS status when applicable and measured background results. Preserve Discard logo and explain that mobile, tray and automatic app updates are excluded.
- [ ] **Step 5:** Perform a final spec-to-deliverable review and `git diff --check`; run the focused static/build checks only where the last code changes justify them. List exact completed commands and all unavailable runtime cases. Commit `docs: document Tauri platforms and migration support`.
- [ ] **Step 6:** Present candidate artifacts, support matrix and measurements for release review. Publication uses the existing authorized release scope only once the actual candidate and blockers are reviewable; never call an unvalidated platform fully supported.

## Plan self-review

- Coverage: all design sections map to Tasks 1–5; earlier icon/theme/background requests are included in Task 3.
- Interfaces: Task 1 defines app/profile/builder ownership; Task 3 adds a post-build attach hook without moving policy into platform code.
- Risk mapping: profile/upgrade (1,5), silent background activity (1,3), remote boundaries (2), startup races (1,3), engine variance (1,4,5).
- Validation: static and manual checks are explicit; unit/functional harness execution requires the existing project authorization rule. CI does not run an unrequested suite.
- Delivery: preserve Inno compatibility; signing and unavailable native runtime checks remain visible blockers, not invented passes.

## Execution handoff

Recommended: native execution in this chat with `superpowers:executing-plans`,
without subagents, as required by the project. Review this plan and its design
before application implementation. This planning change creates documentation
only; it does not migrate the host, publish v0.2.0 or promise lower memory use.
