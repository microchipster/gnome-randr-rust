# Reconcile Recent Work With The Backlog And Publish A Parity Matrix

## Why This Exists

Several old backlog items are no longer accurate because recent work already landed software brightness, richer query output, JSON dumping, and shell completions. Leaving them untouched makes `docs/unaddressed/` lie about what is still active.

This is the lowest-effort, highest-clarity slice before more feature work lands.

## Scope

- move genuinely completed notes from `docs/unaddressed/` to `docs/addressed/notes/`
- add the required addressed-note sections from `backlog.md`
- publish a concise parity matrix in `README.md` or a dedicated doc that marks features as:
  - implemented
  - planned
  - limited by Mutter
  - unsupported with the current backend
- rewrite stale routing text so the next contributor can find the next real task without reading closed issues first

## Acceptance Criteria

- `docs/unaddressed/` no longer contains notes that are already shipped in current `main`
- recent shipped work such as software brightness and `query --json` has truthful addressed notes
- the parity matrix points readers to the next active notes in `docs/unaddressed/notes/`
- `README.md` no longer leaves the feature surface ambiguous compared to `xrandr`

## Likely Files

- `README.md`
- `docs/addressed/notes/`
- `docs/unaddressed/issues/0012_Allow_dumping_the_current_configuration_to_stdout.md`
- `docs/unaddressed/issues/0016_Feature_request__set_brightness.md`
- `docs/unaddressed/issues/0024_Installation.md`

## Follow-ups

- do not fold new feature work into this cleanup note
- after this lands, the next implementation note should be `0020_accept_displayed_scale_values_when_matching_supported_scales.md`

## How This Was Addressed

- moved completed notes out of `docs/unaddressed/` and into `docs/addressed/notes/` so the active backlog only contains real remaining work
- published a concise capability-parity status section in `README.md` that distinguishes implemented features, planned work, Mutter-limited areas, and unsupported backend mismatches
- updated `docs/unaddressed/notes/0000_xrandr_capability_parity_routing.md` so the next active slice is `0020_accept_displayed_scale_values_when_matching_supported_scales.md`
- left `docs/unaddressed/issues/0024_Installation.md` active because only part of it is already covered by the README; the remaining install-path and naming questions are still open

Concrete file pointers:

- `README.md`
- `docs/addressed/notes/0010_reconcile_recent_work_with_the_backlog_and_publish_a_parity_matrix.md`
- `docs/addressed/notes/0012_Allow_dumping_the_current_configuration_to_stdout.md`
- `docs/addressed/notes/0016_Feature_request__set_brightness.md`
- `docs/unaddressed/notes/0000_xrandr_capability_parity_routing.md`

## How To Exercise And Test It

- confirm the addressed notes exist:
  - `ls docs/addressed/notes`
- confirm the next active roadmap note is now `0020`:
  - `ls docs/unaddressed/notes`
  - read `docs/unaddressed/notes/0000_xrandr_capability_parity_routing.md`
- read the parity summary in `README.md` and make sure it matches the current shipped feature surface
- sanity-check that old completed items are no longer under `docs/unaddressed/issues/`
