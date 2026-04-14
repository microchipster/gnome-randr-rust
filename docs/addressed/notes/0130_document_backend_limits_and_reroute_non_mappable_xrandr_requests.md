# Document Backend Limits And Reroute Non-Mappable xrandr Requests

## Why This Exists

Some historical requests are unlikely to become first-class features with the current Mutter `DisplayConfig` backend, even if they exist in `xrandr`:

- custom modelines and arbitrary mode injection
- arbitrary transform matrices, panning, and framebuffer hacks
- provider and CRTC selection features tied to X11 RandR internals

The backlog should say this plainly instead of implying these items are just waiting for implementation time.

## Scope

- document which `xrandr` capabilities are intentionally unsupported with the current backend and why
- reroute or close backlog items that are better understood as backend limits rather than implementation omissions
- call out the one important unresolved case separately: custom mode support may require a different backend strategy or upstream Mutter support, not just more CLI work
- update README and backlog notes so contributors know what not to pick up accidentally

## Acceptance Criteria

- `README.md` or a dedicated doc lists the main unsupported feature classes explicitly
- notes such as `docs/unaddressed/issues/0028_Can_t_change_mode.md` are rerouted honestly rather than left as if a normal CLI patch would solve them
- the parity matrix created by `0010_reconcile_recent_work_with_the_backlog_and_publish_a_parity_matrix.md` points to this explanation for unsupported areas
- no active note in `docs/unaddressed/notes/` promises arbitrary X11-style transform, provider, or modeline support unless the backend story changes first

## References

- `docs/addressed/notes/0028_Can_t_change_mode.md`
- `src/display_config/raw.rs`
- Mutter `org.gnome.Mutter.DisplayConfig` XML documentation

## Follow-ups

- if future backend work makes custom modes feasible, split that into a new concrete implementation note rather than silently broadening this one

## How This Was Addressed

- documented the unsupported feature classes directly in `README.md` under a dedicated `## Backend Limits` section
- updated the parity routing note to mark the implementation track complete for the current backend and stop advertising any remaining active note for X11-only or non-mappable features
- rerouted the custom-mode issue into an addressed note that explicitly states the limit is backend-level, not an omitted CLI flag
- left the door open for future backend work by treating custom modes as a separate backend problem rather than pretending more CLI work alone would solve it

## How To Exercise And Test It

- read the backend-limit summary in the README:
  - `rg -n "Backend Limits|custom modelines|provider and CRTC" README.md`
- inspect the final parity routing note:
  - `sed -n '1,120p' docs/unaddressed/notes/0000_xrandr_capability_parity_routing.md`
- inspect the rerouted custom-mode issue note:
  - `sed -n '1,200p' docs/addressed/notes/0028_Can_t_change_mode.md`
