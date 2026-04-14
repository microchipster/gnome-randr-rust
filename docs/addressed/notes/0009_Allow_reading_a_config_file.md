# Issue #9: Allow reading a config file

**Opened by** maxwellainatchi **at** 2021-10-21T14:04:52Z

## Body



## How This Was Addressed

- added `gnome-randr apply FILE`, which reads a saved layout file generated from `gnome-randr query --json`
- the file format is the documented query JSON schema rather than a separate ad hoc config format
- applying a saved file restores layout by monitor identity and can replay managed software brightness and gamma afterward

## How To Exercise And Test It

- create a config file:
  - `cargo run -- query --json > work-layout.json`
- preview it:
  - `cargo run -- apply work-layout.json --dry-run`
- apply it:
  - `cargo run -- apply work-layout.json`
