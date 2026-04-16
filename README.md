# gnome-randr-rust

A reimplementation of `xrandr` for Gnome on Wayland, especially for systems that don't support `wlr-output-management-unstable-v1`  (e.g. Manjaro). Written ground-up in Rust for performance and fun. This is also my first project in rust, so any suggestions are welcome!

> [!NOTE]  
> I currently am not able to maintain this, as I no longer have access to a working Linux machine. If someone is interested in maintaining, please let me know!
>
> On Gnome 48+, try the [gdctl](https://gitlab.gnome.org/GNOME/mutter/-/blob/main/doc/man/gdctl.rst) CLI that came with it, it's most likely to stay up to date.

(For non-Gnome compositors, see display configuration links at https://arewewaylandyet.com/)

## Installation

Installation requires `pkg-config` and `cargo`, part of the Rust toolchain. [Cargo/Rust installation instructions](https://doc.rust-lang.org/cargo/getting-started/installation.html).

To install the published crate, run `cargo install gnome-randr`. To install the current checkout instead, run `cargo install --path .`.

Cargo installs the binary into Cargo's bin directory by default. If you want a custom install location, use `cargo install --root /your/prefix ...` and invoke `/your/prefix/bin/gnome-randr` directly or add that bin directory to your `PATH`.

The installed binary is named `gnome-randr`. If your system already has another `gnome-randr` on `PATH`, use the full path to the one you want or adjust `PATH` ordering explicitly.

A library is also exposed for use in other Rust programs.

Shell completions can be generated with `gnome-randr completions bash`, `gnome-randr completions zsh`, or `gnome-randr completions fish`. These are generated from the current CLI definition at runtime, with dynamic live completions for monitor-dependent values.

## Capability parity status

`gnome-randr` is aiming for `xrandr` capability parity with a more modern Wayland-first CLI, not argument-for-argument syntax parity.

- Implemented: query text/summary/JSON output, `query --verbose`, `query --properties`, `query --listmonitors`, `query --listactivemonitors`, enabled-state reporting for disabled outputs, typed reflection/color-mode/underscanning visibility in query output, typed native power-save/backlight/luminance visibility in query output, mode selection by id or resolution with `--refresh`, `--preferred`, `--auto`, `--primary` / `--noprimary`, real `modify --off`, absolute `modify --position` / `--pos`, relative placement via `--left-of` / `--right-of` / `--above` / `--below`, `modify --same-as` mirroring with local clone preflight, rotation-aware geometry reflow, `modify --reflect`, `modify --color-mode`, native `modify --layout-mode`, `modify --power-save`, `modify --backlight`, `modify --luminance`, `modify --reset-luminance`, scale including the rounded values shown in `query`, rotation, software brightness with filters, software gamma via `modify --gamma`, `apply FILE` using saved `query --json` layouts matched by monitor identity, dynamic shell completions, current software brightness and gamma reporting in `query`, and an internal transactional full-config planner behind `modify`
- Planned next: none in the parity roadmap; the remaining gaps are backend limits rather than normal CLI work
- Limited by Mutter: some same-as / partial mirroring layouts, some native layout-mode changes, writable monitor properties beyond the documented color-mode/backlight/luminance surface, and some power-save/backlight semantics still depend on what the current `org.gnome.Mutter.DisplayConfig` backend accepts at apply time

## Mirroring

- `gnome-randr modify CONNECTOR --same-as REFERENCE` asks Mutter to place `CONNECTOR` into the same logical monitor as `REFERENCE` instead of trying to overlap separate logical monitors.
- Before apply, gnome-randr rejects obviously impossible mirror requests using Mutter's resource model: clone capability, shared possible CRTCs, and a compatible mode on the target output.
- Even when local preflight passes, Mutter may still reject some partial mirroring layouts. In those cases gnome-randr reports that GNOME's DisplayConfig validation rejected the clone request rather than pretending the local planner proved it would work.
- Unsupported with the current backend: custom modelines, arbitrary transform matrices and panning, X11 provider/CRTC controls, and framebuffer/DPI compatibility flags

## Reflection And Monitor Properties

- `gnome-randr modify CONNECTOR --reflect normal|x|y|xy` maps xrandr-style reflection names onto Mutter's flipped transform model instead of exposing raw transform integers.
- `gnome-randr modify CONNECTOR --color-mode default|bt2100` exposes the small typed monitor-property surface that Mutter and `gdctl` already support today.
- `gnome-randr query` and `gnome-randr query --json` now report typed `rotation`, `reflection`, `color_mode`, `supported_color_modes`, and `is_underscanning` fields in addition to the raw property map.
- Writable underscanning is intentionally not exposed here because the current Mutter-supported write surface could not be justified beyond query visibility; `--properties` and JSON still show it when the compositor exposes it.

## Wayland-Native Controls

- `gnome-randr modify --layout-mode logical|physical|global-ui-logical` exposes Mutter's global layout-mode control when the backend reports that layout-mode changes are supported.
- `gnome-randr modify --power-save on|standby|suspend|off` exposes Mutter's global power-save property using the enum names documented in the upstream DisplayConfig XML.
- `gnome-randr modify CONNECTOR --backlight PERCENT` uses Mutter's native connector backlight API when the connector is listed in the typed `Backlight` state.
- `gnome-randr modify CONNECTOR --luminance PERCENT` and `--reset-luminance` use Mutter's native per-monitor luminance preference for the current or requested color mode.
- These native controls are separate from software `--brightness` and `--gamma`; query and query JSON report both families explicitly.

## Backend Limits

- Custom modelines and arbitrary mode injection are not supported with the current Mutter `DisplayConfig` backend. `gnome-randr` can only select modes that Mutter already exposes in `query` / `query --json`.
- Arbitrary transform matrices, panning, framebuffer hacks, and DPI compatibility flags are not surfaced by this CLI because the current backend does not expose a clean Wayland-native equivalent to the corresponding `xrandr` features.
- X11-specific provider and explicit CRTC selection controls are intentionally unsupported here because they are RandR-internal concepts, not part of the supported GNOME Wayland monitor-management model.
- Generic raw X11-style property setting is intentionally unsupported. `gnome-randr` only exposes typed monitor-property controls that Mutter and `gdctl` already support cleanly today, such as `--color-mode`, `--backlight`, and `--luminance`.
- If a requested mode is missing from `query`, the likely fix is upstream Mutter support or a different backend strategy, not a small CLI patch inside this repository.

## Software Color

- `gnome-randr modify CONNECTOR --brightness VALUE --filter FILTER` controls software brightness using the current compositor-installed LUT as the baseline instead of reconstructing a simplified curve.
- `gnome-randr modify CONNECTOR --gamma R[:G:B]` applies per-channel software gamma on top of that same preserved baseline.
- If only one gamma component is given, it is reused for red, green, and blue, matching `xrandr --gamma` semantics.
- When brightness and gamma are used together, gnome-randr applies gamma first and brightness/filter second, then stores that combined managed state so repeated absolute changes do not compound while the live LUT still matches the last tool-managed state.
- If another tool changes the LUT first, the next gnome-randr apply adopts that new LUT as the baseline instead of overwriting it with a lossy reconstructed curve.

See `docs/addressed/notes/0000_xrandr_capability_parity_routing.md` for the completed parity roadmap.

## Saved Layouts

- `gnome-randr apply FILE` reads a saved layout file generated from `gnome-randr query --json` and applies it as one full transactional monitor config.
- Matching is based on stable monitor identity (`vendor`, `product`, `serial`), not connector names alone, so the same saved file can still apply if connector names change between boots or docks.
- The apply path reuses the documented query JSON schema instead of inventing a second config format.
- Saved layout-mode changes are replayed too when the current backend reports that layout-mode changes are supported.
- Managed software brightness and gamma state from the saved file are restored after the layout apply; `unknown` software color state is intentionally not replayed because it cannot be reproduced faithfully from a query snapshot.

End-to-end example:

```sh
# save the current layout
gnome-randr query --json > work-layout.json

# preview how it resolves on current hardware
gnome-randr apply work-layout.json --dry-run

# apply it for real
gnome-randr apply work-layout.json
```

## Text query views

- `gnome-randr query --listmonitors` prints a concise xrandr-style logical-monitor list.
- `gnome-randr query --listactivemonitors` prints the active logical-monitor list. With the current Mutter query surface this usually matches `--listmonitors`.
- `gnome-randr query --properties` adds raw monitor and mode property maps to the text UI, including values such as underscanning-related state when Mutter exposes them.
- `gnome-randr query CONNECTOR --verbose` prints a more detailed inspection view using the same field names as the JSON schema.
- Disabled-but-still-connected outputs remain queryable by connector and now report `enabled: false` in text output.

## JSON output

`gnome-randr query --json` prints a documented machine-readable schema for scripts. `gnome-randr query CONNECTOR --json` uses the same schema, filtered down to the requested connector, even when that connector is currently disabled. The same schema can be saved and later restored with `gnome-randr apply FILE`. `--summary`, `--properties`, `--listmonitors`, and `--listactivemonitors` are text-only views and cannot be combined with `--json`.

Schema version `6` currently contains:

- top-level metadata: `schema_version`, `serial`, `layout_mode`, `supports_mirroring`, `supports_changing_layout_mode`, `global_scale_required`, `power_save_mode`, `panel_orientation_managed`, optional `apply_monitors_config_allowed`, optional `night_light_supported`, optional `has_external_monitor`, optional `renderer`, and optional raw `properties`
- `logical_monitors`: objects with `x`, `y`, `scale`, typed `rotation`, typed `reflection`, `primary`, associated `monitors`, and optional raw `properties`
- `monitors`: physical outputs with identity fields, `enabled`, optional `display_name` / `is_builtin` / `width_mm` / `height_mm`, typed `color_mode`, typed `supported_color_modes`, optional `is_underscanning`, `hardware_backlight_supported`, optional `hardware_backlight_active`, optional `hardware_backlight`, optional `hardware_backlight_min_step`, `luminance_preferences`, supported `modes`, `software_brightness`, `software_gamma`, and optional raw `properties`
- each luminance preference includes `color_mode`, `luminance`, `default`, and `is_unset`
- each mode includes `id`, `width`, `height`, `refresh_rate`, `preferred_scale`, `supported_scales`, `is_current`, `is_preferred`, and optional raw `properties`
- `software_brightness.state` is one of `managed`, `identity`, or `unknown`
- when `software_brightness.state` is `managed` or `identity`, `brightness` and `filter` are populated; otherwise they are `null`
- `software_gamma.state` is one of `managed`, `identity`, or `unknown`
- when `software_gamma.state` is `managed` or `identity`, `red`, `green`, and `blue` are populated; otherwise they are `null`

Example:

```json
{
  "schema_version": 6,
  "serial": 42,
  "layout_mode": "physical",
  "supports_mirroring": true,
  "supports_changing_layout_mode": false,
  "global_scale_required": false,
  "power_save_mode": "on",
  "panel_orientation_managed": false,
  "apply_monitors_config_allowed": true,
  "night_light_supported": true,
  "has_external_monitor": false,
  "renderer": "native",
  "properties": {
    "compositor-capabilities": ["gamma", "clone"]
  },
  "logical_monitors": [
    {
      "x": 0,
      "y": 0,
      "scale": 1.0,
      "rotation": "normal",
      "reflection": "normal",
      "primary": true,
      "monitors": [
        {
          "connector": "eDP-1",
          "vendor": "BOE",
          "product": "0x07c9",
          "serial": "0x00000000"
        }
      ],
      "properties": {
        "presentation": false
      }
    }
  ],
  "monitors": [
    {
      "connector": "eDP-1",
      "enabled": true,
      "vendor": "BOE",
      "product": "0x07c9",
      "serial": "0x00000000",
      "display_name": "Built-in display",
      "is_builtin": true,
      "width_mm": 300,
      "height_mm": 190,
      "color_mode": "bt2100",
      "supported_color_modes": ["default", "bt2100"],
      "is_underscanning": false,
      "hardware_backlight_supported": true,
      "hardware_backlight_active": true,
      "hardware_backlight": 80,
      "hardware_backlight_min_step": 5,
      "luminance_preferences": [
        {
          "color_mode": "bt2100",
          "luminance": 100.0,
          "default": 100.0,
          "is_unset": true
        }
      ],
      "modes": [
        {
          "id": "1920x1080@59.999",
          "width": 1920,
          "height": 1080,
          "refresh_rate": 59.999,
          "preferred_scale": 1.0,
          "supported_scales": [1.0, 2.0],
          "is_current": true,
          "is_preferred": true,
          "properties": {
            "color-space": "srgb"
          }
        }
      ],
      "software_brightness": {
        "state": "managed",
        "brightness": 1.25,
        "filter": "filmic"
      },
      "software_gamma": {
        "state": "managed",
        "red": 1.1,
        "green": 1.0,
        "blue": 0.9
      },
      "properties": {
        "color-mode": 1,
        "is-underscanning": false,
        "supported-color-modes": [0, 1]
      }
    }
  ]
}
```

Some useful `jq` examples:

```sh
# list connectors with current software brightness state
gnome-randr query --json | jq -r '.monitors[] | "\(.connector)\t\(.software_brightness.state)\t\(.software_brightness.brightness // "null")\t\(.software_brightness.filter // "null")"'

# print the current mode id for each connector
gnome-randr query --json | jq -r '.monitors[] | "\(.connector)\t\(.modes[] | select(.is_current).id)"'

# show only built-in displays
gnome-randr query --json | jq '.monitors[] | select(.is_builtin == true)'

# get brightness/filter for one connector
gnome-randr query eDP-1 --json | jq '.monitors[0].software_brightness'

# get software gamma for one connector
gnome-randr query eDP-1 --json | jq '.monitors[0].software_gamma'

# inspect typed reflection and color-mode state
gnome-randr query --json | jq '.logical_monitors[] | {rotation, reflection, monitors}'
gnome-randr query --json | jq '.monitors[] | {connector, color_mode, supported_color_modes, is_underscanning}'

# inspect native power-save and backlight/luminance state
gnome-randr query --json | jq '{power_save_mode, panel_orientation_managed, night_light_supported}'
gnome-randr query --json | jq '.monitors[] | {connector, hardware_backlight_supported, hardware_backlight, hardware_backlight_min_step, luminance_preferences}'

# inspect raw monitor properties when present
gnome-randr query --json | jq '.monitors[] | {connector, properties}'

# list connectors and whether they are currently enabled
gnome-randr query --json | jq -r '.monitors[] | "\(.connector)\t\(.enabled)"'
```

## Method

`gnome-randr-rust` uses the `dbus` object `org.gnome.Mutter.DisplayConfig`. See https://wiki.gnome.org/Initiatives/Wayland/Gaps/DisplayConfig for the original proposal, although the specification listed there is somewhat out of date (checked via `dbus introspect` on Gnome shell 40.5). Gnome maintain the evolving XML file [here](https://gitlab.gnome.org/GNOME/mutter/-/blob/main/data/dbus-interfaces/org.gnome.Mutter.DisplayConfig.xml).

The `GetCurrentState` method is used to list information about the displays, while `ApplyMonitorsConfig` is used to modify the current configuration.

## Inspiration

This project was heavily inspired by `xrandr` (obviously) and also [`gnome-randr`](https://gitlab.com/Oschowa/gnome-randr/). Sadly, `gnome-randr.py` appears to be broken as of my gnome version (40.5) when trying to modify display configurations. 

`gnome-randr.py` is also slower than my rust reimplementation: querying the python script takes about 30ms on my 3-monitor system, while the rust implementation takes about 3ms (`xrandr` takes about 1.5ms, but is also displaying different information due to limitations in `xrandr`'s bridge.)
