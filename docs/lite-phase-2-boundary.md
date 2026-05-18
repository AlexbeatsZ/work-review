# Work Review Lite Phase 2 Boundary

Lite Phase 2 keeps Work Review focused on local personal activity records.

## Kept Features

- Local timeline recording.
- Screenshot capture and screenshot review.
- Timeline and hourly/period summary pages.
- Manual notes and followups.
- Local work intent recognition from rules and recorded activity.
- Local storage, privacy rules, idle detection, screen lock handling, and database logic.

## Required Back-End Tasks

- `background_screenshot_task`
- `hourly_summary_task`
- `generate_and_save_summary`

## Required Work Intelligence APIs

Keep `work_intelligence` local analysis:

- `build_work_sessions`
- `analyze_intents`
- `classify_session`
- `extract_todos`

Keep Tauri commands:

- `get_work_sessions`
- `recognize_work_intents`
- `extract_todo_items`

## Manual Followups

Manual followups are a Lite feature and are not part of the desktop avatar.

- New code should use `manual_followups` / `ManualFollowupItem`.
- Existing `avatar_followups` data should be migrated for compatibility.
- New writes should not append to `avatar_followups`.

## Removed Surfaces

- Desktop avatar window, animation, input monitoring, and bubble prompts.
- Ask page and full assistant shell.
- Remote integrations: Telegram Bot, Feishu Bot, localhost API, Node Gateway.
- MCP server and skills engine.
- GitHub updater and automatic update checks.

## Front-End Surface

Only these primary UI surfaces remain:

- Timeline
- Summary
- Settings
- Sidebar
- Toast
- ConfirmDialog
