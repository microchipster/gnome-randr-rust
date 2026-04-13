# Build Toward xrandr Capability Parity

## Status

Routing note only. Do not try to ship this as one change.

Pick the next numbered follow-up note that is still true, implement the smallest honest slice, then update this routing note if the order changes.

## Goal

Bring `gnome-randr` as close as practical to the monitor-management power people relied on from `xrandr`, while keeping a more modern CLI and aligning with GNOME Wayland's model instead of copying X11 syntax.

Capability parity matters more than syntax parity.

## What Is Already Landed

- `query` can report logical monitor state, physical monitor state, enabled state for disabled outputs, software brightness state, software gamma state, JSON output, raw property maps, and xrandr-style logical monitor list views.
- `modify` can already change mode by id or resolution, choose nearest refresh, use preferred or auto mode selection, disable outputs with `--off`, set absolute positions with `--position` / `--pos`, place outputs relative to each other with `--left-of` / `--right-of` / `--above` / `--below`, request same-as mirroring with local clone preflight, reflow adjacent layouts after geometry-changing rotations, scale including displayed rounded scale values, rotation, primary or noprimary state, software brightness, and software gamma, and it now plans changes through one full transactional config payload internally.
- dynamic shell completions and single-monitor defaults are already in place.

## Ordered Follow-up Notes

- `0100_add_reflection_and_supported_monitor_property_controls.md`
- `0110_add_saved_profiles_and_apply_from_query_json.md`
- `0120_add_wayland_native_backlight_power_save_and_layout_mode_controls.md`
- `0130_document_backend_limits_and_reroute_non_mappable_xrandr_requests.md`

## Why The Work Is Split This Way

- `0020` through `0040` were the easiest parity wins and mostly extended existing query/modify flows.
- `0050` established the architectural prerequisite for the bigger layout features.
- `0060` through `0080` established the core topology-control foundation after the planner, including same-as mirroring where Mutter accepts it.
- `0090` landed software gamma on top of the preserved LUT pipeline; `0100` through `0120` expand the remaining output-control parity and Wayland-native capabilities.
- `0130` keeps the backlog honest about what the Mutter D-Bus backend can and cannot represent.

## Existing Backlog This Reroutes

- `docs/unaddressed/issues/0028_Can_t_change_mode.md`
- `docs/unaddressed/issues/0009_Allow_reading_a_config_file.md`
- `docs/unaddressed/issues/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`

## Success Criteria For The Whole Track

Call this near-parity when `gnome-randr` can reliably do the common real-world monitor tasks people used `xrandr` for:

- inspect current outputs and modes
- choose preferred, auto, or refresh-constrained modes
- enable and disable outputs
- set primary and unset primary
- position outputs absolutely and relatively
- mirror outputs where Mutter allows it
- rotate and reflect outputs
- scale outputs without hidden precision traps
- control software brightness and gamma
- save and restore named layouts

The remaining X11-only or backend-mismatched features should be documented explicitly instead of being left ambiguous.
