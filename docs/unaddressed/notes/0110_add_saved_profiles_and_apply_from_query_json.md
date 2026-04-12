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
- `docs/unaddressed/issues/0009_Allow_reading_a_config_file.md`
- `docs/unaddressed/issues/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`

## References

- `docs/unaddressed/issues/0009_Allow_reading_a_config_file.md`
- `docs/unaddressed/issues/0012_Allow_dumping_the_current_configuration_to_stdout.md`
- `docs/unaddressed/issues/0030__Feature_Request__Allow_to_save_and_load_configs__like_autor.md`

## Follow-ups

- if the first slice is still too large, ship `apply FILE` before named profile storage
