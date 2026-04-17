# Align Native Surface With gdctl

## Status

Routing note only. Do not try to ship this as one change.

Pick the next numbered follow-up note that is still true, implement the smallest honest slice, then update this routing note if the order changes.

## Goal

Use `gdctl` as the canonical native reference for the `org.gnome.Mutter.DisplayConfig` backend without throwing away the higher-level workflows that make `gnome-randr` useful.

The user intent driving this track is:

- conform to `gdctl`'s approaches as the canonical reference
- maintain feature parity with it
- do not compromise on `gnome-randr`'s additional features

## Principles

- align native terminology, enums, capability boundaries, and error semantics with `gdctl` and Mutter where possible
- preserve higher-level `gnome-randr` features such as `query --json`, `apply FILE`, software brightness/gamma, and xrandr-style one-shot workflows
- prefer additive aliases and compatibility layers over disruptive command renames
- make intentional divergences explicit instead of letting them drift implicitly
- if a `gdctl` feature cannot be represented cleanly with the current backend, document or reroute it rather than faking it

## What Is Already Landed

- `gnome-randr` already exposes most of the typed native Mutter surface that `gdctl` focuses on:
  - layout mode
  - color mode
  - rgb-range
  - rotation / reflection
  - luminance
  - power-save
  - backlight support and visibility
  - same-as mirroring where Mutter accepts it
  - for-lease monitor support
- `gnome-randr` also has higher-level features that are intentionally outside `gdctl`'s current scope:
  - `query --json`
  - `apply FILE`
  - software brightness and software gamma
  - shell completions
  - a broader `modify` surface for practical xrandr-style workflows

## Ordered Follow-up Notes

None. The gdctl-alignment track is functionally complete for the current backend.

## Why The Work Is Split This Way

- `0010` established the compatibility matrix first so later work is grounded in a shared map instead of vague alignment goals.
- `0020` handled the safe alias layer for `show`, `set`, and `--verify` without rewriting the existing CLI model.
- `0030` closed the remaining typed monitor-control gaps that look realistically mappable to the current backend, including `sdr-native` color mode and `rgb-range`.
- `0040` closed monitor leasing with a typed implementation, so the gdctl-alignment track now has a complete native compatibility answer for the current backend.

## Success Criteria For The Whole Track

Call this track complete when:

- there is a concrete matrix mapping `gdctl` commands and options to current `gnome-randr` equivalents, intentional divergences, and remaining gaps
- the native enums and typed monitor/display controls that `gdctl` exposes are either supported, explicitly aliased, or explicitly rejected with documentation
- `gnome-randr` remains a higher-level CLI instead of becoming a lossy fork of `gdctl`
- contributors can use `gdctl` as the upstream semantic reference without having to guess where this repo intentionally differs

## References

- `mutter/doc/man/gdctl.rst`
- `mutter/tools/gdctl`
- `src/cli/mod.rs`
- `README.md`

## How This Was Addressed

- this routing note drove the gdctl-alignment track across notes `0010` through `0040`
- the compatibility matrix, additive aliases, remaining typed monitor-property parity, and typed monitor leasing support were all landed and documented
- the note is now moved into `docs/addressed/notes/` because the gdctl-alignment track is functionally complete for the current backend and `docs/unaddressed/` should contain only active work

## How To Exercise And Test It

- inspect the completed matrix and divergence policy in `README.md`
- inspect the maintainer-facing gdctl guidance in `tutorial.md`
- confirm the active gdctl-alignment track is no longer under `docs/unaddressed/notes/`
- inspect the final leasing closeout note:
  - `docs/addressed/notes/0040_add_for-lease_monitor_support_or_reroute_it.md`
