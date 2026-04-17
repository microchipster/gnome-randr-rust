# Publish gdctl Compatibility Matrix And Divergence Policy

## Why This Exists

Before changing the CLI further, this repo needs one explicit document that says how `gdctl` maps to `gnome-randr` today.

Without that matrix, the alignment goal stays fuzzy and later changes risk being inconsistent or overreaching.

## Scope

- add a compatibility matrix covering the current `gdctl` surface (`show`, `set`, `pref`, and their important options)
- map each item to one of:
  - already supported directly
  - supported with a different `gnome-randr` UX
  - worth adding as a compatibility alias
  - not yet supported but mappable
  - backend-limited / intentionally unsupported
- document the intentional long-term policy:
  - `gdctl` is the native reference
  - `gnome-randr` keeps its higher-level workflow layer

## Acceptance Criteria

- a contributor can open one doc and see how `gdctl` maps to this repo today
- the matrix covers the current upstream `gdctl` manpage and implementation, not an outdated guess
- remaining gaps turn into concrete follow-up notes instead of staying hand-wavy
- intentional divergences are named explicitly rather than implied by omission

## Likely Files

- `README.md`
- `tutorial.md`
- `docs/unaddressed/notes/0000_align_native_surface_with_gdctl_routing.md`
- this addressed note

## References

- `mutter/doc/man/gdctl.rst`
- `mutter/tools/gdctl`
- `src/cli/mod.rs`
- `README.md`

## Follow-ups

- if the matrix shows new concrete gaps, split them into follow-up notes instead of expanding this one indefinitely

## How This Was Addressed

- added a `gdctl` compatibility matrix to `README.md` that maps the current upstream `gdctl` surface to direct support, higher-level `gnome-randr` equivalents, higher-level features beyond `gdctl`, and still-open parity gaps
- added an explicit divergence policy to `README.md` and `tutorial.md` so contributors know to align to `gdctl` on semantics and native capability boundaries without rewriting `gnome-randr` into a syntax clone
- updated the gdctl routing note so `0010` is treated as complete and `0020` is now the next active follow-up

## How To Exercise And Test It

- read the compatibility matrix in `README.md`
- compare it against the current upstream manpage:
  - `rg -n "gdctl Compatibility Matrix|Divergence Policy" README.md`
  - `rg -n "SHOW OPTIONS|SET OPTIONS|PREFS OPTIONS" mutter/doc/man/gdctl.rst`
- read the maintainer-facing policy in `tutorial.md`:
  - `rg -n "Using `gdctl` As The Native Reference" tutorial.md`
- confirm the routing note now advances to `0020_add_non_breaking_gdctl_command_and_flag_aliases.md`
