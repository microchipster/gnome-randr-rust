# Accept Displayed Scale Values When Matching Supported Scales

## Why This Exists

`query` currently shows friendly rounded scale values, but `modify --scale` may require a much more precise float than what is displayed. That makes a normal user-visible value like `1.75` fail even when it obviously refers to an advertised scale.

This is the clearest low-hanging parity bug and is already reported in `docs/unaddressed/issues/0026__--scale__requires_more_precision_than_is_displayed.md`.

## Scope

- when `modify --scale` is provided, match the requested value against the selected mode's supported scales within a small tolerance
- use the exact supported scale value when applying config instead of passing the rounded user input through unchanged
- keep `query` readable while making JSON continue to expose exact values
- explain the behavior in help text so users know that displayed values are accepted directly

## Acceptance Criteria

- `gnome-randr modify CONNECTOR --scale 1.75` succeeds when `query` shows `x1.75` for the chosen mode
- the implementation prefers an exact match when present and otherwise chooses the nearest scale within a bounded tolerance
- out-of-range values still fail with a useful error that points the user back to `query`
- completions remain based on the displayed values users can actually type

## Likely Files

- `src/cli/modify/mod.rs`
- `src/cli/common.rs`
- `src/cli/query.rs`
- `src/cli/complete.rs`

## References

- `docs/unaddressed/issues/0026__--scale__requires_more_precision_than_is_displayed.md`

## Follow-ups

- keep this note focused on scale matching only
- do not mix it with broader mode-selection sugar; that belongs in `0030_add_preferred_auto_refresh_and_noprimary_to_modify.md`

## How This Was Addressed

- added tolerant scale matching in `src/cli/common.rs` so `modify --scale` accepts the rounded values shown in `query` and resolves them back to the nearest exact supported float within half a displayed step
- updated `src/cli/modify/mod.rs` to resolve scale values against the selected mode's advertised `supported_scales` before building the apply config
- improved scale errors so invalid values point users back to `gnome-randr query CONNECTOR`
- updated scale help text to say that displayed values from `query` can be typed directly

Concrete file pointers:

- `src/cli/common.rs`
- `src/cli/modify/mod.rs`
- `src/cli/modify/actions/scale.rs`

## How To Exercise And Test It

- inspect the advertised scales for one output:
  - `cargo run -- query CONNECTOR`
- apply one of the displayed scale values directly:
  - `cargo run -- modify CONNECTOR --mode MODE --scale 1.75 --dry-run`
- on a fractional-scaling setup where Mutter exposes a more precise internal float, repeat without `--dry-run` and confirm the change succeeds even though `query` only showed the rounded value
- verify that an unsupported value still fails with a helpful message:
  - `cargo run -- modify CONNECTOR --mode MODE --scale 1.73 --dry-run`
