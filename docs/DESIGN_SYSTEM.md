# Design system

## Current status

Weyriva has an independent QtQuick shell, shared theme/state primitives, and
Weyriva-owned desktop, greeter, and lock QML in the repository. The current
visual iteration exists in source and has behavior-bearing static tests. Source
presence is not runtime acceptance: pointer input, keyboard navigation, secure
lock behavior, and installed lifecycle behavior have not yet been accepted on
XRY.

The source supports light and dark presentation. It does **not** currently
implement dynamic palette extraction from wallpaper pixels. Documents and UI
must not imply otherwise.

## Two-layer identity

Weyriva combines two separate design responsibilities:

- the Anthropic-inspired brand layer provides flat editorial color, irregular
  carriers, rounded hand-drawn marks, and deliberate asymmetry;
- the Apple-inspired behavior layer provides restrained hierarchy, immediate
  feedback, spatial continuity, focus clarity, and interruptible motion.

These are original project-owned implementations, not copied assets,
affiliations, or runtime dependencies.

### Brand layer

The primary visual grammar is:

- cactus `#BCD1CA` as the characteristic accent field;
- ivory `#FAF9F5` as the paper/carrier color;
- ink `#141413` as the primary mark and text color;
- flat, opaque fills;
- one irregular carrier or deliberately asymmetric composition;
- rounded, slightly uneven linework with generous negative space.

The existing flat cactus artwork is the reference. Avoid gradients, gloss,
glass-like decoration, photographic lighting, generic stock cards, perfect
geometric repetition, and dense cards nested inside cards.

Supporting colors may include clay `#D97757`, heather `#CBCADB`, oat `#E3DACC`,
olive `#788C5D`, sky `#6A9BCC`, fig `#C46686`, and coral `#EBCECE`. Components
consume semantic theme roles rather than scattering these literals.

### Behavior layer

Interactive UI follows these invariants:

- feedback begins on pointer-down or key activation;
- focus remains visibly distinct from hover and selection;
- entry and exit use the same source-anchored path;
- interrupted motion continues from its current visible value;
- hierarchy is expressed with spacing, type, and restrained surface contrast;
- reduced motion uses a cross-fade or static alternative;
- no enabled control is decorative or actionless.

## Surface architecture

Routes intentionally use two spatial families:

| Family | Routes | Placement |
|---|---|---|
| Focused work | launcher, wallpaper, settings | centered |
| Glanceable state | control center, calendar, notifications | top-right |

Focused surfaces prioritize browsing, selection, and deliberate changes.
Glanceable surfaces remain close to their bar triggers and dismiss along the
same path from which they entered.

The required behavior of each route is:

- Launcher filters real desktop entries and executes the selected entry.
- Control center uses a two-column layout of real controls with visible state.
- Calendar provides previous/next month actions and a navigable date grid.
- Notifications allow dismissal and expose an explicit empty state.
- Wallpaper presents visual choices and updates the selected path and related
  appearance state.
- Settings exposes explicit values and state-changing controls; future or
  unavailable functions are visibly disabled.
- Greeter and lock share the same visual family while retaining separate
  authentication and security responsibilities.

## Components and state

Every enabled control has an action, a visible label or accessible name, and
observable feedback. Core states are rest, hover, focus, pressed, selected,
disabled, loading, success, and error.

Pressed feedback is immediate. Focus uses a persistent high-contrast boundary.
Disabled controls do not respond and explain unavailable behavior where useful.
Errors remain visible long enough to understand and recover from them.

Controls must not rely on color alone for essential state. Text, shape, glyph,
or position also communicates selection, DND, authentication failure, and
other status.

## Typography, space, and shape

Use the platform sans-serif stack. Display text is compact and confident; body
text remains readable at increased scale; monospace is reserved for commands,
paths, identifiers, and diagnostics.

Spacing follows a compact four-pixel-derived rhythm. Large carriers may use
generous rounded or irregular silhouettes, while controls use a small,
consistent radius set. Deliberate asymmetry belongs in composition and artwork,
not in unpredictable control alignment.

## Artwork boundary

Artwork may sit behind or beside controls but never becomes a hit target,
carries required status text, obscures focus, or replaces semantic labels.
Background windows use an empty input mask and cannot capture interaction.

## Acceptance boundary

Static source tests may prove that routes, actions, theme roles, and reduced
motion branches exist. They do not prove rendering, input, animation quality,
authentication, or secure lock recovery.

Runtime and XRY evidence are recorded separately in [Testing](TESTING.md).
See also [Motion](MOTION.md), [Theming](THEMING.md), and
[Accessibility](ACCESSIBILITY.md).
