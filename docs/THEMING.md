# Theming

## Current status

The QtQuick source currently provides project-owned semantic colors, light and
dark presentation, reduced-motion state, and packaged light/dark cactus
wallpapers. Wallpaper selection can update shell state; persistence and
installed behavior must be verified from the final source and on XRY.

Dynamic palette extraction from wallpaper pixels is **not implemented**.
Automatic time/system theme resolution, reduced-transparency mode, increased
contrast mode, and a stable public theme IPC must not be claimed unless later
source and runtime evidence prove them.

## Fixed palette

The identity anchors are:

| Role | Light anchor |
|---|---|
| Ink / foreground | `#141413` |
| Ivory / surface | `#FAF9F5` |
| Cactus / carrier | `#BCD1CA` |
| Clay / attention | `#D97757` |

Dark mode is designed separately rather than mechanically inverted. Both modes
retain readable foregrounds, explicit focus, recognizable cactus identity, and
safe greeter/lock contrast.

Runtime components consume shared semantic roles such as background, surface,
alternate surface, foreground, muted, carrier, separator, focus, selected,
pressed, disabled, and error. New controls must not invent independent local
palettes.

## Flat visual rule

Theme surfaces use opaque, flat color. QML gradients, glossy highlights,
decorative glass, and stock-card shadow stacks are outside the visual system.
Depth comes from composition, spacing, boundaries, and restrained contrast.

Irregular carriers and hand-drawn marks may add brand character without
changing control geometry or input behavior.

## Wallpaper selection

The shipped wallpaper route presents explicit visual choices. A selection must:

1. update the active wallpaper path;
2. update the related light/dark appearance state when the choice defines it;
3. expose which choice is selected;
4. produce an explicit failure if the asset cannot be applied.

This deterministic selection is distinct from dynamic color extraction. The
current product uses the fixed Weyriva palette even when a wallpaper changes.

## State and persistence

Configured state and currently rendered state are separate concepts. Until
persistence is implemented and tested, documentation must describe changes as
shell state rather than durable user configuration.

Invalid state must fall back to a readable packaged theme. A theme error cannot
produce a blank shell or prevent lock coverage.

## Greeter and lock

Greeter and lock use the same cactus/ivory/ink visual family as the desktop.
The greeter reads only system-visible resources before authentication. The lock
may use authenticated user appearance, but secure coverage and legibility take
priority over theme continuity.

The typed native QML `status(): string` handler reports shell status for
diagnostics. It is not proof that a secure `WlSessionLock` can be reacquired
after a crash or restart.

## Accessibility

Light and dark modes require visible keyboard focus and adequate text/control
contrast. State is never color-only. Theme acceptance includes increased text
scale and reduced motion even when advanced contrast/transparency preferences
are not yet implemented.

## Acceptance

Current source-level checks should verify:

- shared semantic theme roles;
- light and dark branches;
- absence of QML gradients;
- explicit wallpaper path and appearance updates;
- shared desktop/greeter/lock identity;
- reduced-motion branches;
- deterministic fallback values.

Runtime and XRY acceptance additionally require rendering both modes, applying
each wallpaper, exercising input, restarting the shell, and checking greeter
and lock continuity. No such acceptance is inferred from source tests.
