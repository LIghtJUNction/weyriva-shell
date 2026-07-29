# Accessibility

## Current status

The source includes focus and pressed-state styling, keyboard-capable QtQuick
controls, reduced-motion state, masked password fields, and an input-transparent
background. This is implementation evidence only. Keyboard order, semantics,
contrast, 200% text scale, assistive technology behavior, and real greeter/lock
operation have not yet been accepted on XRY.

## Keyboard and focus

- Every enabled pointer action must also be keyboard reachable.
- Tab order follows visible reading order.
- Arrow keys navigate spatial collections such as the calendar date grid.
- Enter or Space activates the focused control.
- Escape dismisses the topmost route and returns practical control to its
  source.
- Focus never enters hidden, disabled, decorative, or background content.
- Focus appearance remains distinct from hover, pressed, and selected state.
- Greeter and lock remain usable without a pointer.

Global Niri shortcuts accelerate common actions but do not replace navigation
inside a surface.

## Targets and feedback

Interactive controls provide a clear label or accessible name and an effective
target of at least 40 logical pixels per axis where layout permits. Press
feedback begins immediately. Disabled controls do not respond and expose why
they are unavailable when useful.

A visible control that has no action is a defect. Tests should enumerate
enabled controls and verify an action binding rather than checking only that a
button-shaped object exists.

## Contrast, color, and scale

- Normal text targets at least 4.5:1 contrast.
- Large text, focus indicators, and essential control boundaries target 3:1.
- State uses text, shape, glyph, or position in addition to hue.
- Light and dark modes are evaluated independently.
- At 200% text scale, primary actions remain available and content scrolls or
  wraps instead of clipping.
- Icon-only controls retain accessible names.

Dynamic wallpaper palette extraction is not implemented, so no
wallpaper-derived contrast support is claimed.

## Reduced motion

Reduced motion replaces travel and large scaling with a short cross-fade or
static state change. It preserves pressed, focus, selection, progress, success,
and error feedback. Secure lock coverage never waits for visual animation.

Reduced transparency and increased contrast are separate future capabilities;
their absence must not be hidden behind the reduced-motion switch.

## Surface-specific requirements

### Launcher

- Search has a name and visible focus.
- Results expose application names and activation.
- Filtering does not move focus into hidden delegates.
- Empty results are understandable.

### Control center

- The two-column controls expose current state and real actions.
- Toggles communicate on/off state without color alone.
- Launch and lock actions report failure instead of silently doing nothing.

### Calendar

- Previous and next month buttons are focusable and active.
- Each date cell exposes a full date and today/selected state.
- Keyboard movement follows the visual grid.

### Notifications

- Each notification exposes a dismissal action.
- The empty state is explicit.
- Arrival and dismissal do not flood announcements or steal unrelated focus.

### Wallpaper and settings

- Wallpaper choices expose selected state and descriptive labels.
- Settings values are explicit, not inferred only from button wording.
- Unimplemented settings are disabled and identified as unavailable.

### Greeter and lock

- Password entry is masked and named.
- Authentication errors remain visible and focus returns safely.
- Authentication progress does not freeze input.
- Every output remains securely covered while locked.
- No desktop frame appears during transitions.

The typed QML `status(): string` handler is diagnostic only and cannot establish
secure lock recovery.

## Evidence

Acceptance requires keyboard-only recordings, focus inspection, semantic-tree
inspection where supported, contrast measurements, 200% scale screenshots,
reduced-motion runs, and real greeter/lock attempts.

Record source, runtime, system, and XRY evidence separately. Source review alone
does not establish accessibility support.
