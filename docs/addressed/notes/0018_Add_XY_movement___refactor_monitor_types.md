# PR #18: Add XY movement & refactor monitor types

**Opened by** FractalMachinist **at** 2022-06-06T23:51:55Z

## Body

Feature request and review context preserved from the original PR.

## How This Was Addressed

- the user-facing capability from this PR is now covered by the landed planner-based layout work without merging the original refactor directly
- `modify --position` / `--pos` ships explicit XY placement with parsing kept in the CLI layer, matching the maintainer review direction
- `modify --left-of`, `--right-of`, `--above`, and `--below` now provide the clearer relative-placement controls that users actually needed on top of explicit XY positioning
- scale remains a separate option from position, again matching the maintainer review comments on the PR

## How To Exercise And Test It

- preview explicit XY movement:
  - `cargo run -- modify HDMI-1 --position 3200,180 --dry-run`
- preview relative placement instead of hand-computing coordinates:
  - `cargo run -- modify HDMI-1 --left-of eDP-1 --dry-run`
- verify the planner geometry tests:
  - `cargo test cli::modify::planner::tests::planner_can_compose_primary_position_transform_and_mode_changes`
  - `cargo test cli::modify::planner::tests::relative_placement_uses_final_rotated_geometry`
