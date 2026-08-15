# Work Ledger cross-platform interface

## Product and audience

Work Review is a private work-rhythm ledger for people who want to understand where their day went without turning the app into a surveillance dashboard. The interface's single job is to make recorded time readable and controllable.

The direction borrows Claude's calm, natural, content-first character: warm neutral surfaces, articulate typography, plain language, and restrained controls. It does not copy Anthropic logos, illustrations, exact layouts, or brand assets.

## Visual direction

### Palette

- Paper `#F7F3EC`: main light canvas.
- Ink `#292521`: primary text and high-contrast controls.
- Ash `#69625B`: secondary copy.
- Rule `#D9D0C5`: dividers and structural boundaries.
- Copper `#C5663D`: the one strong accent, used for time markers and primary actions.
- Sage `#65705C`: recording/healthy status.

Dark mode uses the same material idea: roasted paper (`#201D1A`), warm ink (`#F3ECE2`), muted rule (`#49413A`), and slightly brighter copper (`#DF7E54`). No blue/indigo accents should remain in primary product surfaces.

### Type

- Display/section titles: restrained serif stack (`Iowan Old Style`, `Palatino Linotype`, `Noto Serif SC`, platform serif).
- Body/control copy: platform humanist sans (`Segoe UI Variable`, `PingFang SC`, `Noto Sans CJK SC`, platform sans).
- Times, durations, and package names: monospaced utility stack (`SFMono-Regular`, `Cascadia Mono`, `Consolas`, platform monospace).

Serif is reserved for page identity and major totals. Dense lists and controls remain sans-serif.

### Signature: the chronicle spine

A thin copper time spine connects the product's meaning across platforms. On desktop it is the structural rail for daily entries; on Android it appears in the date navigator and timeline. Dots, labels, and durations attach to the rail because they encode real time, not as decoration.

Motion is limited to recording status and a short entry reveal. Reduced-motion disables both.

## Desktop structure

```text
┌──────── ledger index ────────┬──────────── daily sheet ───────────────────┐
│ Work Review                  │ Timeline                         [date] [↻] │
│ private work ledger          │ A readable account of the day             │
│                              ├─────────────────────────────────────────────┤
│ ● recording                  │ date stamp · record count · covered hours  │
│                              │                                             │
│ │ Timeline                   │ 09:42 ●  app + category   title   duration  │
│   Settings                   │       │                                     │
│                              │ 09:18 ●  app + category   title   duration  │
│ locale                       │       │                                     │
└──────────────────────────────┴─────────────────────────────────────────────┘
```

- The sidebar is a quiet index, not a floating card stack.
- The main sheet owns scrolling; window chrome stays visually subordinate.
- The daily summary is a ruled masthead, not a gradient KPI card.
- Featured screenshots remain available but sit inside the same ledger rhythm.
- Settings use a left category index and a single reading column; avoid nested glass cards.

## Android structure

```text
┌ Work Review                         ● ┐
│ 今日记录                               │
│ 6 小时 24 分                         │
│ ──‹── 8 月 16 日 ──›──                │
│                                       │
│ 使用排行                               │
│ [icon] Claude            1h 42m       │
│ [icon] Obsidian          1h 08m       │
│                                       │
├  总览     时间线      设置      状态  ┤
└───────────────────────────────────────┘
```

- Use edge-to-edge Compose layout with safe insets and a warm paper canvas.
- Bottom navigation has real icons and short Chinese labels; no initial-letter placeholders.
- Overview prioritizes the total time and app identity.
- Timeline rows always show icon + label; the package name is shown only where it aids debugging.
- Settings are grouped as plain ledger sections with hairline rules.
- Blocked apps open in a full-height sheet with search, icon + label rows, package name secondary, clear selected state, and a persistent Save action.
- If an icon cannot be loaded, show a deterministic monogram tile. If a label cannot be loaded, use a readable package-derived name and keep the raw package below it.

## Interaction and accessibility contract

- Minimum touch target: 44 px desktop controls, 48 dp Android controls.
- Every icon-only control has a label/content description.
- Keyboard focus is visible on desktop.
- Text and controls meet WCAG AA contrast in both themes.
- Empty states state what is missing and the next action.
- Destructive data clearing stays visually distinct and requires existing confirmation behavior where present.
- `prefers-reduced-motion` and Android system animation preferences are respected.

## Non-regression boundaries

- Desktop: preserve Tauri window controls, recording pause/resume, timeline loading/paging, summary route, activity details, category edits, settings save, locale switching, and dark mode.
- Android: preserve Usage Access onboarding, background collection, session threshold, ignored packages, export directory/times, manual export, data clearing, and debug status.
- Android application IDs and stored preference keys must not change.
