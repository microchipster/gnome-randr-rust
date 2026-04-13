# Build Toward xrandr Capability Parity

## Status

Routing note only. Do not try to ship this as one change.

Pick the next numbered follow-up note that is still true, implement the smallest honest slice, then update this routing note if the order changes.

## Goal

Bring `gnome-randr` as close as practical to the monitor-management power people relied on from `xrandr`, while keeping a more modern CLI and aligning with GNOME Wayland's model instead of copying X11 syntax.

Capability parity matters more than syntax parity.

## What Is Already Landed

- `query` can report logical monitor state, physical monitor state, software brightness state, JSON output, raw property maps, and xrandr-style logical monitor list views.
- `modify` can already change mode by id or resolution, choose nearest refresh, use preferred or auto mode selection, scale including displayed rounded scale values, rotation, primary or noprimary state, and software brightness.
- dynamic shell completions and single-monitor defaults are already in place.

## Ordered Follow-up Notes

- `0050_build_a_transactional_multi_output_monitor_planner.md`
- `0060_add_real_output_disable_and_absolute_positioning.md`
- `0070_add_relative_placement_and_fix_rotation_reflow.md`
- `0080_add_same_as_clone_group_support_with_clear_mutter_limits.md`
- `0090_add_software_gamma_controls.md`
- `0100_add_reflection_and_supported_monitor_property_controls.md`
- `0110_add_saved_profiles_and_apply_from_query_json.md`
- `0120_add_wayland_native_backlight_power_save_and_layout_mode_controls.md`
- `0130_document_backend_limits_and_reroute_non_mappable_xrandr_requests.md`

## Why The Work Is Split This Way

- `0020` through `0040` were the easiest parity wins and mostly extended existing query/modify flows.
- `0050` is the architectural prerequisite for the bigger layout features.
- `0060` through `0080` cover the most important missing topology controls from the historical issue backlog.
- `0090` through `0120` expand output-control parity and Wayland-native capabilities after the layout foundation is solid.
- `0130` keeps the backlog honest about what the Mutter D-Bus backend can and cannot represent.

## Existing Backlog This Reroutes

- `docs/unaddressed/issues/0013_Add_an__--off__flag.md`
- `docs/unaddressed/issues/0020__Feature_Request___Mirroring_config.md`
- `docs/unaddressed/issues/0021__Feature_Request___Add_capability_to_turn_off_screen_and_set.md`
- `docs/unaddressed/issues/0027_when_try_to_rotate_left_or_right_encounter_error_D-Bus_error.md`
- `docs/unaddressed/issues/0028_Can_t_change_mode.md`
- `docs/unaddressed/issues/0009_Allow_reading_a_config_file.md`
- `docs/unaddressed/issues/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`
- `docs/unaddressed/prs/0018_Add_XY_movement___refactor_monitor_types.md`

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
