# Native integration execution

Spec: docs/native-integration.md. Windows first; keep Rust/Tauri and existing data.

1. Embed checksum-verified Vencord browser assets at a fixed commit, local CSS, origin/main-frame guard, native tray disable/restart recovery.
2. Add event-driven Windows foreground-game detection and process-exit cleanup with a filtered, pinned executable catalog. Send only sanitized activity into the Vencord adapter; retain across page reload.
3. Add a manually enabled application-audio prototype using WASAPI process loopback and a bounded binary transport to a browser MediaStream; no raw video transport or arbitrary remote commands. Keep native selection and explicit consent. Existing browser video capture remains.
4. Format, Clippy, JS syntax and release compilation; review the complete change. Actual call acceptance remains manual as requested by the user.

No unit/functional tests are run without an explicit user request (AGENTS.md). No runtime/performance claims from compilation. A native-to-browser track prototype is experimental until another participant confirms audio, clean stop and no echo.

Execution: tasks 1–3 implemented; task 4 formatting, Clippy and JS syntax pass, fresh review finding fixed. Release compilation runs in Windows CI. Real-call acceptance and CPU/memory measurements remain pending; audio stays labeled experimental. No official Discord native binaries are reused.
