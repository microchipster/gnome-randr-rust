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

- `docs/unaddressed/issues/0028_Can_t_change_mode.md`
- `src/display_config/raw.rs`
- Mutter `org.gnome.Mutter.DisplayConfig` XML documentation

## Follow-ups

- if future backend work makes custom modes feasible, split that into a new concrete implementation note rather than silently broadening this one
