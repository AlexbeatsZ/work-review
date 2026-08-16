# Dark Current cross-platform interface

## Product and audience

Work Review is a private activity stream for people who spend long periods at a computer and need to scan where their day went. The interface has one job: turn captured activity into a fast, legible sequence without feeling like a surveillance dashboard.

The previous Work Ledger direction is retired. Its brown surfaces, paper metaphor, serif headings, and editorial ornament made a compact utility feel old and visually heavy. The product now uses a dark-only, cool-neutral visual system.

## Visual direction

### Palette

- Void `#080A0D`: window canvas and deepest background.
- Graphite `#0E1217`: sidebar, window chrome, and quiet surfaces.
- Slate `#141A22`: rows, controls, and raised content.
- Edge `#252E3A`: structural borders and separators.
- Frost `#F3F6FA`: primary text.
- Signal `#7AA2F7`: current route, time position, focus, and primary actions.

Supporting text uses blue-gray `#8A96A8`; recording health uses mint `#43D6A2`; destructive actions use coral `#FF7B72`. Brown, copper, cream, warm paper, and serif typography are prohibited.

### Type

- Page identity and body: platform UI sans (`Segoe UI Variable`, `PingFang SC`, `Noto Sans CJK SC`, platform sans).
- Totals and section headings: the same UI family at a firmer weight; no display serif.
- Times, durations, package names, and state codes: `Bahnschrift`, `Cascadia Mono`, `SFMono-Regular`, platform monospace.

The hierarchy comes from weight, width, alignment, and space—not decorative font switching.

### Signature: the signal track

A thin blue signal track connects captured activity across desktop and Android. Its dots represent real session positions; the currently active or strongest point has a restrained halo. It is the only luminous element. No ambient blobs, fake paper rules, ornamental gradients, or unrelated glow effects are allowed.

Motion is limited to the live recording pulse and a short row hover/press response. Reduced-motion disables the pulse and transitions.

## Desktop structure

```text
┌──── command rail ────┬──────────────── activity stream ────────────────┐
│ Work Review          │ Timeline                         date · refresh │
│ ● Recording          │ Today · 18 sessions · 08:14—16:42             │
│                      ├─────────────────────────────────────────────────┤
│ ▌ Timeline           │ 16:32  ●  [icon] Application     08m           │
│   Settings           │        │  Window or task title                  │
│                      │ 16:08  ●  [icon] Application     21m           │
│                      │        │  Window or task title                  │
│ ZH                   │                                             │
└──────────────────────┴─────────────────────────────────────────────────┘
```

- The command rail is compact and visually quieter than the activity stream.
- Window chrome merges with the canvas rather than appearing as a separate bar.
- The page header is compact; the content starts near the top instead of presenting an oversized title.
- The date summary is one low-profile status band, not a KPI or editorial masthead.
- Activity rows are dense, flat, and separated by edges. Rounded containers are used only where grouping is necessary.
- Settings use a compact category rail and a single graphite reading pane. Avoid nested cards.

## Android structure

```text
┌ WORK REVIEW                         ● REC ┐
│ 今日活动                                     │
│ 06:24                                      │
│ ‹               8 月 16 日              ›  │
├─────────────────────────────────────────────┤
│ 应用排行                                     │
│ [icon] Claude                         1:42  │
│ [icon] Obsidian                       1:08  │
│                                             │
├  总览      时间线       设置       状态  ────┤
```

- The app is dark-only and edge-to-edge; it does not follow the device light theme.
- Overview opens with a compact total-time block and immediately exposes useful rows.
- Bottom navigation is flat with a top edge; selected state is color and weight, not a large pill.
- Timeline rows show icon + readable label with time data aligned in a stable column.
- Package names remain secondary and are shown only where identity/debugging benefits.
- Settings use stacked graphite sections with subtle edges and direct labels.
- Blocked apps open in a near-full-height dark sheet with search, icon + label, package secondary, a clear check state, and a persistent Save action.

## Interaction and accessibility contract

- Minimum target: 44 px desktop and 48 dp Android.
- Icon-only controls have labels/content descriptions.
- Desktop focus uses a visible Signal outline.
- Frost/blue-gray text maintains WCAG AA contrast against Void, Graphite, and Slate.
- Empty and error states explain the next action plainly.
- Destructive controls stay coral and retain existing confirmation behavior.
- Reduced-motion and Android animation settings are respected.

## Non-regression boundaries

- Desktop: preserve Tauri window controls, recording pause/resume, timeline loading/paging, summary route, activity details, category edits, settings save, locale switching, and the always-dark runtime.
- Android: preserve Usage Access onboarding, background collection, session threshold, ignored packages, application icon/name resolution, export directory/times, manual export, data clearing, and debug status.
- Android application IDs and stored preference keys must not change.
