# Theming

Weyriva normally derives its shell palette from the active wallpaper and keeps
the custom ink/ivory/cactus palette as an offline reference and explicit
fallback selection. Both light and dark variants are required, and login, lock,
and desktop must resolve to the same visual system.

Semantics in this document are pinned to:

- Noctalia source
  [`cebcc62284a42620ebb3518b3243665b43c11a96`](https://github.com/noctalia-dev/noctalia/tree/cebcc62284a42620ebb3518b3243665b43c11a96);
- Noctalia documentation
  [`f88820cc90170ceb212efdea87711802ebaca1c9`](https://github.com/noctalia-dev/noctalia-docs/tree/f88820cc90170ceb212efdea87711802ebaca1c9);
- Noctalia Greeter
  [`d6275cbcb5b9acae2348bed16e358aa6c2cf8188`](https://github.com/noctalia-dev/noctalia-greeter/tree/d6275cbcb5b9acae2348bed16e358aa6c2cf8188).

Refresh these pins deliberately when the packaged engine changes.

## Fixed profile

The intended zero-configuration runtime policy is:

```toml
[theme]
mode = "auto"
source = "wallpaper"
wallpaper_scheme = "soft"
custom_palette = "Weyriva"

[location]
custom_schedule = true
sunrise = "07:00"
sunset = "19:00"
```

`source` accepts exactly `builtin`, `wallpaper`, `community`, or `custom`.
`mode` accepts `dark`, `light`, or `auto`. The complete upstream schema is
defined in
[`config_types.h`](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/config/config_types.h#L1331-L1443).
`custom_palette` is only the selection used when `source = "custom"`; Noctalia
does not automatically fall back from a failed wallpaper generator to that
file. The explicit offline recovery command is
`weyriva shell msg color-scheme-set custom Weyriva`.

The fixed day/night schedule is intentionally independent of network and
geographic location. Users who want local astronomical sunrise/sunset maintain
that policy in a fork; the distributed installer does not ask. Noctalia's
coordinate and schedule precedence is pinned in the
[location reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/services/location.mdx).

## Wallpaper extraction pipeline

When `source = "wallpaper"`, changing the default wallpaper triggers:

```text
wallpaper path
→ decode
→ resize to 112×112 RGB
→ palette generation
→ dark and light token maps
→ terminal token synthesis
→ optional pure-black/high-contrast transform
→ live shell palette transition
→ templates and hooks
→ optional Greeter appearance sync
```

For Material schemes, Noctalia quantizes pixels with Wu and a deterministic
WSMeans path, ranks a colorful seed, and builds HCT/Material 3 dynamic tokens.
For `vibrant`, `faithful`, `soft`, `dysfunctional`, and `muted`, it uses a
deterministic Lab/HSL custom path. Weyriva chooses `soft` because it stays close
to the source image while reducing saturation enough to keep shell surfaces
calm.

Source evidence:

- [112×112 image path](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/image_loader.cpp#L257-L294)
- [Material extraction](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/m3_schemes.cpp)
- [custom generators](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/custom_schemes.cpp)
- [resolve, cache, and transition](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/theme_service.cpp#L403-L549)

The generated result is cached by wallpaper path, file modification time, and
scheme. Editing a file at the same path invalidates the cache when its mtime
changes.

Noctalia cannot keep selected generated tokens fixed while dynamically
extracting only one accent. Weyriva therefore does not claim partial-token
dynamic extraction. A strict ink/ivory surface plus wallpaper-derived accent
would require a new palette transform in code and a separate review.

## Wallpaper policy

The fixed profile uses a single deterministic cross-fade:

```toml
[wallpaper]
enabled = true
fill_mode = "crop"
transition = ["fade"]
transition_duration = 400
edge_smoothness = 0.3
transition_on_startup = true
directory = "~/.local/share/weyriva/wallpapers"
directory_light = "~/.local/share/weyriva/wallpapers/light"
directory_dark = "~/.local/share/weyriva/wallpapers/dark"

[wallpaper.default]
path = "~/.local/share/weyriva/wallpapers/light/weyriva-cactus.png"
```

Noctalia supports `center`, `crop`, `fit`, `stretch`, `repeat`, and `span`, plus
theme-aware and per-monitor directories. See the pinned
[wallpaper reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/desktop/wallpaper.mdx).

## Complete custom palette

The fallback `palettes/Weyriva.json` must contain `dark` and `light`. Each mode
must define all 16 UI roles:

```text
mPrimary, mOnPrimary, mSecondary, mOnSecondary,
mTertiary, mOnTertiary, mError, mOnError,
mSurface, mOnSurface, mSurfaceVariant, mOnSurfaceVariant,
mOutline, mShadow, mHover, mOnHover
```

Each mode also needs a complete terminal object:

```json
{
  "terminal": {
    "background": "#141413",
    "foreground": "#FAF9F5",
    "cursor": "#FAF9F5",
    "cursorText": "#141413",
    "selectionBg": "#BCD1CA",
    "selectionFg": "#141413",
    "normal": {
      "black": "#141413",
      "red": "#D19A94",
      "green": "#BCD1CA",
      "yellow": "#D8C58F",
      "blue": "#8FAFC3",
      "magenta": "#B9A7C9",
      "cyan": "#8FBDB4",
      "white": "#FAF9F5"
    },
    "bright": {
      "black": "#5B5B57",
      "red": "#E3ABA5",
      "green": "#CEE0DA",
      "yellow": "#E8D7A2",
      "blue": "#A7C4D5",
      "magenta": "#CCBBDD",
      "cyan": "#A6D0C7",
      "white": "#FFFFFF"
    }
  }
}
```

Light mode needs its own readable terminal mapping. Do not copy dark
background/foreground blindly.

The runtime custom-palette loader rejects a mode without `terminal` and falls
back to the built-in palette:
[theme_service.cpp](https://github.com/noctalia-dev/noctalia/blob/cebcc62284a42620ebb3518b3243665b43c11a96/src/theme/theme_service.cpp#L162-L198).
Missing UI roles can resolve to transparent values, so schema validation alone
is not sufficient. Repository tests must lint palette resources semantically.

## Theme IPC

All commands below address the isolated Weyriva profile:

```bash
weyriva shell msg theme-mode-get
weyriva shell msg theme-mode-toggle
weyriva shell msg theme-mode-set dark
weyriva shell msg theme-mode-set light
weyriva shell msg theme-mode-set auto

weyriva shell msg color-scheme-get
weyriva shell msg color-scheme-set wallpaper soft
weyriva shell msg color-scheme-set custom Weyriva
weyriva shell msg color-scheme-set builtin Noctalia
weyriva shell msg templates-apply
```

`theme-mode-get` reports the resolved `dark` or `light` mode, not the literal
configured word `auto`. `color-scheme-get` reports two tokens, for example
`wallpaper soft`. The authoritative commands are in the pinned
[Media & UI IPC reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/ipc/media-and-ui.mdx).

## Hooks

Theme-aware integration can use:

```toml
[hooks]
wallpaper_changed = "..."
colors_changed = ["...", "..."]
theme_mode_changed = "..."
```

`wallpaper_changed` receives `NOCTALIA_WALLPAPER_PATH` and
`NOCTALIA_WALLPAPER_CONNECTOR`. `theme_mode_changed` receives
`NOCTALIA_THEME_MODE`, `NOCTALIA_THEME_MODE_PREVIOUS`, and
`NOCTALIA_THEME_MODE_CONFIGURED`. Hooks are trusted shell commands and must not
be populated from untrusted data. See the pinned
[hook reference](https://github.com/noctalia-dev/noctalia-docs/blob/f88820cc90170ceb212efdea87711802ebaca1c9/src/content/docs/v5/automation/hooks.mdx).

## Greeter sync

With Noctalia Greeter and its apply helper installed:

```toml
[shell.greeter_sync]
auto_sync = true
```

Manual sync is:

```bash
weyriva shell msg greeter-sync
```

The sync copies the resolved palette and mode, wallpaper/per-output wallpapers,
font, corner radius, session actions, output layout, and transforms. It does
not copy shell animation settings or accessibility UI scale. Declarative
`/var/lib/noctalia-greeter/greeter.toml` wins over mutable `sync.toml`; details
are in [Session lifecycle](SESSION_LIFECYCLE.md).

## Diagnosis

Start with:

```bash
weyriva shell config validate
weyriva shell config export full
weyriva shell msg theme-mode-get
weyriva shell msg color-scheme-get
weyriva shell msg wallpaper-get
journalctl --user -u weyriva-shell.service -b --no-pager
```

If `color-scheme-get` says `custom Weyriva` but the UI looks like built-in
Noctalia, inspect the journal for a missing/invalid custom palette fallback and
run the palette semantic tests. If a declarative value is ignored, inspect the
effective export and the isolated `settings.toml` override layer before editing
files.
