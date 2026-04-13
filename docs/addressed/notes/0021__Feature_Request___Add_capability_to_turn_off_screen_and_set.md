# Issue #21: [Feature Request]: Add capability to turn off screen and set position

**Opened by** bzhpwr **at** 2023-01-28T13:27:06Z

## Body

Hi,

First, thank you for this cool tool :)

With gnome-randr.py, i use something like this as a keyboard shortcut

```
  # Turn off second screen when using first monitor as a TV
  gnome-randr.py --output "DP-1" --auto --primary --output "DP-3" --off
  # Turn on second screen when I need second screen
  gnome-randr.py --output "DP-1" --auto --primary --output "DP-3" --left-of "DP-1"  --auto
```

Is it possible to add _left-of_ and _right-of_ option to set the screen position and maybe the _off_ option to turn off one screen ?

Thank U :)

## Comments

Historical maintainer discussion preserved from the original issue.

## How This Was Addressed

- `modify --off` landed in `0060` as a real planner-level output disable
- `modify --left-of`, `--right-of`, `--above`, and `--below` landed in `0070` as planner-based relative placement controls
- `modify --position` / `--pos` also exists for explicit coordinates when relative placement is not desired

## How To Exercise And Test It

- preview turning off one output:
  - `cargo run -- modify HDMI-1 --off --dry-run`
- preview relative placement:
  - `cargo run -- modify HDMI-1 --left-of eDP-1 --dry-run`
  - `cargo run -- modify HDMI-1 --right-of eDP-1 --dry-run`
- inspect the resulting layout model:
  - `cargo run -- query`
  - `cargo run -- query --json | jq '.logical_monitors[] | {x, y, monitors}'`
