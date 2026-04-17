# PR Summary

## Summary

- bring `gnome-randr` to practical xrandr capability parity on GNOME Wayland, then align the native control surface and semantics with upstream `gdctl`
- add full typed support for modern Mutter display controls: layout planning, mirroring, reflection, color mode, rgb range, native power/backlight/luminance, software color, and saved layout restore
- document the completed parity and gdctl-alignment workstreams, archive addressed backlog items, and tighten the maintainer/user docs around the final model

## Details

### Query And JSON Surface

- expand `query` into a typed inspection interface with text, summary, verbose, properties, listmonitors/listactivemonitors, and JSON modes
- evolve the JSON schema through versions `1` to `8`, adding typed software color state, reflection, color mode, rgb range, native backlight/luminance, power-save, lease state, and layout metadata while preserving file-based restore compatibility
- keep disabled outputs queryable by connector and expose stable hardware identity (`vendor`, `product`, `serial`) for later restore/apply flows

### Layout And Output Control

- move output mutation under one coherent `modify` command with single-output defaulting when exactly one connector exists
- add mode selection by id or resolution, refresh selection, preferred/auto, scale matching against rounded query values, primary/noprimary, output disable, absolute placement, relative placement, same-as mirroring, reflection, color mode, rgb range, leasing, native power-save, backlight, luminance, and layout-mode changes
- replace ad hoc one-output application with a transactional planner over full `ApplyMonitorsConfig` payloads, including geometry-aware reflow and clone-group handling
- support top-level `ApplyMonitorsConfig` properties such as `layout-mode` and `monitors-for-lease`

### Software Color

- implement exact LUT-based software brightness with `linear`, `gamma`, and `filmic` filters
- preserve Night Light / ICC / external LUT state by adopting the live LUT as the baseline when it no longer matches the last managed state
- extend the same preserved-LUT pipeline to typed per-channel software gamma (`--gamma R[:G:B]`) with xrandr-compatible semantics
- surface both `software_brightness` and `software_gamma` in query text and JSON, and restore them after `apply FILE` when the saved state is reproducible

### Saved Layout Restore

- add `gnome-randr apply FILE` using the documented `query --json` schema instead of inventing a second config format
- match monitors by stable identity rather than connector names, resolve modes by exact id first then resolution/nearest refresh, and restore typed color properties plus managed software color after layout apply
- allow saved layouts to carry layout-mode changes and leased-monitor state through the same typed schema

### gdctl Alignment

- treat upstream `gdctl` as the semantic/native reference for supported Mutter controls, enums, and capability boundaries
- land non-breaking `show`, `set`, and `--verify` aliases where semantics truly match
- extend typed monitor-control parity to include `sdr-native` color mode, `rgb-range`, and for-lease monitors
- preserve `gnome-randr`'s higher-level workflow layer (`query --json`, `apply FILE`, software color, richer one-shot modify flows) instead of cloning `gdctl` syntax wholesale

### Docs And Maintenance

- replace the sprawling in-progress backlog with addressed notes documenting each completed slice and the remaining backend limits
- clean up the README into a more compact operational guide while still documenting native controls, software color, saved layouts, JSON output, backend limits, and gdctl alignment
- add `tutorial.md`, a maintainer-focused walkthrough covering Rust concepts used here, Mutter/Wayland concepts, end-to-end code paths, risk areas, and a maintenance workflow
- update compatibility/backlog docs so both the xrandr-parity and gdctl-alignment workstreams are archived as complete for the current backend

## Verification

- `cargo fmt`
- `cargo test`
- `cargo install --path .`
- `bash -n <(cargo run --quiet -- completions bash)`
- `zsh -n <(cargo run --quiet -- completions zsh)`
- `cargo run --quiet -- completions fish >/dev/null`
- representative dry-runs for layout, leasing, native controls, software color, and saved profile apply
- `query --json` and `apply FILE --dry-run` round-trip checks across schema upgrades
