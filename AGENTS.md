# Goal

Work Review is a personal activity recorder: quiet Windows/macOS agents collect locally and synchronize to one hub with a Chinese web dashboard. Keep current capture/privacy defaults, remove desktop UI and multilingual/provider configuration. The user authorized deployment to OMEN and Mac collectors with ROG as the central hub. No new global tools were installed.

# Current State

- Runtime version `0.2.0` on `codex/background-hub`, refactored from `88c1332`. Original desktop history remains on `lite-phase-2`.
- Cargo workspace: shared `core`, Windows/macOS `agent`, cross-platform HTTP `server`. The server embeds the Chinese Svelte frontend; Tauri, translations, AI providers and Linux desktop adapters have been removed.
- Immutable local SQLite queue, retry-safe uploads, per-device and union-time overview, notes, search, screenshots/OCR and Markdown export are implemented.
- Defaults match the live OMEN settings: 10-second sampling, 5-minute idle threshold, screenshots/OCR off, existing privacy rules. Init imports previous capture/privacy JSON; legacy database import is explicit and read-only.
- Deployed: OMEN interactive logon collector, Mac GUI LaunchAgent collector and ROG SYSTEM startup hub. Hub URL `http://100.106.169.46:47831/` uses the existing Tailscale network; 54,006 old OMEN records were imported read-only and uploaded. Defaults and device identities are preserved. See [deployment](docs/deployment.md) for paths, lifecycle and rollback.
- Mac runs the user-authorized, certificate-signed `bin/work-review-agent` in its project. Window metadata uses direct native AppKit/AX calls, avoiding recurring System Events subprocess timeouts. New records are being captured; screen recording remains unapproved/off. macOS updates must preserve the existing certificate and runtime path.
- Design index: [background-hub](docs/design/background-hub.md). Read before changing capture, sync, authentication, time accounting or service lifecycle.
- OMEN's Windows login action now runs a GUI-subsystem `data/agent/bin/run-agent.exe` launcher, compiled using Windows' existing .NET Framework. It starts the collector with `CreateNoWindow`, appends logs and propagates its exit code; the installer and release package include the launcher source.

# Active Work

- Completed: headless capture/sync, Chinese hub dashboard, CLI configuration reload, stable identity, opt-in logon/LaunchAgent scripts and read-only legacy import.
- Completed: storage/time/privacy/authentication/offline sync/lifecycle tests, browser tests and both native release builds. Four-platform GitHub CI passed for core refactor `d06a7d7`; see [validation](docs/validation.md).
- Completed: authorized deployment, direct hub networking, narrow Tailscale firewall rule, old desktop startup retirement and existing-certificate macOS signing. Both collectors generate new records that arrive at ROG, including after task/LaunchAgent restarts. Native capture and dashboard search fixes in code commit `261af74` passed all four CI platforms; see [validation](docs/validation.md).
- Completed 2026-10-09: repaired OMEN's blank Windows Terminal at login. Direct PowerShell 7 and 5.1 task actions both reproduced it; the no-console launcher and actual installer restart pass without a Terminal window. Capture remains healthy. The configured ROG hub was unreachable during this check; unsynchronized records remain local. Cold boot/logout was not forced.

# Build / Run / Test

- `npm ci && npm run build` before building the hub; `cargo build --release --workspace --locked` creates standalone binaries.
- `npm test`, `cargo fmt --all --check`, `cargo test --workspace --locked` after shared capture/storage/protocol changes.
- `cargo build -p work-review-server --locked` then `npm run test:e2e` checks the real hub and desktop/mobile frontend. Windows uses installed Edge; Linux CI installs Playwright Chromium in its isolated runner.
- Hub: `init`, `credentials`, `run`. Agent: `init`, `doctor`, `run`, `status`, `config --set ...`, `sync`, `import-legacy`. All support `--data-dir`. Detailed platform commands are in [README](README.md).
- After Windows launcher or installer changes: `pwsh -NoProfile -File scripts/test-agent-launcher.ps1`, PowerShell script syntax checks, then a targeted installed-task restart when authorized. The launcher regression also runs in Windows CI; it checks a real console child, path quoting, log append, exit status and startup failures.

# Durable Lessons

- Windows session-0 services and macOS system LaunchDaemons cannot reliably observe the interactive user's desktop. Collectors must run in the logged-in user's session.
- Initialize SQLite WAL/schema before starting concurrent collector and sync connections. Concurrent first-open mode changes produced `database is locked` on macOS despite a busy timeout.
- Legacy timestamps describe interval ends and merged rows can change; convert to interval starts during import and synchronize immutable new IDs. Pagination needs the full `(timestamp, activity_id, device_id)` tuple.
- macOS ad hoc code signatures bind TCC permissions to one build; stable path alone cannot preserve authorization. Use the existing certificate with a stable identifier, preserve the granted runtime path, and verify actual LaunchAgent capture rather than SSH-inherited authorization.
- A stopped Windows PowerShell task wrapper can leave its child alive. Installers stop only the exact installed binary before copying updates. SYSTEM also needs process-scoped execution policy; a user's policy is not inherited.
