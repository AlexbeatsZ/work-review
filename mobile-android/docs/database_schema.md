# Database schema

## app_events

- `id: Long`
- `ts: Long`
- `packageName: String`
- `className: String?`
- `eventType: Int`
- `source: String`

## app_sessions

- `id: Long`
- `startTs: Long`
- `endTs: Long`
- `durationMs: Long`
- `packageName: String`
- `appLabel: String?`
- `source: String`
- `confidence: Double`

## browser_events

- `id: Long`
- `ts: Long`
- `browserPackage: String`
- `url: String`
- `title: String?`
- `referrer: String?`
- `eventType: String`
- `durationMs: Long?`
- `source: String`

## user_preferences

- `key: String`
- `value: String`

Room schema export is enabled. Version 1 is the initial schema; future changes should add explicit migrations.
