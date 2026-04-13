# Issue #13: Add an `--off` flag

**Opened by** maxwellainatchi **at** 2021-10-26T15:07:29Z

## Body



## Comments

Historical discussion preserved from the original issue.

## How This Was Addressed

- `gnome-randr modify CONNECTOR --off` now disables an output by removing it from the transactional logical-monitor payload instead of trying to pass a fake `-1` mode id through Mutter
- `query CONNECTOR` and `query CONNECTOR --json` now keep disabled-but-still-connected outputs visible and explicitly report `enabled: false`
- this resolves the core bug documented in the issue: `--mode=-1` was rejected by D-Bus as an invalid mode and was never the right backend model for output disable

## How To Exercise And Test It

- preview the new off path:
  - `cargo run -- modify HDMI-1 --off --dry-run`
- apply it for real on a non-primary external monitor:
  - `cargo run -- modify HDMI-1 --off`
- confirm the output is now reported as disabled instead of disappearing:
  - `cargo run -- query HDMI-1`
  - `cargo run -- query HDMI-1 --json | jq '.monitors[0].enabled'`
