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

- `docs/addressed/notes/0020__Feature_Request___Mirroring_config.md`

## Follow-ups

- if this note proves too large, split it into:
  - straightforward one-to-one clone support
  - advanced partial-mirroring behavior and error reporting

## How This Was Addressed

- added `modify --same-as CONNECTOR` so gnome-randr now requests mirroring by moving the target connector into the reference connector's logical monitor instead of trying to overlap separate logical monitors
- extended the transactional planner with `clone_with(...)` so active outputs can be merged into an existing clone group and disabled outputs can be added back into one
- added local clone preflight using Mutter's resource model: clone capability, shared possible CRTCs, and a compatible target mode matching the reference output's current width and height
- when GNOME's DisplayConfig backend still rejects the clone apply, gnome-randr now surfaces a clearer `MutterRejectedClone` error explaining that partial mirroring remains constrained by Mutter validation rules
- added tests for planner clone-group merging, enabling a disabled output into a clone group, clone preflight success, and clone preflight failure

## How To Exercise And Test It

- inspect outputs and confirm the connectors you want to mirror:
  - `cargo run -- query`
- preview a same-as mirror request:
  - `cargo run -- modify HDMI-1 --same-as eDP-1 --dry-run`
- if you have two compatible outputs available, apply it for real and inspect the resulting shared logical monitor:
  - `cargo run -- modify HDMI-1 --same-as eDP-1`
  - `cargo run -- query`
  - `cargo run -- query --json | jq '.logical_monitors[] | .monitors'`
- confirm local validation catches obviously bad clone requests:
  - `cargo run -- modify eDP-1 --same-as eDP-1 --dry-run`
- if Mutter still rejects a partial mirroring layout on your hardware, confirm the error is labeled as backend rejection rather than a local capability failure.
