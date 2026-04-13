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

## How This Was Addressed

- extended the transactional planner with geometry-aware logical-monitor sizing based on the selected mode ids, layout mode, scale, and transform
- added `modify --left-of`, `--right-of`, `--above`, and `--below`, resolving final coordinates from post-mode/post-scale/post-rotation geometry instead of stale pre-change coordinates
- added automatic geometry reflow when mode, scale, or rotation changes alter a monitor's extents and the user did not request an explicit new placement, which fixes the stale-adjacency rotation failure path
- updated dynamic completions so relative-placement flags complete connector names while excluding the current target connector
- added planner and CLI tests covering rotated geometry placement, reflow of right-side neighbors after width changes, same-logical-monitor rejection, and explicit conflict validation between absolute and relative placement

## How To Exercise And Test It

- on a multi-monitor machine, preview relative placement:
  - `cargo run -- modify HDMI-1 --left-of eDP-1 --dry-run`
  - `cargo run -- modify HDMI-1 --right-of eDP-1 --dry-run`
  - `cargo run -- modify HDMI-1 --above eDP-1 --dry-run`
  - `cargo run -- modify HDMI-1 --below eDP-1 --dry-run`
- preview a geometry-changing rotation that should now reflow neighbors instead of leaving stale extents:
  - `cargo run -- modify HDMI-1 --rotate right --dry-run`
- inspect the resulting positions after a real apply:
  - `cargo run -- query`
  - `cargo run -- query --json | jq '.logical_monitors[] | {x, y, rotation, monitors}'`
- in a single-monitor environment, exercise the planner math directly:
  - `cargo test cli::modify::planner::tests::relative_placement_uses_final_rotated_geometry`
  - `cargo test cli::modify::planner::tests::reflow_moves_right_neighbors_after_rotation_changes_width`
