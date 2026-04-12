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
