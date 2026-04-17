# Add Non-Breaking gdctl Command And Flag Aliases

## Why This Exists

If `gdctl` is the canonical native reference, users and maintainers should be able to see some obvious CLI correspondence without forcing a full command rewrite.

The right target is additive compatibility, not replacing the current `query` / `modify` / `apply` model.

## Scope

- evaluate and add the most sensible non-breaking aliases between `gdctl` and `gnome-randr`
- likely candidates include:
  - `show` as an alias for `query`
  - `set` as an alias for `modify`
  - `--verify` as an alias for `--dry-run`
  - `--verbose` alignment where it already maps cleanly
- only add aliases when the semantics really match; do not create misleading near-matches

## Acceptance Criteria

- any alias that lands is additive and does not break existing `gnome-randr` workflows
- help text and completions make the alias relationship clear
- aliases are only added where the underlying semantics are genuinely compatible
- command-shape mismatches that are too awkward to alias are left documented rather than forced

## Likely Files

- `src/cli/mod.rs`
- `src/cli/query.rs`
- `src/cli/modify/mod.rs`
- `src/cli/completions.rs`
- `src/cli/complete.rs`
- `README.md`

## References

- `mutter/doc/man/gdctl.rst`
- `mutter/tools/gdctl`
- `src/cli/mod.rs`

## Follow-ups

- if a true `pref`-style native preferences surface is warranted, split it into a separate note rather than overloading this one

## How This Was Addressed

- added `show` as a visible alias for `query`
- added `set` as a visible alias for `modify`
- added `--verify` as an alias for `--dry-run` on `modify` and `apply`
- updated the dynamic completion helper so `show` and `set` use the same runtime completion logic as `query` and `modify`
- updated README and tutorial docs so the alias layer is explicit and intentionally narrow rather than implicit parser trivia
- intentionally did not add a `pref` alias because `gdctl pref` is narrower than `modify` and forcing a near-match would be misleading

## How To Exercise And Test It

- inspect the alias-aware help surfaces:
  - `cargo run -- --help`
  - `cargo run -- show --help`
  - `cargo run -- set --help`
  - `cargo run -- apply --help`
- verify the runtime completion helper understands the aliases:
  - `cargo run -- __complete "e" show --summary`
  - `cargo run -- __complete "" set eDP-1 --same-as`
- verify the dry-run alias is accepted:
  - `cargo run -- set --verify --power-save off`
  - `cargo run -- apply --verify /tmp/layout.json`
