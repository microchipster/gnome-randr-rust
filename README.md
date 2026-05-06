# gnome-randr-rust

`gnome-randr` is a GNOME Wayland display-management CLI built on
`org.gnome.Mutter.DisplayConfig`.

It targets capability parity with the useful parts of `xrandr`, not argument
compatibility with RandR.

## Install

Requirements:

- `cargo`
- `pkg-config`

Published crate:

```sh
cargo install gnome-randr
```

Current checkout:

```sh
cargo install --path .
```

Custom prefix:

```sh
cargo install --path . --root /your/prefix
```

The installed binary name is `gnome-randr`.

Completions:

```sh
gnome-randr completions bash
gnome-randr completions zsh
gnome-randr completions fish
```

## Capabilities

- text and JSON query
- mode, refresh, scale, primary, enable/disable
- absolute and relative placement
- mirroring with `--same-as`
- rotation and reflection
- typed monitor properties such as `--color-mode`
- software brightness and software gamma
- native layout mode, power-save, backlight, and luminance controls
- saved layout restore with `apply FILE`

Completed roadmap:

- `docs/addressed/notes/0000_xrandr_capability_parity_routing.md`
- `docs/addressed/notes/0000_align_native_surface_with_gdctl_routing.md`

## Common Commands

Inspect state:

```sh
gnome-randr query
gnome-randr query --summary
gnome-randr query --json
gnome-randr query eDP-1 --verbose
```

Change layout:

```sh
gnome-randr modify eDP-1 --mode 1920x1080 --refresh 60
gnome-randr modify HDMI-1 --right-of eDP-1
gnome-randr modify HDMI-1 --same-as eDP-1
gnome-randr modify HDMI-1 --off
```

Preview without applying:

```sh
gnome-randr modify eDP-1 --scale 2 --dry-run
```

## Software Color

Software color uses the current compositor-installed LUT as baseline.

Brightness:

```sh
gnome-randr modify eDP-1 --brightness 1.25 --filter filmic
```

Gamma:

```sh
gnome-randr modify eDP-1 --gamma 1.1
gnome-randr modify eDP-1 --gamma 1.1:1.0:0.9
```

Composition order:

- gamma first
- brightness/filter second

`query` reports both `software_brightness` and `software_gamma`.

## Native Controls

These are distinct from software brightness/gamma:

```sh
gnome-randr modify --layout-mode logical
gnome-randr modify --power-save off
gnome-randr modify eDP-1 --backlight 80
gnome-randr modify eDP-1 --luminance 90
gnome-randr modify eDP-1 --reset-luminance
```

## Reflection And Typed Properties

Reflection:

```sh
gnome-randr modify eDP-1 --reflect x
gnome-randr modify eDP-1 --reflect xy
```

Typed property control:

```sh
gnome-randr modify eDP-1 --color-mode default
gnome-randr modify eDP-1 --color-mode bt2100
```

`query` exposes typed:

- `rotation`
- `reflection`
- `color_mode`
- `supported_color_modes`
- `is_for_lease`
- `is_underscanning`

`query --properties` still exposes the raw property maps.

## Leasing

`gnome-randr` supports Mutter's typed monitor leasing surface.

```sh
gnome-randr modify --for-lease-monitor DP-2 --dry-run
```

- leased monitors are removed from any active logical monitor in the applied config
- leasing is sent through the top-level `monitors-for-lease` property in `ApplyMonitorsConfig`
- `query` and `query --json` expose typed `is_for_lease`

## Saved Layouts

The saved layout format is the same schema returned by `query --json`.

```sh
gnome-randr query --json > work-layout.json
gnome-randr apply work-layout.json --dry-run
gnome-randr apply work-layout.json
```

Matching uses monitor identity (`vendor`, `product`, `serial`), not connector
name alone.

## JSON Output

Current schema version: `8`.

Top-level structure:

- display metadata and native display state
- `logical_monitors`
- `monitors`

`logical_monitors` include geometry, typed rotation/reflection, primary flag,
and attached monitor identities.

`monitors` include identity, enabled state, typed lease state, modes, typed monitor properties, native backlight/luminance state, and software color state.

Examples:

```sh
# native display state
gnome-randr query --json | jq '{layout_mode, power_save_mode, night_light_supported}'

# software color state
gnome-randr query eDP-1 --json | jq '.monitors[0] | {software_brightness, software_gamma}'

# typed reflection and monitor properties
gnome-randr query --json | jq '.logical_monitors[] | {rotation, reflection, monitors}'
gnome-randr query --json | jq '.monitors[] | {connector, color_mode, supported_color_modes, is_for_lease, is_underscanning}'

# native backlight and luminance
gnome-randr query --json | jq '.monitors[] | {connector, hardware_backlight_supported, hardware_backlight, luminance_preferences}'
```

## Backend Limits

Not supported with the current backend:

- custom modelines or arbitrary mode injection
- arbitrary transform matrices or panning
- framebuffer and DPI compatibility flags
- X11 provider and explicit CRTC controls
- generic raw X11-style property setting

Some operations are backend-limited even when the CLI supports them, especially:

- partial mirroring in some layouts
- some layout-mode transitions
- some native power-save and backlight behavior

If a mode is missing from `query`, Mutter did not expose it.

## Method

The CLI uses `org.gnome.Mutter.DisplayConfig` over D-Bus.

- `GetCurrentState` for inspection
- `ApplyMonitorsConfig` for layout changes
- native Mutter setters for power-save, backlight, and luminance where available

## Inspiration

- `xrandr`
- [`gnome-randr`](https://gitlab.com/Oschowa/gnome-randr/)

## Alignment With `gdctl`

[`gdctl`](https://gitlab.gnome.org/GNOME/mutter/-/blob/main/doc/man/gdctl.rst)
is the upstream Mutter CLI for the same `org.gnome.Mutter.DisplayConfig`
backend. `gnome-randr` now uses it as the native reference for terminology,
enums, capability boundaries, and backend expectations.

Native surface aligned with `gdctl`:

- `show` / `set` / `--verify` aliases for `query` / `modify` / `--dry-run`
- layout mode, placement, same-as mirroring, rotation, reflection, mode,
  scale, and primary controls
- typed monitor properties such as `color-mode`, `rgb-range`, and
  `for-lease-monitor`
- native power-save, backlight, and luminance controls

Intentional differences:

- `gnome-randr` keeps the higher-level `query` / `modify` / `apply FILE` model
  instead of cloning `gdctl` syntax wholesale
- `query --json`, software brightness/gamma, saved layout restore, and shell
  completions are higher-level features beyond `gdctl`
- aliases are additive only where semantics actually match cleanly

Reference notes:

- `docs/addressed/notes/0000_align_native_surface_with_gdctl_routing.md`
- `docs/addressed/notes/0010_publish_gdctl_compatibility_matrix_and_divergence_policy.md`
