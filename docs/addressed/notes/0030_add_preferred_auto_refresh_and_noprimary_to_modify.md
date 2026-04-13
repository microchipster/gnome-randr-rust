# Add Preferred, Auto, Refresh, And No-Primary To modify

## Why This Exists

`modify` can already set an explicit mode and primary flag, but it still lacks several of the most common day-to-day selection controls people reached for in `xrandr`.

These are straightforward parity wins because the current query data already exposes preferred and current modes, and the existing apply path already changes mode and primary state.

## Scope

- add `--preferred` to select the preferred mode for the target output
- add `--auto` to select the preferred mode, or the best current fallback if preferred is unavailable
- add `--refresh` / `--rate` to select the nearest refresh among modes with the requested resolution or current resolution
- add `--noprimary` to explicitly clear primary state from the target logical monitor
- define and validate flag interactions so the command line stays unambiguous
- update help text and completions for the new flags

## Acceptance Criteria

- users can choose a mode without spelling the full mode id when the desired behavior is obvious
- `--primary` and `--noprimary` have deterministic behavior and useful conflict errors
- dry-run output clearly states what mode and primary change will be applied
- tests cover the flag-resolution logic without depending on a live display server

## Likely Files

- `src/cli/modify/mod.rs`
- `src/cli/modify/actions/`
- `src/cli/complete.rs`
- `src/cli/completions.rs`

## Follow-ups

- keep this note scoped to output-selection sugar on the existing single-output modify flow
- richer query surfaces belong in `0040_add_richer_query_views_and_property_inspection.md`
- multi-output atomic behavior belongs in `0050_build_a_transactional_multi_output_monitor_planner.md`

## How This Was Addressed

- added `--preferred`, `--auto`, `--refresh` with visible `--rate` alias, and `--noprimary` to `gnome-randr modify`
- taught `--mode` to accept either a full mode id like `1920x1080@59.999` or a resolution shorthand like `1920x1080`
- resolved `--refresh` against either the requested resolution or the current resolution, choosing the nearest available refresh advertised by Mutter
- validated ambiguous combinations so `--refresh` cannot be paired with an exact mode id, and `--primary` / `--noprimary` conflict cleanly
- updated dynamic completions so refresh suggestions follow the selected resolution and the new flags appear in generated shell completions
- added unit tests for preferred, auto, refresh, resolution shorthand, and conflict handling without depending on a live display server

## How To Exercise And Test It

- inspect the available modes and refresh variants first:
  - `cargo run -- query eDP-1`
- preview preferred-mode selection:
  - `cargo run -- modify --preferred --dry-run`
- preview auto-mode selection:
  - `cargo run -- modify --auto --dry-run`
- preview resolution shorthand with nearest refresh selection:
  - `cargo run -- modify --mode 1920x1080 --refresh 60 --dry-run`
- preview clearing primary state explicitly:
  - `cargo run -- modify --noprimary --dry-run`
- confirm conflict handling for ambiguous input:
  - `cargo run -- modify --mode 1920x1080@59.999 --refresh 60 --dry-run`
- check dynamic completion values:
  - `cargo run -- __complete "" modify --refresh`
  - `cargo run -- __complete "" modify --mode 1920x1080 --refresh`
