# Add Saved Profiles And Apply From query --json

## Why This Exists

Two old backlog items both point at the same missing capability:

- reading a config file
- saving and loading layouts like autorandr

The project now has a stable `query --json` schema, which gives a natural base for a reproducible profile workflow instead of inventing a second export format immediately.

## Scope

- add a profile or apply command that can read a documented config file generated from current state
- support naming saved layouts and matching them to hardware identity using connector plus vendor/product/serial data
- decide whether the first shipped slice should be:
  - `apply FILE`
  - `profile save NAME`
  - `profile apply NAME`
- keep the format and CLI explicit rather than trying to mimic `autorandr` exactly

## Acceptance Criteria

- a user can dump current state, edit or store it, and apply it later without manually retyping all monitor choices
- profile matching is based on stable monitor identity data rather than connector name alone
- errors clearly distinguish schema problems, mismatched hardware, and invalid monitor requests
- README includes at least one end-to-end example for saving and restoring a layout

## Likely Files

- `src/cli/query.rs`
- new profile/apply CLI modules under `src/cli/`
- `README.md`
- `docs/addressed/notes/0009_Allow_reading_a_config_file.md`
- `docs/addressed/notes/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`

## References

- `docs/addressed/notes/0009_Allow_reading_a_config_file.md`
- `docs/addressed/notes/0012_Allow_dumping_the_current_configuration_to_stdout.md`
- `docs/addressed/notes/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`

## Follow-ups

- later work can still add named profile storage on top of the file-based `apply FILE` machinery if it becomes worthwhile

## How This Was Addressed

- added a new public `gnome-randr apply FILE` command that consumes the documented `query --json` schema instead of introducing a separate saved-layout format
- implemented schema validation for saved files, currently accepting schema versions `4` and `5`
- matched saved monitors to current hardware by stable identity (`vendor`, `product`, `serial`) instead of connector names
- restored layout in one full `ApplyMonitorsConfig` payload, including transform, geometry, primary state, matched modes, and typed color-mode properties
- restored managed software brightness and gamma after the layout apply while intentionally skipping `unknown` software-color states that cannot be reproduced faithfully from a saved query
- added tests covering identity-based matching, missing hardware, unsupported schema versions, and unsupported color-mode restoration

## How To Exercise And Test It

- save the current layout:
  - `cargo run -- query --json > work-layout.json`
- preview how the saved file resolves on current hardware:
  - `cargo run -- apply work-layout.json --dry-run`
- preview the persistent path:
  - `cargo run -- apply work-layout.json --persistent --dry-run`
- apply it for real:
  - `cargo run -- apply work-layout.json`
- inspect the restored state afterward:
  - `cargo run -- query --summary`
  - `cargo run -- query --json`
- run focused tests:
  - `cargo test cli::apply::tests::build_apply_configs_matches_monitor_by_identity`
  - `cargo test cli::apply::tests::build_apply_configs_rejects_missing_hardware`
  - `cargo test cli::apply::tests::parse_profile_rejects_unsupported_schema_version`
