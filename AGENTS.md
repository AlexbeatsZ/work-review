# Goal

Work Review is a personal activity recorder: quiet Windows/macOS agents collect locally and synchronize to one hub with a Chinese web dashboard. Keep current capture/privacy defaults, remove desktop UI and multilingual/provider configuration. No global installation or live autostart changes without user authorization.

# Current State

- Runtime version `0.2.0` on `codex/background-hub`, refactored from `88c1332`. Original desktop history remains on `lite-phase-2`.
- Cargo workspace: shared `core`, Windows/macOS `agent`, cross-platform HTTP `server`. The server embeds the Chinese Svelte frontend; Tauri, translations, AI providers and Linux desktop adapters have been removed.
- Immutable local SQLite queue, retry-safe uploads, per-device and union-time overview, notes, search, screenshots/OCR and Markdown export are implemented.
- Defaults match the live OMEN settings: 10-second sampling, 5-minute idle threshold, screenshots/OCR off, existing privacy rules. Init imports previous capture/privacy JSON; legacy database import is explicit and read-only.
- Windows x64 and real macOS arm64 release builds, temporary captures and synchronization to one hub passed. Live login services have not been installed. Mac screen-recording permission was false; optional screenshots need local permission.
- Design index: [background-hub](docs/design/background-hub.md). Read before changing capture, sync, authentication, time accounting or service lifecycle.

# Active Work

- Completed: headless capture/sync, Chinese hub dashboard, CLI configuration reload, stable identity, opt-in logon/LaunchAgent scripts and read-only legacy import.
- Completed: storage/time/privacy/authentication/offline sync/lifecycle tests, browser tests and both native release builds. Four-platform GitHub CI passed for core refactor `d06a7d7`; see [validation](docs/validation.md).
- Deployment remains separate: choose a durable hub location, configure devices and install login services with user authorization.

# Build / Run / Test

- `npm ci && npm run build` before building the hub; `cargo build --release --workspace --locked` creates standalone binaries.
- `npm test`, `cargo fmt --all --check`, `cargo test --workspace --locked` after shared capture/storage/protocol changes.
- `cargo build -p work-review-server --locked` then `npm run test:e2e` checks the real hub and desktop/mobile frontend. Windows uses installed Edge; Linux CI installs Playwright Chromium in its isolated runner.
- Hub: `init`, `credentials`, `run`. Agent: `init`, `doctor`, `run`, `status`, `config --set ...`, `sync`, `import-legacy`. All support `--data-dir`. Detailed platform commands are in [README](README.md).

# Durable Lessons

- Windows session-0 services and macOS system LaunchDaemons cannot reliably observe the interactive user's desktop. Collectors must run in the logged-in user's session.
- Initialize SQLite WAL/schema before starting concurrent collector and sync connections. Concurrent first-open mode changes produced `database is locked` on macOS despite a busy timeout.
- Legacy timestamps describe interval ends and merged rows can change; convert to interval starts during import and synchronize immutable new IDs. Pagination needs the full `(timestamp, activity_id, device_id)` tuple.
