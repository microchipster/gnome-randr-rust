# Issue #27: when try to rotate left or right encounter error D-Bus error: Logical monitors not adjacent (org.freedesktop.DBus.Error.InvalidArgs)

**Opened by** yuchinchenTW **at** 2024-06-26T05:48:26Z

## Body

normal and inverted rotation works well

but when trying on left or right rotation encounter error

```
setting rotation to right
setting mode to 1920x1200@40.012973785400391
D-Bus error: Logical monitors not adjacent (org.freedesktop.DBus.Error.InvalidArgs)
Logical monitors not adjacent
```

```
 gnome-shell --version
GNOME Shell 42.9
```

Not sure whats wrong have tried with directly using Settings->Display->orientation works well

## How This Was Addressed

- `0070` added planner-side geometry reflow after mode, scale, or rotation changes when the user did not request an explicit new placement
- left and right rotations now recompute final extents from post-transform geometry and shift affected right-side/below neighbors instead of leaving stale pre-rotation adjacency in place
- planner tests now cover the exact failure shape that used to leave right-side neighbors at stale x coordinates after a width-changing rotation

## How To Exercise And Test It

- on a multi-monitor machine, preview a left or right rotation that used to fail:
  - `cargo run -- modify HDMI-1 --rotate right --dry-run`
  - `cargo run -- modify HDMI-1 --rotate left --dry-run`
- inspect the resulting logical-monitor coordinates after a real apply:
  - `cargo run -- query`
- in any environment, run the targeted regression test:
  - `cargo test cli::modify::planner::tests::reflow_moves_right_neighbors_after_rotation_changes_width`
