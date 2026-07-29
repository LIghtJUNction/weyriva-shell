# Motion

Weyriva motion communicates state and spatial relationship. It is not a
decoration layer. The fixed profile uses native Noctalia animations with one
deterministic wallpaper cross-fade and does not claim spring controls the engine
does not expose.

## Interaction goals

The Apple-inspired interaction discipline requires:

- immediate pressed and focus feedback;
- panels opening from the control or edge that invoked them;
- enter and exit paths that are spatially symmetric;
- interruption or redirection from the currently displayed state;
- short, critically damped-feeling responses without decorative overshoot;
- a reduced-motion path that preserves state feedback without displacement.

These are Weyriva acceptance goals, not a claim that Noctalia exposes physical
spring parameters.

## Native controls

Noctalia exposes only global enable and duration scaling:

```toml
[shell.animation]
enabled = true
speed = 1.0
```

The validated `speed` range is 0.1–4.0. `enabled = false` snaps animations
managed by `AnimationManager` to their final value while still running
completion callbacks. The implementation is pinned in
[`animation_manager.cpp`](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/render/animation/animation_manager.cpp#L29-L108).

There is no public stiffness, damping, mass, per-component duration/easing, or
animation IPC. Component timing and easing remain upstream implementation
details. Documentation and UI must not describe them as configurable springs.

## Deterministic default

```toml
[shell.animation]
enabled = true
speed = 1.1

[wallpaper]
transition = ["fade"]
transition_duration = 400
edge_smoothness = 0.3
transition_on_startup = true
```

Theme palette changes use an upstream 400ms cross-fade. Weyriva matches that
duration for wallpaper changes. The profile does not use the upstream random
pool of fade, wipe, disc, stripes, zoom, and honeycomb effects.

Expected response classes:

| Interaction | Expected behavior |
| --- | --- |
| Button press | Immediate visual response; action may complete asynchronously |
| Tooltip/hover | Fast reveal with no movement of the target |
| Panel open/close | Source-anchored, symmetric, interruptible where native |
| Theme change | 400ms color cross-fade |
| Wallpaper change | 400ms image cross-fade |
| Lock | Immediate secure acquisition; visual polish never delays security |
| Error | Stable surface with visible error, not a shake-only signal |

## Wallpaper exception

Wallpaper transitions use Noctalia's `animateTimer`. They intentionally ignore
both global animation speed and the animation enabled switch:
[`wallpaper.cpp`](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/shell/wallpaper/wallpaper.cpp#L1500-L1514).

Therefore `shell.animation.enabled = false` is not a complete reduced-motion
setting by itself.

## Reduced motion

The Weyriva reduced-motion mapping is:

```toml
[shell.animation]
enabled = false

[wallpaper]
transition = ["fade"]
transition_duration = 400
```

This removes native shell displacement while retaining a short no-displacement
wallpaper cross-fade. For users who require no temporal transition at all:

```toml
[wallpaper]
transition = []
```

Noctalia has no system portal or desktop preference binding that switches these
two settings as one transaction, and Greeter has no matching public animation
configuration. A future Weyriva accessibility control must update both settings
and verify the login surface separately; until then reduced motion is a
documented profile policy rather than a proven global OS preference.

## Reduced transparency

Motion and transparency are independent. The fixed profile should prefer solid
or soft material for dense content. Noctalia does not expose a system
reduced-transparency portal binding. Accessibility acceptance must therefore
verify readability with blur/translucency disabled or unavailable rather than
assuming compositor effects.

## Acceptance

For every launcher, bar menu, settings dialog, calendar transition, plugin
panel, notification, session panel, and lock surface:

1. activate it by pointer and keyboard;
2. interrupt or reverse it during motion where the engine supports that;
3. confirm no dead period blocks input;
4. repeat with `shell.animation.enabled = false`;
5. repeat wallpaper changes with `transition = ["fade"]` and `transition = []`;
6. inspect light and dark modes and multiple output scales;
7. record any fixed upstream animation that cannot be reduced.

Visual review is required on XRY. Source inspection alone cannot establish that
a rendered transition is smooth, correctly anchored, or accessible.
