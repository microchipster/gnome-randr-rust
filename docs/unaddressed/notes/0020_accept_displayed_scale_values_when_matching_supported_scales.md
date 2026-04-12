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
