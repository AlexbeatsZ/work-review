# Goal

- Deliver a cohesive Windows-native desktop and dark-only native Android Work Review interface using Microsoft Fluent design language without copying Windows application assets.
- Fix Android blocked-app management so every resolvable app is presented as icon + human-readable name, with the package name only as secondary technical context.
- Preserve existing collection, privacy, export, timeline, categorization, and storage behavior.

# Current State

- Branch: `codex/lite-without-intent`, with the native Android project from `codex/mobile-android-mvp` integrated into the cross-platform product.
- Desktop (Windows): native WinUI 3 + Windows App SDK 1.7 app in `desktop-winui/` (C# / .NET 8, unpackaged self-contained portable) talking to the Rust engine over P/Invoke. The Tauri 2 + Svelte shell remains buildable for macOS/Linux from the same lib.
- Desktop routes: timeline, summary (时段摘要), settings (常规/隐私/存储 pivot).
- Android: native Kotlin/Jetpack Compose app with overview, timeline, settings, export, diagnostics, and a full-height blocked-app picker.
- Android app identity resolution now combines package-manager labels, decoded launcher icons, known labels, and a readable package-derived fallback. The package name is only secondary context in the blocked-app sheet.
- Desktop release `1.1.0` is the WinUI 3 native shell: real Mica system backdrop, 32 px custom title bar, NavigationView pane, SelectorBar settings tabs, compact grouped lists, tray + single-instance + registry autostart (self-healing path on startup).
- Windows production copies are portable deployments at `C:\Portable Programs\Work Review\Work_Review.exe` on both the local PC and Windows server. Do not deploy or update Work Review through the NSIS installer or `%LOCALAPPDATA%\Programs`.
- Android release `0.1.1` retains the touch-native Dark Current direction: cool graphite surfaces, blue-gray text, restrained blue signals, sans-serif typography, and dense content-first layouts.
- App and UI icons use Microsoft Fluent System Icons (`microsoft/fluentui-system-icons`) across desktop, web, Android launcher, and system tray.
- Design contract: [docs/design/cross-platform-interface.md](docs/design/cross-platform-interface.md).

# Active Work

- Completed: rebuild the Windows desktop frontend as a native WinUI 3 app (`desktop-winui/`, `1.1.0`) on top of the shared Rust engine; the Svelte/WebView shell stays available for macOS/Linux but is no longer the Windows production path.
- Completed: split `src-tauri` into lib+bin with framework-free `collection.rs` / `shell.rs` / `events.rs`; added the `ffi` feature building `work_review_engine.dll` (engine_start / engine_invoke / event callback / registry autostart). 88 Rust tests pass; node suite back at baseline (93 pass / 14 fail).
- Completed: visually verified the deployed portable EXE on timeline (live collection, icons, category pills, real upsert), summary (intent distribution + hourly bands), and settings (general/privacy/storage pivots bound to the real config).
- Completed: deploy Windows `1.1.0` to the local PC portable folder; verified process path/version and the repaired `Work Review` registry autostart entry.
- Completed: enable the native DWM system backdrop and rounded corners, with transparent WebView surfaces and a deterministic Mica fallback for unsupported sessions.
- Completed: verify the real release EXE on the timeline, summary, and settings routes; the old glowing pill cards are replaced by compact grouped-list rows.
- Completed: deploy Windows `1.1.0` (WinUI 3 native shell) to the local PC and `META-ROGALLY` as portable copies under `C:\Portable Programs\Work Review`, and verify the portable process path, version, and running state on both machines.
- Completed: install Android `0.1.1` (`versionCode 2`) on the connected `vermeer` device with `adb install -r` and relaunch its main activity without clearing application data.
- Completed: replace the rejected Work Ledger treatment with the cold-dark Dark Current desktop direction.
- Completed: apply the same dark-only direction to Android navigation, overview, timeline, settings, and blocked-app management.
- Completed: add resilient app identity resolution and icon rendering for Android.
- Completed: overhaul application branding and in-app icons to Microsoft Fluent System Icons (`microsoft/fluentui-system-icons`), including multi-res PNGs/ICO/ICNS, Android vector launcher icon, window controls, sidebar navigation, timeline controls, summary, settings tabs, toast notifications, stats cards, and fallback monogram SVGs.
- Completed: run Android unit tests and APK assembly; run focused desktop tests (`23/23` pass), Vite production build, and the Tauri Windows release build.

# Build / Run / Test

## Desktop

### Windows production frontend (desktop-winui)

```powershell
cargo build --release -p work-review --features ffi --lib
dotnet publish desktop-winui/WorkReview.csproj -c Release -p:Platform=x64 -o desktop-winui/publish
```

- The build requires `desktop-winui/Engine/work_review_engine.dll` (copied from `target/release/`).
- Deploy by stopping `Work_Review.exe`, replacing the contents of `C:\Portable Programs\Work Review\` with `desktop-winui/publish/*`, then starting the portable EXE and verifying process path + product version. The EXE name stays `Work_Review.exe` so registry autostart and shortcuts keep working; the new app rewrites the registry path itself when `auto_start` is on.
- Engine crash/stage diagnostics are written to `%TEMP%\workreview-ui.log`.

### Legacy Tauri shell (macOS/Linux)

```powershell
node --test
npm run build
npm run tauri:build
```

- Vite development server: `npm run dev -- --host 127.0.0.1 --port 5173`.
- Browser-only Vite rendering is not a valid desktop acceptance path because `App.svelte` initializes the Tauri webview window API at module startup; use the actual Tauri window for final visual checks.
- Windows release artifacts are under `target\release\` and `target\release\bundle\` because this repository uses a workspace-level Cargo target directory.
- Deploy the Windows portable build by stopping `Work_Review.exe`, replacing `C:\Portable Programs\Work Review\Work_Review.exe` with `target\release\Work_Review.exe`, then starting that portable EXE and verifying its process path and product version. The NSIS bundle is only a build artifact and is not the production update path.
- The focused Windows Fluent/UI suite is green (23/23). The repository-wide `node --test` reports 93 pass / 14 fail (legacy Lite-branch debt: Overview, avatar, AI/provider, update flows). Tests that assert on Rust source structure read `lib.rs` / `collection.rs` / `shell.rs` / `events.rs` after the split — update those paths when moving code again.

## Android

```powershell
Set-Location mobile-android
.\gradlew.bat testDebugUnitTest assembleDebug
```

- Debug APK: `mobile-android\app\build\outputs\apk\debug\app-debug.apk`.
- `mobile-android\local.properties` is machine-local and must remain untracked.
- Device installation should use `adb install -r` so application data and collection settings are retained.

# Durable Lessons

- WinUI 3 (WASDK 1.7, self-contained unpackaged): re-parenting an already-attached element subtree (`panel.Children.Add(rowFromAnotherPanel)`) can fail with REGDB_E_CLASSNOTREGISTERED ("没有检测到已安装的组件"); build cards directly into the target panel or insert into the existing body. NumberBox was replaced with TextBox numeric input as a precaution.
- `CalendarDatePicker.DateFormat` requires Windows.Globalization template tokens (`{day.integer}/{month.numeric}/{year.full}`); .NET-style `yyyy/M/d` throws formatTemplate.
- WinUI 3 XAML compiler pass1 crashes silently (exit 1, no diagnostics) on certain `tb:TaskbarIcon` attribute/property-element combinations — build tray menus in code-behind.
- The `get_recording_state` command returns a Rust tuple serialized as a JSON array `[is_recording, is_paused]`, not an object.
- Acquire the named single-instance mutex and activation event before starting the Rust engine; a losing process must never open the shared SQLite database or launch collection tasks. The FFI JSON parameters must use UTF-8 marshaling (`LPUTF8Str`), and hide-to-tray requires setting `WindowEventArgs.Handled` before hiding the window.

- The Android source is tracked on `codex/mobile-android-mvp`; seeing only empty source directories plus an APK on another branch does not mean the app must be reconstructed from bytecode.
- Root `.gitignore` must use `/data/`, not `data/`, or Android package folders named `data` are silently ignored.
- Android app labels alone are not enough for blocked-app UX: package visibility, fallback identity resolution, and the icon rendering path must all be verified together.
- Warm cream, brown/copper accents, and large serif editorial headings read as a generic generated-dashboard pattern in this product and were explicitly rejected; retain the cool graphite Dark Current palette and compact sans hierarchy.
- A Windows-looking WebView surface is not equivalent to a Windows-native desktop shell: keep the DWM backdrop, transparent WebView, caption controls, and WinUI token layer working together, and verify the actual release EXE rather than browser-only rendering.
- Svelte component-scoped dark rules can override global shell tokens late in the cascade. Keep route-specific WinUI overrides beside the component when necessary and validate hover/focus states in the release window.
- Do not infer the Windows deployment target from a running legacy copy. This product is intentionally operated as a portable application under `C:\Portable Programs`; `%LOCALAPPDATA%\Programs\Work Review` is a retired installation and must not be recreated by deployments.
- A GUI process started directly by Windows OpenSSH can exit when the SSH job closes. For the server, start the portable EXE in the logged-in console session through a one-time interactive scheduled task, delete that task immediately, and verify the process path again in a fresh SSH command.
