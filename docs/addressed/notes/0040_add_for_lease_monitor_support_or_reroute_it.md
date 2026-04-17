# Add For-Lease Monitor Support Or Reroute It

## Why This Exists

`gdctl` exposes `--for-lease-monitor`, and it is one of the clearest remaining native feature gaps between the upstream CLI and this repo.

It is also exactly the kind of feature that might hit backend-specific or compositor-policy limits, so it deserves its own bounded note.

## Scope

- investigate the current Mutter `for-lease-monitor` / `monitors-for-lease` surface in the checked-out `mutter/` tree and live runtime
- decide whether `gnome-randr` can support it honestly with the existing transactional planner and apply path
- if yes, add a typed CLI path and query visibility
- if not, reroute it into an addressed backend-limit note with concrete evidence

## Acceptance Criteria

- there is a truthful final answer for leasing support in this repo
- if supported, the feature is queryable, writable, documented, and validated
- if unsupported or backend-limited, the limitation is documented explicitly and removed from the active queue

## Likely Files

- `src/cli/modify/mod.rs`
- `src/cli/query.rs`
- `src/display_config/proxied_methods.rs`
- `src/cli/apply.rs`
- `README.md`

## References

- `mutter/doc/man/gdctl.rst`
- `mutter/tools/gdctl`
- `src/cli/modify/planner.rs`
- `src/display_config/raw.rs`

## Follow-ups

- if monitor leasing is workable but large, split implementation from documentation rather than keeping this note as an oversized umbrella

## How This Was Addressed

- verified from the live XML and upstream `gdctl` source that monitor leasing is a real typed `ApplyMonitorsConfig` top-level property (`monitors-for-lease`), not a speculative feature gap
- added repeated `modify --for-lease-monitor CONNECTOR` support that removes leased connectors from active logical monitors and sends them through the top-level `monitors-for-lease` property
- added typed `is_for_lease` visibility to `query` text and JSON so leasing state is inspectable without raw property parsing
- extended `apply FILE` so saved layouts can round-trip leasing state through the same `query --json` schema rather than needing a separate profile format
- kept the implementation honest to Mutter's model: leased monitors must not remain part of active logical monitors

## How To Exercise And Test It

- preview leasing a monitor:
  - `cargo run -- modify --for-lease-monitor DP-2 --dry-run`
- inspect the typed lease state in text:
  - `cargo run -- query --summary`
- inspect the typed lease state in JSON:
  - `cargo run -- query --json | jq '.monitors[] | {connector, is_for_lease}'`
- verify saved-layout round-tripping:
  - `cargo test build_apply_configs_collects_monitors_for_lease`
- verify the full suite:
  - `cargo fmt && cargo test`
