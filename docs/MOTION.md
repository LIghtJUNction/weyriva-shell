# Motion

## Current status

The repository contains immediate button press feedback, source-specific route
motion, and reduced-motion branches in the QtQuick source. Static tests cover
their behavior-bearing structure. Interruption, input behavior, and frame
pacing remain unverified on XRY.

Motion never proves that an action completed, and it must never delay secure
lock acquisition.

## Behavior contract

Weyriva uses restrained, physical behavior:

- pointer-down feedback is immediate;
- a surface enters from its source and exits along the same path;
- retargeting starts from the current visible value, not a reset keyframe;
- input remains live during animation;
- hierarchy changes are subtle and do not bounce for decoration;
- authentication and error transitions prioritize clarity over flourish.

For direct manipulation, content tracks the pointer one-to-one and retains the
grab offset. Momentum is used only when supplied by the gesture.

## Spatial origins

| Surface | Origin and path |
|---|---|
| Launcher | centered materialization; centered dismissal |
| Wallpaper | centered materialization; centered dismissal |
| Settings | centered materialization; centered dismissal |
| Control center | top-right/bar source; reverse to the same source |
| Calendar | clock source at top-right; reverse to the same source |
| Notifications | notification/bar source at top-right; reverse to it |
| Greeter and lock | restrained state cross-fade; no exposed desktop frame |

Opening one route while another is visible retargets from current opacity,
scale, and position. It must not snap to a hidden initial value before moving.

## Component response

- Buttons and tiles compress or change color while pressed, before release.
- Focus appearance is stable and not animated away.
- Selection transitions retain an explicit selected state.
- Notification dismissal is visible and does not steal unrelated focus.
- Wallpaper changes avoid abrupt full-screen brightness flashes.
- Loading and errors hold layout steady instead of replacing the whole panel.

Ordinary surface changes target a short, critically damped response. Overshoot
is not used for menus, settings, calendar navigation, errors, greeter, or lock.

## Reduced motion

Reduced motion is an equivalent behavior path:

- travel and large scaling become a short cross-fade or immediate state change;
- pressed, focus, selection, progress, success, and error feedback remain;
- decorative loops and oscillation stop;
- wallpaper changes avoid abrupt luminance flashes;
- lock coverage is established without waiting for animation.

Implementation must contain an explicit reduced-motion branch. Merely setting a
duration to a smaller nonzero value is insufficient when travel or scaling can
still be vestibularly disruptive.

Reduced transparency and increased contrast are separate preferences; neither
is inferred from reduced motion.

## Performance

Prefer transform and opacity animation, avoid per-frame layout churn, coalesce
high-frequency updates, and stop hidden timers. Never discard pointer or
keyboard input to conceal poor frame pacing.

## Acceptance

For every route, record:

1. immediate pressed feedback;
2. correct entry origin;
3. symmetric exit;
4. interruption and reversal from the current value;
5. pointer and keyboard agreement;
6. reduced-motion equivalence;
7. stable final state and action result;
8. frame pacing on target hardware.

Source inspection proves only the presence of behavior-bearing branches.
Screenshots do not prove motion, input, or interruption. XRY acceptance remains
pending until the matrix above is exercised against the installed revision.
