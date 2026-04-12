# Add Richer query Views And Property Inspection

## Why This Exists

Current `query` output is good, but it still lacks some of the inspection affordances people used from `xrandr`, especially property-heavy views and monitor-focused summaries.

This is a good bounded slice because the data is already available from Mutter and much of it is already present in JSON output.

## Scope

- add `query --properties` / `--prop` to surface monitor properties in the text UI
- add `query --verbose` as a more detailed inspection mode rather than a no-op alias
- add `query --listmonitors` and `query --listactivemonitors` views oriented around logical monitors instead of full mixed dumps
- make sure the text and JSON outputs agree on naming and meaning for the shared fields

## Acceptance Criteria

- users can inspect monitor properties such as underscanning-related state from the CLI without switching to JSON
- `--listmonitors` and `--listactivemonitors` provide concise monitor-centric output rather than the default full dump
- the new flags are documented in help text and README examples
- tests cover formatting and flag interactions

## Likely Files

- `src/cli/query.rs`
- `src/cli/mod.rs`
- `README.md`

## Follow-ups

- do not add mutation behavior to `query`
- output-control property mutation belongs in `0100_add_reflection_and_supported_monitor_property_controls.md`
