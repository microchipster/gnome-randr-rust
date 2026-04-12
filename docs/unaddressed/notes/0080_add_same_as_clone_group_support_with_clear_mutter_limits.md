# Add same-as Clone Group Support With Clear Mutter Limits

## Why This Exists

Mirroring is one of the biggest remaining parity asks. The repo can already read mirror-capability state and clone-compatible outputs, but there is still no CLI path to ask for `same-as` style behavior.

At the same time, the historical mirroring issue shows that Mutter rejects at least some overlapping-monitor layouts, so this note must stay honest about backend constraints.

## Scope

- add a user-facing clone or `same-as` operation on top of the multi-output planner
- prefer the logical-monitor model Mutter already exposes when multiple connectors share one logical monitor
- when hardware clone constraints require it, use `Resources.outputs[].clones` and `possible_crtcs` to validate what is likely feasible before apply
- produce a helpful error when the requested mirror layout is rejected by Mutter validation

## Acceptance Criteria

- users can request a straightforward mirror operation between compatible outputs from the CLI
- preflight validation catches obviously impossible clone requests before D-Bus apply when the resource model already proves they cannot work
- failure messages distinguish between local validation failure and Mutter-side overlap rejection
- docs clearly state that partial mirroring may still be limited by Mutter's layout rules

## Likely Files

- planner modules introduced by `0050_build_a_transactional_multi_output_monitor_planner.md`
- `src/display_config/resources.rs`
- `src/cli/modify/mod.rs`
- `README.md`

## References

- `docs/unaddressed/issues/0020__Feature_Request___Mirroring_config.md`

## Follow-ups

- if this note proves too large, split it into:
  - straightforward one-to-one clone support
  - advanced partial-mirroring behavior and error reporting
