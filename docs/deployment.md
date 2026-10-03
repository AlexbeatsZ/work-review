# OMEN / Mac collectors and ROG hub

Deployment authorized by the user and performed on 2026-10-04 (Asia/Singapore). No new global runtime or package was installed.

| Device | Role | Runtime/data location | Startup |
| --- | --- | --- | --- |
| META-OMEN | Windows collector | `C:\Users\Meta\Project\Workspaces\work-review\data\agent` | `WorkReviewAgent-Meta`, interactive user Meta, limited token, at logon |
| Meta's MacBook Pro, macOS arm64 | macOS collector | `/Users/meta/Project/Workspaces/work-review/data/agent` | `~/Library/LaunchAgents/io.work-review.agent.plist`, user GUI session |
| META-ROGALLY | Central HTTP hub, storage, aggregation, web UI | `C:\Users\Meta\Project\Workspaces\work-review\data\hub` on ROG | `WorkReviewHub`, SYSTEM, at system startup |

## Access and identities

- Hub URL: `http://100.106.169.46:47831/`, reachable through the existing Tailscale network. It listens only on ROG's Tailscale IPv4 address. Firewall rule `WorkReviewHub-Tailscale` allows TCP 47831 from `100.64.0.0/10` to this address; no LAN/public listener or router changes.
- OMEN device UUID: `40ab0a86-143c-48d6-811f-a39d8d7ccbeb`.
- Mac device UUID: `8403a563-2601-4bc7-9579-9c25428d17a8`.
- Hub keys are in ROG's private `data\hub\config.json`. The collector key is present only in agent configs; the viewer key is also saved privately on OMEN as `data\rog-view-key.txt` for convenient login. These files are excluded from Git. Hub data ACL allows Meta, SYSTEM and Administrators; the local viewer-key file is limited to Meta and SYSTEM.
- Collector HTTP requests bypass inherited HTTP proxies and connect directly to the configured hub. Existing device routing and global proxy settings were left unchanged.

## Records and retirement

OMEN's old database path was read from the existing desktop config, not inferred from the default Roaming directory. It was opened read-only and 54,006 historical records were imported and synchronized to ROG. New records use the same OMEN UUID. Native capture/privacy defaults remain 10 seconds, 5 minutes idle and screenshots/OCR off.

The previous `Work Review` HKCU Run entry was removed on OMEN and ROG, and any exact `C:\Portable Programs\Work Review\Work_Review.exe` process was stopped. Original executables, configs and databases remain in place. The removed entry is saved in `legacy-startup.json` inside each replacement runtime's data directory. ROG runs the hub only; it does not run a new collector.

Mac originally had no Work Review collector. Its approved stable program path is `/Users/meta/Project/Workspaces/work-review/bin/work-review-agent`; records/config remain under `data/agent`. The LaunchAgent runs the exact binary granted accessibility access by the user. SSH's permission check is insufficient because its inherited authorization differs from the LaunchAgent. Screenshots are disabled, so screen-recording permission is unnecessary.

The Mac binary is signed with the existing `Local Development Code Signing` identity and identifier `io.work-review.agent`. This reuses the existing keychain certificate without adding a certificate or changing keychain policy. Updates must preserve that identity: run the Mac installer locally with the original signing identity as its third argument. The installer preserves the existing LaunchAgent program path and checks the replacement against the installed designated requirement before stopping the collector. The first accessibility grant was bound to the old ad hoc build; TCC logs confirmed a code-hash mismatch. The user re-added the signed project `bin` executable; the LaunchAgent was configured to use that authorized path and actual new capture was verified.

Window metadata now uses native NSWorkspace/AX calls. The deployed System Events subprocess approach intermittently exceeded its timeout in the GUI LaunchAgent, despite fast SSH probes. Native reads retain the same accessibility/privacy boundary and bound per-object messaging timeouts; browser-specific optional URL collection remains separate from the core window read.

## Lifecycle and checks

Windows tasks permit battery operation, have no execution time limit and retry failed exits after one minute. Mac's LaunchAgent uses `RunAtLoad`, `KeepAlive`, the logged-in Aqua session and a 30-second restart throttle. The ROG hub needs no interactive login. PowerShell startup wrappers apply execution policy only to their own process; no global execution policy was changed. Windows installers explicitly stop the exact installed child binary after stopping its task wrapper, preventing stale processes from locking updates.

```powershell
# OMEN
.\data\agent\bin\work-review-agent.exe --data-dir .\data\agent status
Get-ScheduledTask -TaskName WorkReviewAgent-Meta
Get-Content .\data\rog-view-key.txt

# ROG, elevated session for startup-task changes
Get-ScheduledTask -TaskName WorkReviewHub
Get-Content C:\Users\Meta\Project\Workspaces\work-review\data\hub\hub.stderr.log -Tail 20
```

```bash
# Mac
"$HOME/Project/Workspaces/work-review/bin/work-review-agent" \
  --data-dir "$HOME/Project/Workspaces/work-review/data/agent" status
launchctl print gui/$(id -u)/io.work-review.agent
launchctl kickstart -k gui/$(id -u)/io.work-review.agent
```

The live browser check can be repeated without putting secrets in arguments:

```bash
node scripts/check-deployment.mjs http://100.106.169.46:47831 data/rog-view-key.txt
```

It accepts a private viewer-key text file or hub config, logs only sanitized device status and saves a local screenshot under `artifacts/deployment/`. The browser check does not edit records. Cold boot / logout were not forced during deployment; the effective startup definitions and live process sessions were checked directly.

## Rollback

Use `scripts/install-agent.ps1 -Uninstall` on OMEN, `bash scripts/install-agent.sh --uninstall` on Mac, and elevated `scripts/install-hub.ps1 -DataDir <hub-data> -Uninstall` on ROG. These remove startup registrations and preserve records. Remove only the `WorkReviewHub-Tailscale` firewall rule if retiring the hub. Restore the saved old HKCU Run entry from `legacy-startup.json` only after stopping the replacement collector to avoid duplicate recording. Device UUIDs and config files should be kept across updates.
