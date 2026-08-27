# Goal

- Deliver a cohesive Windows-native desktop and dark-only native Android Work Review interface using Microsoft Fluent design language without copying Windows application assets.
- Fix Android blocked-app management so every resolvable app is presented as icon + human-readable name, with the package name only as secondary technical context.
- Preserve existing collection, privacy, export, timeline, categorization, and storage behavior.

# Current State

- Branch: `codex/lite-without-intent`, with the native Android project from `codex/mobile-android-mvp` integrated into the cross-platform product.
- Desktop: Tauri 2 + Svelte 4 + Tailwind; primary routes are timeline, summary, and settings.
- Android: native Kotlin/Jetpack Compose app with overview, timeline, settings, export, diagnostics, and a full-height blocked-app picker.
- Android app identity resolution now combines package-manager labels, decoded launcher icons, known labels, and a readable package-derived fallback. The package name is only secondary context in the blocked-app sheet.
- Desktop release `1.0.49` uses the Windows Fluent direction: a real Windows 11 DWM Mica backdrop, rounded system window treatment, Segoe UI Variable typography, NavigationView-style sidebar, Pivot-style settings navigation, compact grouped lists, and Microsoft Fluent System Icons.
- Android release `0.1.1` retains the touch-native Dark Current direction: cool graphite surfaces, blue-gray text, restrained blue signals, sans-serif typography, and dense content-first layouts.
- App and UI icons use Microsoft Fluent System Icons (`microsoft/fluentui-system-icons`) across desktop, web, Android launcher, and system tray.
- Design contract: [docs/design/cross-platform-interface.md](docs/design/cross-platform-interface.md).

# Active Work

- Completed: replace the desktop Dark Current shell with a Windows 11 Fluent shell while preserving collection, storage, timeline, summary, settings, and privacy behavior.
- Completed: enable the native DWM system backdrop and rounded corners, with transparent WebView surfaces and a deterministic Mica fallback for unsupported sessions.
- Completed: verify the real release EXE on the timeline, summary, and settings routes; the old glowing pill cards are replaced by compact grouped-list rows.
- Completed: replace the rejected Work Ledger treatment with the cold-dark Dark Current desktop direction.
- Completed: apply the same dark-only direction to Android navigation, overview, timeline, settings, and blocked-app management.
- Completed: add resilient app identity resolution and icon rendering for Android.
- Completed: overhaul application branding and in-app icons to Microsoft Fluent System Icons (`microsoft/fluentui-system-icons`), including multi-res PNGs/ICO/ICNS, Android vector launcher icon, window controls, sidebar navigation, timeline controls, summary, settings tabs, toast notifications, stats cards, and fallback monogram SVGs.
- Completed: run Android unit tests and APK assembly; run focused desktop tests (`23/23` pass), Vite production build, and the Tauri Windows release build.

# Build / Run / Test

## Desktop

```powershell
node --test
npm run build
npm run tauri:build
```

- Vite development server: `npm run dev -- --host 127.0.0.1 --port 5173`.
- Browser-only Vite rendering is not a valid desktop acceptance path because `App.svelte` initializes the Tauri webview window API at module startup; use the actual Tauri window for final visual checks.
- Windows release artifacts are under `target\release\` and `target\release\bundle\` because this repository uses a workspace-level Cargo target directory.
- The focused Windows Fluent/UI suite is green (23/23). The repository-wide `node --test` reports 93 pass / 14 fail because legacy tests still require modules deliberately absent from the Lite branch (notably Overview, avatar, AI/provider, and update flows); treat those as baseline branch debt rather than redesign regressions.

## Android

```powershell
Set-Location mobile-android
.\gradlew.bat testDebugUnitTest assembleDebug
```

- Debug APK: `mobile-android\app\build\outputs\apk\debug\app-debug.apk`.
- `mobile-android\local.properties` is machine-local and must remain untracked.
- Device installation should use `adb install -r` so application data and collection settings are retained.

# Durable Lessons

- The Android source is tracked on `codex/mobile-android-mvp`; seeing only empty source directories plus an APK on another branch does not mean the app must be reconstructed from bytecode.
- Root `.gitignore` must use `/data/`, not `data/`, or Android package folders named `data` are silently ignored.
- Android app labels alone are not enough for blocked-app UX: package visibility, fallback identity resolution, and the icon rendering path must all be verified together.
- Warm cream, brown/copper accents, and large serif editorial headings read as a generic generated-dashboard pattern in this product and were explicitly rejected; retain the cool graphite Dark Current palette and compact sans hierarchy.
- A Windows-looking WebView surface is not equivalent to a Windows-native desktop shell: keep the DWM backdrop, transparent WebView, caption controls, and WinUI token layer working together, and verify the actual release EXE rather than browser-only rendering.
- Svelte component-scoped dark rules can override global shell tokens late in the cascade. Keep route-specific WinUI overrides beside the component when necessary and validate hover/focus states in the release window.
