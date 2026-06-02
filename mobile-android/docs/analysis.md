# Desktop model analysis

## Reusable concepts

- The desktop project separates raw activity records from session/timeline views. Android keeps the same idea with `app_events` as raw events and `app_sessions` as derived sessions.
- The timeline/reporting concept is reusable: users need a chronological view and a per-app duration summary.
- Local-first storage remains the right default. The Android app uses Room over SQLite and avoids cloud upload.

## Android-specific rewrite

- Desktop active-window and title capture does not map cleanly to Android. Android uses `UsageStatsManager.queryEvents(start, end)` and package names.
- Screenshot, OCR, active window title, and desktop process/window APIs are not ported.
- Browser URL capture is out of scope for Android Lite because userscript-to-localhost logging is unreliable across mobile browsers and WebView implementations.
- Background behavior uses WorkManager and backfill instead of assuming a permanently running process.

## Not in first version

- Screenshot capture.
- OCR.
- AI summary.
- Cloud sync.
- Root/private database reading.
- Account system.
- Accessibility-based browser tracking, except as a possible later experiment.
