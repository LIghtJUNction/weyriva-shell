# Status and roadmap

Weyriva is moving from a collection of separate desktop components to one
Noctalia-powered Niri session. Milestones remain incomplete until their runtime
acceptance gates pass.

## Current foundation

- Noctalia v5 is the sole desktop-shell engine for the bar, launcher, panels,
  notifications, wallpaper, OSD, lock screen, settings, and native Luau plugins.
- Weyriva keeps an isolated Noctalia profile and its versioned `weyriva.*` IPC
  namespace for compatibility with trusted executable plugins.
- Noctalia Greeter is the visible login surface; greetd remains the internal PAM
  and VT broker.
- Niri owns the graphical-session lifecycle through systemd user units, with
  bounded shell restart, lock reconciliation, and session-exit failsafe design.
- The root one-command installer and AUR scaffold are the system installation
  paths. The user-only installer manages profile data and does not install
  service overrides.
- Light and dark editorial assets, wallpaper-derived color, deterministic theme
  scheduling, and a complete offline fallback palette are present in source.

## Acceptance milestones

1. **Noctalia v5 shell and native plugin integration**
   - Complete the full settings/surface parity audit.
   - Validate official and community plugin lifecycle behavior against pinned
     upstream catalogs.

2. **Login, lock, and desktop lifecycle**
   - Validate the Noctalia Greeter → greetd → Weyriva → Niri chain on supported
     distributions.
   - Qualify crash-loop recovery, locked-shell restart, TTY2 recovery, logout,
     suspend, multi-monitor, and HiDPI behavior on real hardware.

3. **Packaging and zero-configuration installation**
   - Build and install the AUR package in a clean Arch environment.
   - Exercise the one-command installer on Arch-family systems and verify
     conservative failure on unsupported package/runtime combinations.
   - Publish and maintain the package only after those gates pass.

4. **Legacy Noctalia v4 compatibility**
   - Implement and isolate the QML compatibility host.
   - Execute representative v4 plugins; manifest discovery alone is not
     compatibility.

5. **Catalog and XRY acceptance**
   - Run the pinned official/community catalog matrix across all entry kinds and
     supported API boundaries.
   - Install on XRY and complete visual, interaction, login, lock, recovery, and
     runtime-log acceptance.

The v4 compatibility, catalog matrix, AUR publication, and XRY acceptance gates
are pending. Source implementation or local configuration validation does not
make those milestones complete.
