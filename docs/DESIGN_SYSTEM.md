# Design system

## Current status

Weyriva has an independent QtQuick shell, shared theme/state primitives, and
Weyriva-owned desktop, greeter, and lock QML in the repository. UI iteration 3
has independent source review, behavior-bearing tests, and an XRY preview of
the shell/greeter trees. That preview is not full input, lock, lifecycle, or
current all-Rust control-plane acceptance; XRY retains only the previously
deployed control-plane milestone.

The source supports light and dark presentation. It does **not** currently
implement dynamic palette extraction from wallpaper pixels. Documents and UI
must not imply otherwise.

## Two-layer identity

Weyriva combines two separate design responsibilities:

- the Anthropic-inspired brand layer provides flat editorial color, irregular
  composition, and rounded hand-drawn marks only in environmental art, brand
  moments, greeter/lock composition, and genuine empty states;
- the Apple-inspired behavior layer provides restrained hierarchy, immediate
  feedback, source-screen ownership, focus clarity, and source-specific
  interruptible motion.

These are original project-owned implementations, not copied assets,
affiliations, or runtime dependencies.

### Brand layer

The primary visual grammar is:

- cactus `#BCD1CA` as the characteristic accent field;
- ivory `#FAF9F5` as the paper/carrier color;
- ink `#141413` as the primary mark and text color;
- flat, opaque fills;
- at most one irregular brand carrier or deliberately asymmetric composition;
- rounded, slightly uneven linework with generous negative space.

The existing flat cactus artwork is the reference. Avoid gradients, gloss,
glass-like decoration, photographic lighting, generic stock cards, perfect
geometric repetition, dense cards nested inside cards, and a universal carrier
around functional panels.

Supporting colors may include clay `#D97757`, heather `#CBCADB`, oat `#E3DACC`,
olive `#788C5D`, sky `#6A9BCC`, fig `#C46686`, and coral `#EBCECE`. Components
consume semantic theme roles rather than scattering these literals.

### Behavior layer

Interactive UI follows these invariants:

- feedback begins on pointer-down or key activation;
- focus remains visibly distinct from hover and selection;
- entry and exit use the same source-anchored path;
- interrupted motion continues from its current visible value;
- each route belongs to exactly the screen whose control opened it;
- hierarchy is expressed with spacing, type, and restrained surface contrast;
- reduced motion uses a cross-fade or static alternative;
- no enabled control is decorative or actionless.

## Surface architecture

Routes intentionally use three spatial families:

| Family | Routes | Placement and structure |
|---|---|---|
| Command palette | launcher | centered search plus dense result rows |
| Utility popovers | control center, calendar, notifications | compact, top-anchored to distinct controls on the centered bar |
| Structured workspaces | wallpaper, settings | centered navigation/content regions |

Control center aligns to the bar's left utility source, calendar to the clock
near the center, and notifications to the right source. Each owning screen has
at most one active/focused host. Utility popovers retain their presentation
route while closing, so they dismiss toward the same trigger from which they
entered. Switching between them retargets from current position and size.

The required behavior of each route is:

- Launcher filters real desktop entries and executes the selected entry.
- Control center uses compact rows of real controls with visible state.
- Calendar provides previous/next month actions and a navigable date grid.
- Notifications allow dismissal and expose an explicit empty state.
- Wallpaper presents visual choices and updates the selected path and related
  appearance state.
- Settings exposes only explicit values and implemented state-changing
  controls. Future capabilities do not appear as dead rows.
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

Spacing follows a compact four-pixel-derived rhythm. Functional surfaces use a
small, consistent radius set and semantic boundaries. Irregular silhouettes
belong to brand composition and artwork, not panel geometry or unpredictable
control alignment.

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
