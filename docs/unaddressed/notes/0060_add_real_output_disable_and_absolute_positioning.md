# Add Real Output Disable And Absolute Positioning

## Why This Exists

Two long-standing parity gaps are still open:

- disabling an output with a real `--off` equivalent
- setting explicit output coordinates with a real `--pos` style control

Historical issue `#13` showed that faking disable through `--mode=-1` does not work. The real fix needs to operate at the logical-monitor configuration level.

## Scope

- add a real `modify --off` path that removes or disables the target output through the planner instead of inventing a fake mode id
- add absolute positioning such as `--position X,Y` or `--pos XxY`, keeping the modern CLI naming decision consistent across the project
- make dry-run show the final connector, enabled-state, and coordinates clearly
- update completions and help text for the new options

## Acceptance Criteria

- disabling an output no longer depends on unsupported `-1` mode ids
- a user can set absolute coordinates for an output in one command
- `query` reflects the changed enabled state and position after apply
- tests cover planner serialization for disabled and repositioned outputs

## Likely Files

- `src/cli/modify/mod.rs`
- planner modules introduced by `0050_build_a_transactional_multi_output_monitor_planner.md`
- `src/cli/complete.rs`
- `docs/unaddressed/issues/0013_Add_an__--off__flag.md`
- `docs/unaddressed/issues/0021__Feature_Request___Add_capability_to_turn_off_screen_and_set.md`

## References

- `docs/unaddressed/issues/0013_Add_an__--off__flag.md`
- `docs/unaddressed/issues/0021__Feature_Request___Add_capability_to_turn_off_screen_and_set.md`

## Follow-ups

- relative placement belongs in `0070_add_relative_placement_and_fix_rotation_reflow.md`
