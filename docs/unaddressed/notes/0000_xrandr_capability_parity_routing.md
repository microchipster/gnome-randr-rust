# Build Toward xrandr Capability Parity

## Status

Routing note only. Do not try to ship this as one change.

Pick the next numbered follow-up note that is still true, implement the smallest honest slice, then update this routing note if the order changes.

## Goal

Bring `gnome-randr` as close as practical to the monitor-management power people relied on from `xrandr`, while keeping a more modern CLI and aligning with GNOME Wayland's model instead of copying X11 syntax.

Capability parity matters more than syntax parity.

## What Is Already Landed

- `query` can report logical monitor state, physical monitor state, enabled state for disabled outputs, typed rotation/reflection state, typed color-mode and underscanning visibility, native power-save/backlight/luminance state, software brightness state, software gamma state, JSON output, raw property maps, and xrandr-style logical monitor list views.
- `modify` can already change mode by id or resolution, choose nearest refresh, use preferred or auto mode selection, disable outputs with `--off`, set absolute positions with `--position` / `--pos`, place outputs relative to each other with `--left-of` / `--right-of` / `--above` / `--below`, request same-as mirroring with local clone preflight, reflow adjacent layouts after geometry-changing rotations, reflect outputs with `--reflect`, set supported color modes with `--color-mode`, set native layout mode, power-save, backlight, and luminance controls, scale including displayed rounded scale values, rotation, primary or noprimary state, software brightness, and software gamma, and it now plans changes through one full transactional config payload internally.
- `apply FILE` can restore a saved `query --json` layout by monitor identity, replay supported layout-mode changes, and replay managed software brightness and gamma afterward.
- dynamic shell completions and single-monitor defaults are already in place.

## Ordered Follow-up Notes

- None. The parity track is functionally complete for the current backend.

## Why The Work Is Split This Way

- `0020` through `0040` were the easiest parity wins and mostly extended existing query/modify flows.
- `0050` established the architectural prerequisite for the bigger layout features.
- `0060` through `0080` established the core topology-control foundation after the planner, including same-as mirroring where Mutter accepts it.
- `0090` through `0120` landed software color, the supported reflection/property-control slice, file-based saved-layout restore, and the Wayland-native layout-mode/power-save/backlight/luminance controls.
- `0130` closes the loop by documenting what the Mutter D-Bus backend still cannot represent and rerouting those requests out of the active implementation track.

## Existing Backlog This Reroutes

- None. The backend-limit reroutes have been moved into addressed notes.

## Success Criteria For The Whole Track

Call this near-parity when `gnome-randr` can reliably do the common real-world monitor tasks people used `xrandr` for:

- inspect current outputs and modes
- choose preferred, auto, or refresh-constrained modes
- enable and disable outputs
- set primary and unset primary
- position outputs absolutely and relatively
- mirror outputs where Mutter allows it
- rotate and reflect outputs
- scale outputs without hidden precision traps
- control software brightness and gamma
- save and restore layouts

The remaining X11-only or backend-mismatched features should be documented explicitly instead of being left ambiguous.
