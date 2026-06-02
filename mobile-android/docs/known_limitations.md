# Known limitations

- Android Lite intentionally does not capture browser URLs or browser history.
- Android apps cannot normally read another app's private history database. This project deliberately avoids root-only or sandbox-bypassing logic.
- App usage sessions are derived from UsageStats events. Missing events, lock-screen transitions, and OEM battery policies can reduce precision.
