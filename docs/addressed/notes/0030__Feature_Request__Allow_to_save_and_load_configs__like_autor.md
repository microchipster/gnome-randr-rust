# Issue #30: [Feature Request] Allow to save and load configs (like autorandr)

**Opened by** nonZero **at** 2025-12-07T17:37:53Z

## Body

see https://github.com/phillipberndt/autorandr

## How This Was Addressed

- landed the first honest saved-layout slice as a file-based workflow built on the documented `query --json` schema:
  - save with `gnome-randr query --json > file`
  - restore with `gnome-randr apply file`
- monitor matching is based on stable hardware identity, not connector names alone, which covers the core practical need behind autorandr-style restore workflows
- named profile registries were intentionally left for later follow-up rather than blocking the reusable file-based restore path

## How To Exercise And Test It

- save a layout file:
  - `cargo run -- query --json > docked-layout.json`
- preview how it resolves now:
  - `cargo run -- apply docked-layout.json --dry-run`
- restore it later:
  - `cargo run -- apply docked-layout.json`
- inspect the applied result:
  - `cargo run -- query --summary`
