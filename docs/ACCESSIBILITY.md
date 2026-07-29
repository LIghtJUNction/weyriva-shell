# Accessibility

Accessibility is a release gate, not a theme option. Weyriva must remain
operable when animations, transparency, precise pointer input, network
services, plugin backends, or optional system services are unavailable.

The current status is deliberately conservative: keyboard and pointer behavior
must be verified on XRY, and screen-reader semantics have not yet been accepted.

## Keyboard contract

Every visible action must be reachable and usable without a pointer:

- Tab and Shift+Tab move through focusable controls in visual order.
- Arrow keys navigate lists, grids, calendars, menus, and segmented controls
  according to their orientation.
- Enter or Space activates the focused control.
- Escape dismisses the topmost transient surface without triggering an action.
- Closing a panel returns focus or practical keyboard control to its source.
- Focus never enters hidden, disabled, clipped, or background content.

Global compositor shortcuts are documented in
[Developer experience](DEVELOPER_EXPERIENCE.md). They supplement, rather than
replace, keyboard navigation inside Noctalia surfaces.

## Focus and interaction

Keyboard focus must be persistent and distinguishable from pointer hover and
selection. The active focus ring targets at least 3:1 contrast against adjacent
colors. Focus must remain visible in light, dark, wallpaper-derived, and
high-contrast modes.

Controls provide immediate activation feedback. A backend failure must produce
a visible error or notification; a control that accepts input and does nothing
is a failed interaction even when its process remains healthy.

Primary pointer/touch-like targets are at least 44×44 logical pixels. Dense
desktop rows may be smaller only when the entire row is clickable and keyboard
focus remains unambiguous. Do not place destructive actions immediately beside
routine actions without spacing and semantic differentiation.

## Contrast and color

Normal text targets 4.5:1 contrast. Large text, focus indicators, control
boundaries, and essential icons target 3:1. Validate all 16 semantic palette
roles described in [Design system](DESIGN_SYSTEM.md), including hover and error
states.

Meaning is never color-only. DND, network state, battery state, errors,
selected dates, destructive actions, and recording/capture indicators need an
icon, label, shape, or position cue in addition to color.

Noctalia provides:

```toml
[accessibility]
ui_scale = 1.0
high_contrast = false
```

`high_contrast = true` stretches token contrast, strengthens outlines, and
forces dark surfaces toward pure black. It still requires rendered acceptance;
an algorithmic transform does not prove every plugin bitmap or tray icon is
readable. The behavior is pinned in
[`palette_transform.cpp`](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/palette_transform.cpp).

## Scale

`accessibility.ui_scale` scales panels and non-bar shell UI. It does not change
Wayland output scale, HiDPI buffer scale, `bar.scale`, or individual widget
scale. Noctalia validates it in the 0.5–2.5 range.

Login has its own Greeter output scale:

```toml
[output]
scale = 1.5
```

Desktop and Greeter scale must be tested separately. Greeter auto scale derives
from display geometry and may not match the desktop compositor.

## Reduced motion and transparency

The native engine has no unified system reduced-motion preference. Weyriva's
mapping is:

```toml
[shell.animation]
enabled = false

[wallpaper]
transition = ["fade"]
```

Use `transition = []` when no wallpaper cross-fade is acceptable. Wallpaper
timing is intentionally independent of the global animation toggle. See
[Motion](MOTION.md).

Noctalia also has no system reduced-transparency preference binding. Dense
surfaces must remain readable in solid material and when compositor blur is
unavailable. Do not use wallpaper detail as a surface boundary.

## Screen readers and semantics

Screen-reader compatibility is currently **unverified**. The native C++ Wayland
UI does not become accessible merely because controls have visible labels.
Release evidence must identify the accessibility protocol/tooling used, inspect
names, roles, states, value changes, focus order, and live notifications, and
record unsupported surfaces. Until that evidence exists, Weyriva must not claim
screen-reader support.

## Required interaction acceptance

### Bar and panels

- Activate every bar widget by pointer and keyboard.
- Verify hover, pressed, focus, selected, disabled, and error states.
- Confirm launcher, control center, clipboard, wallpaper, session, network,
  Bluetooth, volume, brightness, battery, tray, media, and notification
  actions produce an outcome or an explicit failure.

### Calendar

- Open from the clock and through the control center.
- Move by Tab and arrow keys.
- Activate previous/next month, today, date cells, account controls, refresh,
  and event rows.
- Verify selected/today/focused dates are distinguishable without color alone.
- Test empty, loading, offline, authentication-failed, and populated states.

### Settings

- Navigate every section and search by keyboard.
- Operate toggles, sliders, steppers, selects, lists, dialogs, and Apply/Cancel
  actions.
- Verify validation banners and privilege prompts receive focus.
- Confirm changing scale, contrast, theme, and animation gives immediate,
  reversible feedback.

### Plugins

- Exercise settings and every enabled entry kind.
- Confirm unavailable dependencies produce an error rather than a dead control.
- Verify plugin UI follows focus, contrast, scaling, and reduced-motion rules.

### Lock and login

- Lock by shortcut, IPC, idle, and suspend path.
- Authenticate with keyboard only and recover from an incorrect password.
- Confirm lock remains secure after a shell crash/restart.
- Navigate user, session, scheme, password, and power controls in Greeter.
- Test multiple monitors, HiDPI, portrait transform, and TTY2 recovery.

## Evidence

An accessibility result records:

```text
surface:
mode: light | dark | high-contrast
scale:
input: pointer | keyboard | assistive technology
action:
expected:
observed:
evidence: screenshot, video, journal excerpt, or test
status: pass | fail | blocked | not-run
risk:
```

Do not convert “not tested” into “supported.”
