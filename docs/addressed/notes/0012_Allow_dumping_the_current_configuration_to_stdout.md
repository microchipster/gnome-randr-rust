# Issue #12: Allow dumping the current configuration to stdout

**Opened by** maxwellainatchi **at** 2021-10-22T13:44:27Z

## Body



## How This Was Addressed

This behavior is shipped.

`gnome-randr query` prints the current display state to stdout in text form, and `gnome-randr query --json` now provides a documented machine-readable dump for scripts.

Concrete file pointers:

- `src/cli/query.rs`
- `src/cli/mod.rs`
- `README.md`

In practice, the original ask ended up covered in two useful ways:

- default text dumping for humans
- structured JSON dumping for tools and saved-layout workflows

## How To Exercise And Test It

- dump the current display state in text form:
  - `cargo run -- query`
- dump the same state in JSON form:
  - `cargo run -- query --json`
- dump one connector only in JSON form:
  - `cargo run -- query eDP-1 --json`
- verify that `--summary` and `--json` intentionally conflict:
  - `cargo run -- query --summary --json`
