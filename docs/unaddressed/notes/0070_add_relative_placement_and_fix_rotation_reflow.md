# Add Relative Placement And Fix Rotation Reflow

## Why This Exists

Once the planner can own full layout state, the next major parity slice is relative placement:

- `left-of`
- `right-of`
- `above`
- `below`

This is also the right place to fix the existing bug where left or right rotation can fail with `Logical monitors not adjacent` because the layout is not reflowed around the transformed geometry.

## Scope

- add relative placement controls using current connector identities
- compute final coordinates from the target layout instead of mutating the current coordinates in place
- recompute adjacency and extents after rotation, especially for `left` and `right`
- make conflicts between absolute and relative placement explicit and well-tested

## Acceptance Criteria

- users can place one output relative to another without manually computing pixel coordinates
- left and right rotation no longer fail solely because stale pre-rotation geometry was reused
- dry-run shows the resolved final positions after all requested transforms are considered
- tests cover placement math and rotation-aware reflow

## Likely Files

- planner modules introduced by `0050_build_a_transactional_multi_output_monitor_planner.md`
- `src/cli/modify/mod.rs`
- `src/display_config/logical_monitor.rs`
- `src/cli/complete.rs`

## References

- `docs/unaddressed/issues/0021__Feature_Request___Add_capability_to_turn_off_screen_and_set.md`
- `docs/unaddressed/issues/0027_when_try_to_rotate_left_or_right_encounter_error_D-Bus_error.md`
- `docs/unaddressed/prs/0018_Add_XY_movement___refactor_monitor_types.md`

## Follow-ups

- mirror and clone-group work belongs in `0080_add_same_as_clone_group_support_with_clear_mutter_limits.md`
