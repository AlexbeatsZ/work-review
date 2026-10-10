# Background agents and central hub

## Scope

The user wants a quiet recorder on several personal computers, one aggregate frontend and macOS support, with current settings frozen or adjustable by commands. Chinese is the only UI language. Replace the previous Lite Phase 2 desktop direction. Keep capture, privacy, screenshot review and manual context; remove Tauri, model-provider UI, translations and desktop-only settings. Existing data directories are never deleted or modified by this refactor.

## Modules

- `core`: small shared configuration, activity protocol, privacy rules and SQLite storage. This is the durable data interface.
- `agent`: Windows/macOS adapters and capture/sync scheduling. UI lifetime has no effect on recording. Stable device UUID is persisted once; hostname is a label.
- `server`: authenticated ingest and queries, screenshot storage, embedded static web assets. It can run on Windows, macOS or Linux.
- `src`: Chinese Svelte dashboard, browser HTTP only.

Linux desktop adapters, floating-window aggregation and duplicate category logic are removed. Linux remains supported for the hub. Classification is owned by `core`.

## Capture and configuration

Frozen live OMEN settings verified 2026-10-03: record every 10 seconds, idle threshold 5 minutes, screenshots/OCR disabled, no work-hour filtering, all displays, adaptive image width, JPEG quality 85, screenshots retained 3 days, metadata 30 days, storage limit 2048 MB. Init can import the currently saved desktop JSON capture/privacy fields. Do not copy AI credentials or desktop settings. Local unsynchronized records/screenshots must never be removed by retention. Lock/idle/sleep and failed captures do not accrue long gaps as work. Configuration changes are validated and reloaded during recording. Native OCR only; no Python/model downloads.

## Synchronization

Immutable UUID records are queued locally. Server ingestion is transactional and idempotent by `(device_id, activity_id)`. Acknowledgment lists exact IDs; only acknowledged records are marked synchronized. Screenshots travel with their record. A failed request retains the local queue; bounded batches retry on the next cycle. Persist metadata even if screenshot or OCR fails. Only one collector may own a data directory at a time. Heartbeats include paused/idle/locked/permission/error state and backlog.

Collectors connect directly to their configured hub and disable inherited HTTP proxies. A Windows hub may run under SYSTEM at startup because it does not inspect a user's desktop; collectors still require an interactive user session. Login/startup wrapper invocations use process-scoped PowerShell execution policy so deployment does not change global policy.

The hub's configured IP can appear after the boot task starts, particularly for a Tailscale adapter. Retry only `AddrNotAvailable` bind failures inside the same running hub every five seconds. Keep the configured address and data-directory lock while waiting; `AddrInUse`, permission failures and invalid config still exit. A missing boot-time address must not permanently consume the task's finite restart attempts. A live Windows bind conflict without a visible TCP owner still requires an actual alternate-port probe; do not infer a free port solely from connection enumeration.

## Hub and time

Separate collector and viewer secrets; protect all data and screenshot endpoints. Never put credentials in URLs. No cross-origin access by default. Embed frontend assets in the hub binary. UTC epoch seconds are stored; browser day bounds define query intervals. Clip intervals at day/hour boundaries. Display both union activity time and summed device time so simultaneous use does not inflate the main activity total. Device filters apply to summary, chart, list and report consistently. Timeline uses stable cursor pagination and search applies to the whole selected range.

Invalidate the current query generation as soon as a search input changes, before its debounce timer starts a request. Otherwise an in-flight device refresh can replace search results during that delay. Cancel pending search timers when disconnecting. The browser regression delays an all-device response into this window.

## Service lifecycle

Provide scripts to install/uninstall a Windows interactive logon task or macOS user LaunchAgent. Collector scripts take explicit data/binary paths, quote paths safely and never require administrator/root privileges; the Windows SYSTEM hub installer requires an elevated session. Live deployment is separately authorized by the user. macOS accessibility/screen-recording permissions require local user approval; `doctor` explains missing permissions, background execution never repeatedly prompts, and permission requests for screen recording occur only when screenshots are enabled.

Windows collector tasks execute `bin/run-agent.exe`, a GUI-subsystem launcher compiled with the built-in .NET Framework compiler. It starts the sibling collector with `UseShellExecute=false` and `CreateNoWindow=true`, appends stdout/stderr logs and waits for the collector's exit code so Task Scheduler can retry failures. A direct console task action delegated to Windows Terminal can leave a visible blank window even with PowerShell's `-WindowStyle Hidden`; changing from PowerShell 7 to 5.1 reproduced the same failure on OMEN. Installers stop only this data directory's launcher and collector before updates. The hub's SYSTEM action remains separate.

macOS updates use a stable installed path. An optional existing signing identity signs the staged binary with identifier `io.work-review.agent`. If the installed binary has a certificate-backed designated requirement, the replacement must satisfy it before the running collector is stopped. Ad hoc signatures bind privacy authorization to one build and cannot preserve it across changes. Verify the LaunchAgent's actual state rather than SSH-inherited permission checks.

Frontmost macOS application/window metadata uses NSWorkspace and direct AXUIElement calls with bounded messaging timeouts and owned Core Foundation values. Do not depend on System Events/osascript for the core capture: live GUI LaunchAgent deployment exposed recurring subprocess timeouts even while SSH probes succeeded. Browser-specific optional URL helpers remain separate; their failure must not suppress the activity record.

## Existing records

Provide an explicit read-only import of old `workreview.db` with deterministic new IDs so repeated imports do not duplicate data. Preserve screenshot, OCR and intent-note fields where present. Old data and configuration remain in place. Import copies into the new agent queue; it never opens the old database for writing.
