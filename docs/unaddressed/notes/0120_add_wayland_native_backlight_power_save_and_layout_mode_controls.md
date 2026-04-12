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
