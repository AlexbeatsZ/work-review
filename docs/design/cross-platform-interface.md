# Windows Fluent desktop and Dark Current Android

## Product and audience

Work Review is a private activity stream for people who spend long periods at a computer and need to scan where their day went. The interface has one job: turn captured activity into a fast, legible sequence without feeling like a surveillance dashboard.

The desktop and mobile applications share information architecture and icon meaning, but they deliberately follow their host platforms. The Windows desktop is a native WinUI 3 / Windows App SDK application (`desktop-winui/`) backed by the shared Rust collection engine. Android remains a native Compose application using the compact Dark Current theme.

## Windows desktop architecture: WinUI 3 shell + Rust engine

- `desktop-winui/` is the production Windows frontend: C# / .NET 8, Windows App SDK 1.7, unpackaged self-contained portable deployment (no MSIX, no runtime install).
- `src-tauri` builds both the legacy Tauri shell (macOS/Linux) and, with the `ffi` feature, the `work_review_engine.dll` cdylib: `cargo build --release -p work-review --features ffi --lib`.
- The C# app P/Invokes `engine_start` / `engine_invoke` (JSON in / JSON out, method names identical to Tauri commands) and receives `screenshot-taken`, `recording-state-changed`, and `config-changed` through an event callback.
- Collection, storage, privacy, categorization, and data-directory behavior stay in Rust (`crates/core` + `src-tauri/src/collection.rs`), so the Windows portable build and the Tauri build share one engine and one SQLite database. Data directories, `config.json`, and `workreview.db` are unchanged by the frontend switch.
- App identity, autostart (registry value `Work Review`), tray, and single-instance behavior must keep working across frontend swaps; the WinUI app repairs a stale autostart path on startup when `auto_start` is enabled.

## Desktop direction: Windows 11 native Fluent

The desktop must feel like it belongs beside Windows Settings and modern inbox applications. It uses the same kinds of resources recommended by Microsoft's Windows design guidance:

- WinUI 3 Gallery for control anatomy, states, title-bar behavior, navigation, command surfaces, and settings patterns.
- Windows Design Kit proportions for spacing, type hierarchy, corner radii, and layer relationships.
- Microsoft Fluent System Icons SVG paths for cross-platform-safe icon rendering inside the Tauri WebView.
- DWM Mica for the real Windows 11 window backdrop, with an opaque WinUI dark fallback when the operating system cannot provide it.

This is not a CSS glass theme. The compositor owns the backdrop; the web layer stays quiet and supplies controls, content layers, and interaction states.

### Desktop tokens

- Mica fallback `#202020`: deepest canvas when DWM material is unavailable.
- Navigation pane `rgba(44, 44, 44, 0.82)`: translucent pane over DWM Mica.
- Layer `rgba(45, 45, 45, 0.92)`: grouped lists, settings sections, and summary cards.
- Layer alt `rgba(50, 50, 50, 0.88)`: setting rows and secondary surfaces.
- Stroke `rgba(255, 255, 255, 0.0837)`: WinUI dark-theme structural outline.
- Primary text `#FFFFFF`; secondary text `#D1D1D1`; tertiary text `#9D9D9D`.
- System-style accent `#60CDFF`; accent text `#99EBFF`; success `#6CCB5F`; destructive `#FF99A4`.

Controls use a 4 px radius; grouped layers and flyouts use 8 px. Large pill-shaped controls, luminous card borders, decorative gradients, and generic dashboard atmosphere are prohibited.

### Desktop type

- Page titles: `Segoe UI Variable Display`, semibold, 28 px.
- Interface and body: `Segoe UI Variable Text`, regular, 13–14 px.
- Times and durations: `Cascadia Mono` or Consolas, used only where stable numeric alignment helps scanning.

Hierarchy comes from the Windows type ramp, layer boundaries, alignment, and whitespace. It does not use display serifs, uppercase decorative kickers, or exaggerated tracking.

### Desktop structure

```text
┌─ icon · Work Review ─────────────────────────────  —  □  × ┐
│ NavigationView pane │ Timeline                    date  ↻ │
│                     │ Activity record · live time          │
│  ▌ Timeline         │ ┌ command/status strip ────────────┐ │
│    Settings         │ ├ 18:36  icon  App  Title     43s ┤ │
│                     │ ├ 18:35  icon  App  Title      6s ┤ │
│ ┌ recording · pause┐│ ├ 18:34  icon  App  Title     12m ┤ │
│ └──────────────────┘│ └──────────────────────────────────┘ │
│ ZH                  │                                      │
└─────────────────────┴──────────────────────────────────────┘
```

- The custom title bar is 32 px high and follows Windows caption-button geometry.
- The 240 px left pane behaves like a compact `NavigationView`: flat rows, a 3 px selection indicator, and restrained hover/pressed states.
- Recording state lives at the bottom of the navigation pane as an operational status row, not as branding.
- Page headers are compact and unboxed. Commands use WinUI button/input geometry.
- The timeline is one grouped layer. Rows highlight on hover instead of appearing as individually glowing cards.
- Settings use a horizontal Pivot for General, Privacy, and Storage, followed by one reading pane. This avoids a second competing vertical navigation rail.
- Flyouts and date pickers use a solid WinUI dark surface and system-style elevation.

### Signature: the activity signal rail

The only product-specific flourish is the thin accent activity rail. It connects real captured sessions and uses the Windows accent color, so it reads as part of the system while still making the product recognizable. It must not emit ambient glow or compete with content.

## Android direction: Dark Current

Android remains a native Jetpack Compose interface and does not imitate Windows controls. It keeps the established Dark Current tokens:

- Void `#080A0D` for the app canvas.
- Graphite `#0E1217` for navigation and quiet surfaces.
- Slate `#141A22` for rows and raised content.
- Edge `#252E3A` for separators.
- Frost `#F3F6FA` for primary text.
- Signal `#7AA2F7` for the current destination and primary actions.
- Mint `#43D6A2` for recording health.

The mobile app stays dark-only, edge-to-edge, compact, and content first. App identity remains icon + readable label, with the package name only as secondary technical context where it helps selection or diagnosis.

## Interaction and accessibility contract

- Every icon-only desktop control has a tooltip or accessible name; Android icons have content descriptions.
- Desktop keyboard focus uses a 2 px accent outline. Navigation and Pivot states remain visible without relying on hover.
- Primary and secondary text maintain WCAG AA contrast against the dark layers.
- Motion is limited to short WinUI hover/press feedback and recording state. Reduced-motion disables nonessential animation.
- Empty and error states explain the next action plainly.
- Destructive controls retain their existing confirmation behavior and destructive color.

## Non-regression boundaries

- Desktop: preserve Tauri window controls, recording pause/resume, timeline loading/paging, summary route, activity details, category edits, settings save, locale switching, system tray behavior, and the always-dark runtime.
- DWM material is progressive enhancement: Windows versions without the required backdrop attributes must continue with the dark fallback surface.
- Android: preserve Usage Access onboarding, background collection, session threshold, ignored packages, application icon/name resolution, export directory/times, manual export, data clearing, and debug status.
- Android application IDs, database schema, and stored preference keys must not change.
