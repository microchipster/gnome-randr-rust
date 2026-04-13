# Issue #20: [Feature Request]: Mirroring config

**Opened by** Merrit **at** 2022-07-13T20:03:21Z

## Body

Hi :wave: nice tool you've made, thanks for your work!

Would be great to be able to configure display mirroring, as GNOME itself currently only allows either extended or mirror all.

Example: I have three displays attached and want to mirror 2 as my "primary", leaving the 3rd as an extended.

## Comments

Historical discussion preserved from the original issue.

## How This Was Addressed

- `modify --same-as CONNECTOR` now mirrors one output onto another by building a shared logical monitor through the transactional planner instead of attempting the old overlapping-logical-monitor approach that Mutter rejected
- gnome-randr now performs local clone preflight using Mutter's resource model before apply, then reports a clearer backend-limits error if GNOME's DisplayConfig validation still rejects a partial mirroring layout
- this addresses the requested user-facing mirroring capability while keeping the old overlap failure mode documented as a Mutter constraint rather than the intended implementation path

## How To Exercise And Test It

- inspect the candidate outputs first:
  - `cargo run -- query`
- preview a mirror request:
  - `cargo run -- modify HDMI-1 --same-as eDP-1 --dry-run`
- if your hardware exposes a compatible pair, apply it and inspect the shared logical monitor:
  - `cargo run -- modify HDMI-1 --same-as eDP-1`
  - `cargo run -- query`
  - `cargo run -- query --json | jq '.logical_monitors[] | .monitors'`
- if Mutter still rejects the layout, confirm the error explains that GNOME's DisplayConfig backend rejected the clone request rather than reporting a local capability mismatch.
