# PR #18: Add XY movement & refactor monitor types

**Opened by** FractalMachinist **at** 2022-06-06T23:51:55Z

## Body

# Main Change
Created an Action to handle changing monitor position in X and Y. Tied this action into the CLI. Tested functionality - tests pass.

# Secondary Changes
This change also led me to refactor most of display_config. physical_monitor and virtual_monitor now share distinct types for:
- Transformation
    - Displacement
        - Handles X, Y, and Scale
        - Handles parsing CLI instructions
    - Orientation
        - Handles u32 encoding Rotation and Flipping
        - Handles parsing CLI instructions
- Connector/Vendor/Product/Serial (now MonitorDescription)

There are no longer two 'rotation' types ("Actions::Rotation" and "LogicalMonitor::Transform") - Actions::Rotation has been superseded, extending the CLI's rotation capabilities to include flipping, without breaking previous behavior.

I'm relatively new to Open Source / GitHub tradition, and I'm very very new to Rust. If there's something I can do to help in any way, or if I've made a mistake in some way, please let me know.

## Review comments (on diffs)

---
**maxwellainatchi** at 2022-07-06T11:37:46Z on src/cli/modify/mod.rs (pos 72)
I don't like that there's a repetition between `--scale` and `--displacement`, and I also am not a fan of comma-separated syntax and would like to keep it to a minimum.

Can you make this `--position` and allow position and scale to be set separately? Otherwise the behavior if you set both `--displacement` and `--scale` is unclear, and you end up with a more opaque `--displacement 100,88,1` instead of `--position 100,88 --scale 1` which reads a lot clearer to me.

---
**maxwellainatchi** at 2022-07-06T11:49:11Z on src/cli/modify/actions/displace.rs (pos 8)
I like what you've done here with the string parsing, but there are a couple issues with it:

1. Like I said below, I'd rather keep the scale as a separate argument and make this just the `x,y`
1. The string parsing is now in the `display_config` directory, which is supposed to be a separate library. I'd rather avoid putting CLI details in the library wherever possible, which is why I had the rotation struct to begin with. This is a relatively minor transgression though so I don't mind much.

I would make a `position` struct that gets parsed to an x and y value and use that as the property on this Action.

## Reviews

---
**maxwellainatchi** at 2022-07-06T11:51:01Z — CHANGES_REQUESTED
Hey there, sorry for the delay in response. This looks great! I'm also fairly new to OSS and Rust, this is actually my first and only Rust project.

The refactors you did are a nice improvement to code cleanliness, and it's a great new feature.

There are a couple of changes I'd like you to make based on my vision for this, but there's no way you would've known that since I haven't put it to words yet anywhere.
