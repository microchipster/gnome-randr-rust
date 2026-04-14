# Add Wayland-Native Backlight, Power-Save, And Layout-Mode Controls

## Why This Exists

Once the core parity work is in better shape, the project should expose the useful GNOME/Mutter controls that go beyond classic `xrandr`:

- hardware backlight when Mutter exposes it
- screen power-save mode
- layout-mode switching when supported

These are good fits for a modern Wayland-first tool and help distinguish `gnome-randr` from merely being an `xrandr` imitation.

## Scope

- expose hardware backlight capabilities and mutation through a dedicated CLI path when supported
- expose the global power-save mode in a clear, explicit way
- allow changing layout mode when `supports-changing-layout-mode` is true
- surface all of these capabilities in query text and JSON so scripts can discover support before attempting a change

## Acceptance Criteria

- the CLI can distinguish between hardware backlight support and software brightness support
- unsupported backlight and layout-mode changes fail with explicit capability errors
- query output shows support and current state clearly enough for scripting
- help text explains the difference between hardware backlight, software brightness, and global power-save

## Likely Files

- `src/display_config/raw.rs`
- `src/display_config/mod.rs`
- `src/cli/query.rs`
- new CLI modules for backlight or power management

## Follow-ups

- keep this note focused on Mutter-native controls already visible in the D-Bus API
- do not mix in unsupported X11-only flags here

## How This Was Addressed

- extended `LayoutMode` to support `global-ui-logical` in addition to `logical` and `physical`
- added typed native display state wrappers for `PowerSaveMode`, `Backlight`, and `Luminance`, plus setter methods for power-save, backlight, luminance, and reset-luminance
- added `modify --layout-mode`, `--power-save`, `--backlight`, `--luminance`, and `--reset-luminance` with capability checks and explicit native-control errors
- refactored `modify` so connector resolution is conditional, allowing global-only controls such as `--power-save off` and `--layout-mode physical` without requiring a connector
- taught `apply FILE` to replay saved layout-mode changes through top-level `ApplyMonitorsConfig` properties when the backend reports that layout-mode changes are supported
- bumped `query --json` to schema version `6` and added typed top-level native display state plus per-monitor hardware backlight and luminance preference fields, while keeping raw property maps available for inspection
- explicitly kept hardware backlight distinct from software brightness/gamma in help text and docs

## How To Exercise And Test It

- inspect native state in text:
  - `cargo run -- query --summary`
  - `cargo run -- query --verbose`
- inspect native state in JSON:
  - `cargo run -- query --json | jq '{power_save_mode, panel_orientation_managed, night_light_supported}'`
  - `cargo run -- query --json | jq '.monitors[] | {connector, hardware_backlight_supported, hardware_backlight, hardware_backlight_min_step, luminance_preferences}'`
- preview global native controls:
  - `cargo run -- modify --power-save off --dry-run`
  - `cargo run -- modify --layout-mode physical --dry-run`
- preview per-monitor native controls:
  - `cargo run -- modify --backlight 80 --dry-run`
  - `cargo run -- modify --luminance 90 --dry-run`
  - `cargo run -- modify --reset-luminance --dry-run`
- preview saved layout replay with query JSON:
  - `tmpfile=$(mktemp); cargo run -- query --json > "$tmpfile" && cargo run -- apply "$tmpfile" --dry-run; rm -f "$tmpfile"`
- run targeted tests:
  - `cargo test power_save_mode_round_trips_raw_values`
  - `cargo test apply_monitor_serializes_color_mode_property`
  - `cargo test cli::query::tests::json_output_uses_documented_schema`
