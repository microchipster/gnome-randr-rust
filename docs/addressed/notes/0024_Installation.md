# Issue #24: Installation

**Opened by** pokorj54 **at** 2023-06-10T15:41:48Z

## Body

In installation it should be mentioned that you need pkg-config.

The name `gnome-randr` is already taken by the command that you are trying to replace, but you have different arguments, it should not be the same.

The installation is done in a home folder without any choice to change it. Which means manually installing it somewhere else, or need to specify the exact path somewhere if it is executed by something else than a user in its own terminal.

## How This Was Addressed

- the README installation section now explicitly mentions `pkg-config` as a prerequisite alongside Cargo/Rust
- the README now documents both published-crate install (`cargo install gnome-randr`) and local-tree install (`cargo install --path .`)
- the README now documents Cargo's default install location behavior and the `--root /your/prefix` override for choosing a custom install prefix
- the README now explicitly calls out that the installed binary is named `gnome-randr` and explains that users can invoke it by full path or change `PATH` ordering if another binary with the same name already exists

## How To Exercise And Test It

- read the installation section in the README:
  - `sed -n '12,24p' README.md`
- verify the local-tree install command still works:
  - `cargo install --path .`
- verify a custom install root is now documented and works in principle:
  - `cargo install --path . --root /tmp/gnome-randr-test-root`
  - `/tmp/gnome-randr-test-root/bin/gnome-randr --help`
