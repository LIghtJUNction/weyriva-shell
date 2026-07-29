# Design system

Weyriva combines two clearly separated influences:

- the `apple-design` interaction discipline informs hierarchy, familiar
  controls, restrained material depth, immediate feedback, spatial continuity,
  keyboard behavior, and motion;
- the `anthropic-art` visual grammar informs project-owned wallpapers,
  login/lock artwork, and empty-state illustrations: opaque muted fields,
  irregular ivory carriers, and near-black hand-drawn marks.

These are design references, not endorsements. Weyriva is not affiliated with,
sponsored by, or presented as an Apple or Anthropic product.

## Responsibility boundary

Functional UI belongs to the interaction system. It must remain legible,
predictable, focusable, and operable without relying on the wallpaper.
Editorial artwork belongs behind or beside the controls. It must never become a
button, cover a hit target, carry required status text, or replace a semantic
icon or label.

The boundary is deliberate:

| Layer | Allowed language | Not allowed |
| --- | --- | --- |
| Wallpaper and illustration | Flat accent field, irregular ivory carrier, gestural ink linework | Glass, gradients, photographic lighting, decorative UI controls |
| Shell surfaces | Semantic palette roles, restrained translucency, clear borders, system typography | Hand-drawn hit targets, text embedded in art, uncontrolled transparency |
| Interaction | Immediate pressed/focus state, source-anchored panels, symmetric dismissal | Decorative motion, surprise relocation, hover-only access |

## Brand primitives

The offline reference palette starts from:

| Token | Reference | Use |
| --- | --- | --- |
| `ink` | `#141413` | Primary dark surface, linework, light-mode text |
| `ivory` | `#FAF9F5` | Primary light surface, dark-mode text, illustration carrier |
| `cactus` | `#BCD1CA` | Default muted accent and wallpaper field |

Wallpaper-derived mode is the normal runtime source, so active colors may
change. Components use semantic roles rather than these literals. The exact
mapping and upstream generator are documented in [Theming](THEMING.md).

## Semantic color contract

Noctalia exposes 16 shell roles. Every Weyriva palette or generated theme must
produce a valid, opaque value for all of them:

| Role | Requirement |
| --- | --- |
| `primary` | Main action, active selection, focus emphasis |
| `on_primary` | Readable content on `primary` |
| `secondary` | Lower-priority accent; never indistinguishable from disabled state |
| `on_secondary` | Readable content on `secondary` |
| `tertiary` | Sparse supporting emphasis, not a third competing brand color |
| `on_tertiary` | Readable content on `tertiary` |
| `error` | Destructive/error state only |
| `on_error` | Readable content on `error` |
| `surface` | Main panel, settings, and shell background |
| `on_surface` | Primary text/icons on `surface` |
| `surface_variant` | Cards and secondary regions |
| `on_surface_variant` | Secondary text/icons on variant surfaces |
| `outline` | Borders, separators, focus reinforcement |
| `shadow` | Restrained depth cue; never the only boundary |
| `hover` | Pointer hover and keyboard-selection background |
| `on_hover` | Readable content on `hover` |

Text and essential icons target WCAG AA contrast: 4.5:1 for normal text and
3:1 for large text and non-text UI boundaries. Destructive actions require
both color and a label/icon; color alone is insufficient.

## Typography

Use system-resolved fonts through Fontconfig. The fixed profile may name Noto
Sans, but controls must tolerate distribution fallback. Do not ship text as
images.

- Body and controls: regular or medium system sans.
- Titles: weight and size provide hierarchy; avoid decorative display faces.
- Monospace content: terminal-resolved monospace, never forced into general UI.
- Keep labels concise and literal. Icon-only controls require an accessible
  name and tooltip where the engine supports one.

## Spacing, radius, and material

Use a 4 px base rhythm. Common gaps are 8, 12, 16, 24, and 32 px. Keep related
controls closer than unrelated groups.

Rounded geometry communicates grouping, not decoration. A 12–16 px panel/card
radius and smaller control radius are the normal range. Do not nest multiple
large rounded cards without a hierarchy reason.

Translucency is allowed only when text remains stable over every bundled
wallpaper. Prefer solid or soft material for dense panels, settings, the
launcher, calendar, and authentication. Blur and shadow are supporting depth
cues; borders and contrast must still define the surface when effects are
disabled.

## Component state matrix

Every actionable component must define all applicable states:

| State | Visual and behavioral expectation |
| --- | --- |
| Rest | Clear affordance and readable label/icon |
| Hover | Immediate background or outline change; no layout shift |
| Pressed | Immediate compression/color response before the action completes |
| Keyboard focus | Persistent, high-contrast ring independent of hover |
| Selected | Durable state distinct from transient hover |
| Disabled | Lower emphasis while retaining readable purpose; not clickable |
| Loading | Activity and label/status; do not silently ignore repeated input |
| Success | Confirm outcome without moving the initiating control |
| Error | Explain failure near the control and preserve a recovery path |

Pointer targets are at least 44×44 logical pixels for primary touch-like
actions. Dense desktop rows may be smaller only when the whole row is clickable
and keyboard focus remains clear. Adjacent destructive and routine actions need
enough separation to prevent accidental activation.

## Light and dark

Light and dark are first-class modes, not an inverted screenshot. Validate:

- surfaces, cards, popups, borders, text, icons, terminal ANSI colors, and
  selection colors;
- wallpaper and lock/login artwork independently;
- transparent material against both light and dark parts of each wallpaper;
- hover, focus, disabled, error, and destructive states;
- system tray and application icons that do not follow semantic roles.

Automatic mode uses Weyriva's fixed schedule, while the `theme_mode` control
provides a manual override. Exact behavior is in [Theming](THEMING.md).

## Artwork grammar

Project-owned Anthropic-inspired artwork uses exactly three visual layers:

1. a full-frame opaque muted accent field;
2. one large irregular ivory shape;
3. near-black gestural linework and solid marks.

Use one accent family, rounded uneven strokes, deliberate asymmetry, and at
least 10% breathing room around the focal cluster. Avoid gradients, cast
shadows, realistic perspective, glossy lighting, perfect geometry, and generic
stock-vector detail.

## Interaction requirements

- Every button and calendar control must work by pointer and keyboard.
- Enter/Space activates focused controls; Escape closes transient surfaces.
- Focus order follows visual order and never enters invisible content.
- Opening panels originate from their launcher/bar source where the engine
  supports it; closing returns attention to the source.
- Click feedback is immediate even when a backend operation is asynchronous.
- A failed network, plugin, calendar, or privilege action presents an error; it
  must not look like a dead button.

## Anti-patterns

- Artwork placed over controls or used as a control.
- Random transition effects in the fixed default profile.
- Tiny icon-only targets with no focus or accessible name.
- Calendar arrows, dates, settings rows, or plugin controls that only look
  interactive.
- Low-opacity text over an uncontrolled wallpaper.
- Multiple accent families competing in one surface.
- Spring or physics claims unsupported by the native engine.
- Separate visual systems for login, lock, and desktop.
- Branding claims that imply Apple or Anthropic endorsement.
