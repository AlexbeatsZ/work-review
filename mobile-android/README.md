# Work Review Mobile

Android local app-usage recorder based on the desktop `work-review` data model, implemented as a native Android project rather than a Tauri Mobile port.

## Scope

- Records app usage with `UsageStatsManager`.
- Converts usage events into app sessions.
- Saves `app_events`, `app_sessions`, and PC-compatible `activities` rows in Room / SQLite.
- Shows a daily app ranking, app timeline, settings, and debug page.
- Keeps data local by default. No browser history capture, cloud upload, screenshot, OCR, AI summary, or root/private database access is implemented.

## Build

```powershell
cd C:\Users\Meta\Project\Scripts\Rust\work-review\mobile-android
.\gradlew.bat assembleDebug
```

The debug APK is generated under `app\build\outputs\apk\debug\`.

If Gradle cannot locate the Android SDK, create `local.properties` with:

```properties
sdk.dir=C\:\\Users\\Meta\\AppData\\Local\\Android\\Sdk
```

## Usage Access

On first launch, open the permission card and enable usage access for `Work Review Mobile`. The app then backfills usage events from the last collected timestamp to the current time whenever it opens or when the WorkManager job runs.

## Exports

The settings page exports:

- `work_review_mobile.db`
- `app_sessions.csv`
- `activities.csv`

Files are written to the app external files export directory shown in the UI.

## Known limits

- UsageStats events are system-level estimates, not audit-grade exact logs.
- Browser URL logging is intentionally not included in Android Lite.
- The app does not read browser private history databases and does not attempt root-only access.
- Background collection may be delayed by Android battery and background execution policies; opening the app backfills available usage events.
