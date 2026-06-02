# Known limitations

- Android apps cannot normally read Via's private history database. This project deliberately avoids root-only or sandbox-bypassing logic.
- Via userscript logging records only pages visited after installation.
- Some pages, internal browser pages, or highly restricted WebView contexts may not run the userscript.
- WebView or Via may block `fetch` or `sendBeacon` to `127.0.0.1`; the script queues failed logs in `localStorage` for retry.
- App usage sessions are derived from UsageStats events. Missing events, lock-screen transitions, and OEM battery policies can reduce precision.
- The local HTTP service is opt-in and should be started from settings only when Via logging is needed.
