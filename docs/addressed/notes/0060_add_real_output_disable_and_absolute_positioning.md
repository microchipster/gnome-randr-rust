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

## How This Was Addressed

- added a real `modify --off` path that removes the target connector from the applied logical-monitor payload through the transactional planner instead of pretending `-1` is a mode id
- added absolute `modify --position X,Y` with `--pos` alias, using the planner's logical-monitor position mutation path
- extended `modify` validation so `--off` conflicts with layout, primary, and brightness flags that would be ambiguous on a disabled output
- updated `query` text and JSON so disabled-but-still-connected outputs remain queryable by connector and now report explicit `enabled` state
- bumped the JSON schema to version `3` to add physical-monitor `enabled` reporting while keeping disabled connector queries stable

## How To Exercise And Test It

- preview disabling one output:
  - `cargo run -- modify HDMI-1 --off --dry-run`
- actually disable one output, then inspect it directly:
  - `cargo run -- modify HDMI-1 --off`
  - `cargo run -- query HDMI-1`
  - `cargo run -- query HDMI-1 --json | jq '.monitors[0].enabled'`
- preview absolute positioning for an enabled output:
  - `cargo run -- modify eDP-1 --position 0,0 --dry-run`
  - `cargo run -- modify eDP-1 --pos 1920x0 --dry-run`
- inspect enabled state and positions after a real apply:
  - `cargo run -- query`
  - `cargo run -- query --json | jq '.logical_monitors[] | {x, y, monitors}'`
- confirm the old fake-mode path is no longer the intended workflow:
  - `cargo run -- modify HDMI-1 --off --dry-run`
  - compare with the historical issue note showing `--mode=-1` failed through D-Bus validation
