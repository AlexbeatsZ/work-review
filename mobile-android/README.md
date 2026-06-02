# Work Review Mobile

Android local app-usage recorder based on the desktop `work-review` data model, implemented as a native Android project rather than a Tauri Mobile port.

## Scope

- Records app usage with `UsageStatsManager`.
- Converts usage events into app sessions.
- Saves `app_events`, `app_sessions`, and Via browser `browser_events` in Room / SQLite.
- Shows a daily app ranking, app timeline, Via URL timeline, settings, and debug page.
- Receives Via userscript logs through `http://127.0.0.1:17890/log`.
- Keeps data local by default. No cloud upload, screenshot, OCR, AI summary, or root/private database access is implemented.

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

## Via userscript

1. In the app settings page, start the local receive service.
2. In Via, install `userscripts/via_work_review_logger.user.js`.
3. Browse ordinary `http` or `https` pages.
4. Check the app debug page for recent `browser_events`.

The first target endpoint is:

```text
http://127.0.0.1:17890/log
```

If Via or its WebView blocks requests to localhost, use the queued JSON from userscript local storage as a manual import/export fallback in a later phase. A deep-link fallback such as `workreview://log?...` is possible but may interrupt browsing.

## Exports

The settings page exports:

- `work_review_mobile.db`
- `app_sessions.csv`
- `browser_events.csv`

Files are written to the app external files export directory shown in the UI.

## Known limits

- UsageStats events are system-level estimates, not audit-grade exact logs.
- Browser URL logging only starts after the userscript is installed.
- The app does not read Via private history databases and does not attempt root-only access.
- Background collection may be delayed by Android battery and background execution policies; opening the app backfills available usage events.
