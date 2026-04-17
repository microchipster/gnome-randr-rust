# Extend Remaining gdctl Monitor Control Parity

## Why This Exists

After the parity track through `0120`, the biggest remaining functional gap versus `gdctl` appears to be the small set of typed monitor controls that upstream still exposes and this repo does not fully model yet.

The likely examples are:

- additional color-mode variants such as `sdr-native`
- `rgb-range`

These are native typed monitor controls, so they belong in this repo if the backend exposes them cleanly.

## Scope

- verify the current upstream `gdctl` monitor control surface on the checked-out `mutter/` tree and the live backend
- extend typed enums and query/modify/apply support for any clearly supported monitor controls that are still missing here
- start with:
  - current `ColorMode` parity with `gdctl`
  - `rgb-range`, if the backend surfaces it cleanly enough to support honestly

## Acceptance Criteria

- any newly added monitor control is typed, queryable, writable, and documented
- unsupported or partially exposed controls are rejected explicitly instead of hidden behind raw property plumbing
- the repo stays aligned with the upstream `gdctl` surface rather than inventing a parallel naming scheme

## Likely Files

- `src/display_config/proxied_methods.rs`
- `src/cli/modify/mod.rs`
- `src/cli/query.rs`
- `src/cli/apply.rs`
- `README.md`

## References

- `mutter/doc/man/gdctl.rst`
- `mutter/tools/gdctl`
- `src/display_config/proxied_methods.rs`
- `src/cli/modify/mod.rs`

## Follow-ups

- if `rgb-range` and remaining color-mode work turn out to be different backend paths, split them into separate notes instead of stretching this one

## How This Was Addressed

- verified from the checked-out upstream `mutter/` sources that `gdctl` and Mutter still support `sdr-native` color mode and typed `rgb-range`
- extended the typed property layer with `ColorMode::SdrNative`, a new `RgbRange` enum, and `ApplyMonitorProperty::RgbRange`
- added `modify --rgb-range auto|full|limited` and extended `modify --color-mode` to include `sdr-native`
- added completion support for the new color mode and `--rgb-range`
- extended `query` text and JSON to expose typed `rgb_range`, and extended `apply FILE` so saved layouts can round-trip both `color_mode` and `rgb_range`
- kept unsupported live hardware states explicit: if the current backend does not advertise a typed `rgb-range` property or a requested color mode, `gnome-randr` errors clearly instead of hiding the request behind raw property plumbing

## How To Exercise And Test It

- inspect the new monitor-control help:
  - `cargo run -- modify --help`
- preview a new upstream color mode when supported:
  - `cargo run -- modify eDP-1 --color-mode sdr-native --dry-run`
- preview rgb-range when supported:
  - `cargo run -- modify eDP-1 --rgb-range limited --dry-run`
- inspect typed state in query text and JSON:
  - `cargo run -- query --summary`
  - `cargo run -- query --json | jq '.monitors[] | {connector, color_mode, supported_color_modes, rgb_range}'`
- verify apply-file round-tripping still works with the newer schema:
  - `tmpfile=$(mktemp) && cargo run -- query --json > "$tmpfile" && cargo run -- apply --dry-run "$tmpfile" && rm -f "$tmpfile"`
- run targeted tests:
  - `cargo test build_apply_configs_validates_rgb_range_support`
  - `cargo test properties_text_includes_raw_property_sections`
