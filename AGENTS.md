# Goal

- Deliver a cohesive desktop and native Android Work Review interface inspired by Claude's calm, content-first product language without copying Anthropic assets.
- Fix Android blocked-app management so every resolvable app is presented as icon + human-readable name, with the package name only as secondary technical context.
- Preserve existing collection, privacy, export, timeline, categorization, and storage behavior.

# Current State

- Branch: `codex/lite-without-intent`, with the native Android project from `codex/mobile-android-mvp` integrated into the cross-platform product.
- Desktop: Tauri 2 + Svelte 4 + Tailwind; primary routes are timeline, summary, and settings.
- Android: native Kotlin/Jetpack Compose app with overview, timeline, settings, export, diagnostics, and a full-height blocked-app picker.
- Android app identity resolution now combines package-manager labels, decoded launcher icons, known labels, and a readable package-derived fallback. The package name is only secondary context in the blocked-app sheet.
- Design contract: [docs/design/cross-platform-interface.md](docs/design/cross-platform-interface.md).

# Active Work

- Completed: replace the cool glass-card desktop treatment with the Work Ledger direction.
- Completed: rebuild Android navigation, overview, timeline, settings, and blocked-app picker with the same design language adapted to a phone.
- Completed: add resilient app identity resolution and icon rendering for Android.
- Completed: run Android unit tests and APK assembly; run targeted desktop tests, Vite production build, Tauri release build, and visual QA on the actual release EXE.
- No known incomplete work for the cross-platform redesign. A physical Android device was not attached for runtime visual QA.

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
- The focused Work Ledger/UI suite is green. The repository-wide `node --test` currently reports 92 pass / 15 fail because legacy tests still require modules deliberately absent from the Lite branch (notably Overview, avatar, AI/provider, and update flows); treat those as baseline branch debt rather than regressions in this redesign.

## Android

```powershell
Set-Location mobile-android
.\gradlew.bat testDebugUnitTest assembleDebug
```

- Debug APK: `mobile-android\app\build\outputs\apk\debug\app-debug.apk`.
- `mobile-android\local.properties` is machine-local and must remain untracked.
- No ADB device or AVD was available during the redesign; Gradle compilation and unit tests are the current Android verification boundary.

# Durable Lessons

- The Android source is tracked on `codex/mobile-android-mvp`; seeing only empty source directories plus an APK on another branch does not mean the app must be reconstructed from bytecode.
- Root `.gitignore` must use `/data/`, not `data/`, or Android package folders named `data` are silently ignored.
- Android app labels alone are not enough for blocked-app UX: package visibility, fallback identity resolution, and the icon rendering path must all be verified together.
