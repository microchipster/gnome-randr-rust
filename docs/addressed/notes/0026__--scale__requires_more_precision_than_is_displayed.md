# Issue #26: `--scale` requires more precision than is displayed

**Opened by** andersk **at** 2024-04-03T06:55:11Z

## Body

On my laptop running GNOME Shell 45.5 with Wayland and fractional scaling enabled, `x1.75` is listed as an available scale factor, but `--scale=1.75` is not accepted. Instead, I need to provide many more decimals: `--scale=1.7518248` works. But there’s no way to discover this without patching out the 2-decimal rounding from `impl Display for Mode`.

Ideally gnome-randr would automatically select the closest of the available scale factors within some tolerance, rather than passing the provided value directly to GNOME which requires the excessive precision.

```console
$ gnome-randr
supports-mirroring: true
layout-mode: logical
supports-changing-layout-mode: true
global-scale-required: false
legacy-ui-scaling-factor: 1

logical monitor 0:
x: 0, y: 0, scale: 1.7518248558044434, rotation: normal, primary: yes
associated physical monitors:
	eDP-1 BOE 0x07c8 0x00000000

eDP-1 BOE 0x07c8 0x00000000
              3840x2160@60.000	3840x2160 	60.00*+   	[x1.00, x1.25, x1.50, x1.75, x2.00, x2.20, x2.50+, x2.76, x3.00, x3.24, x3.48, x3.75, x4.00]
              3840x2160@48.000	3840x2160 	48.00     	[x1.00, x1.25, x1.50, x1.75, x2.00, x2.20, x2.50+, x2.76, x3.00, x3.24, x3.48, x3.75, x4.00]
              3200x1800@59.956	3200x1800 	59.96     	[x1.00, x1.25, x1.50, x1.75, x2.00+, x2.25, x2.50, x2.74, x2.99, x3.23, x3.51, x3.77]
              2880x1620@59.960	2880x1620 	59.96     	[x1.00, x1.25, x1.50, x1.75+, x2.00, x2.25, x2.50, x2.77, x3.00, x3.27]
              2560x1600@59.987	2560x1600 	59.99     	[x1.00, x1.25, x1.50+, x1.75, x2.00, x2.25, x2.50, x2.76, x2.99, x3.23]
              2560x1440@59.961	2560x1440 	59.96     	[x1.00, x1.25, x1.50+, x1.76, x2.00, x2.25, x2.50, x2.76, x3.02]
              2048x1536@59.954	2048x1536 	59.95     	[x1.00, x1.25+, x1.50, x1.75, x2.00, x2.25, x2.50, x2.75]
              2048x1152@59.903	2048x1152 	59.90     	[x1.00, x1.24+, x1.51, x1.75, x2.00, x2.25]
              1920x1440@59.968	1920x1440 	59.97     	[x1.00, x1.25+, x1.50, x1.75, x2.00, x2.25, x2.50]
              1920x1200@59.885	1920x1200 	59.88     	[x1.00, x1.25+, x1.50, x1.75, x2.00, x2.24]
              1920x1080@59.963	1920x1080 	59.96     	[x1.00, x1.25+, x1.50, x1.74, x2.00, x2.31]
              1680x1050@59.954	1680x1050 	59.95     	[x1.00+, x1.25, x1.50, x1.75, x2.00]
              1600x1200@59.869	1600x1200 	59.87     	[x1.00+, x1.25, x1.50, x1.75, x2.00]
               1600x900@59.946	1600x900  	59.95     	[x1.00+, x1.25, x1.49, x1.75]
              1440x1080@59.989	1440x1080 	59.99     	[x1.00+, x1.25, x1.50, x1.75, x2.00]
               1440x900@59.887	1440x900  	59.89     	[x1.00+, x1.25, x1.50, x1.75]
              1400x1050@59.978	1400x1050 	59.98     	[x1.00+, x1.25, x1.50, x1.75]
               1368x768@59.882	1368x768  	59.88     	[x1.00+, x1.26, x1.50]
               1280x960@59.939	1280x960  	59.94     	[x1.00+, x1.25, x1.50, x1.75]
               1280x800@59.810	1280x800  	59.81     	[x1.00+, x1.25, x1.50]
               1280x720@59.855	1280x720  	59.86     	[x1.00+, x1.25, x1.51]
               1152x864@59.959	1152x864  	59.96     	[x1.00+, x1.25, x1.50]
               1024x768@59.920	1024x768  	59.92     	[x1.00+, x1.25]
                800x600@59.861	800x600   	59.86     	[x1.00+]
is-builtin: true
display-name: "Built-in display"

$ gnome-randr modify eDP-1 --mode=3840x2160@60.000 --scale=1.75
setting mode to 3840x2160@60.000
setting scale to 1.75
D-Bus error: Scale 1.75 not valid for resolution 3840x2160 (org.freedesktop.DBus.Error.InvalidArgs)
Scale 1.75 not valid for resolution 3840x2160

$ gnome-randr modify eDP-1 --mode=3840x2160@60.000 --scale=1.7518248
setting mode to 3840x2160@60.000
setting scale to 1.7518248
```

## Comments

---
**maxwellainatchi** at 2024-04-17T17:43:26Z
@andersk thanks for contributing!

Fascinating... and frustrating. I like your suggested solution. Maybe a tolerance of $.05$ would work? Seems fractional scaling is usually to the nearest $.25$.

As I've mentioned in other issues, I don't currently have access to a machine to test on, so I can't make any contributions myself. If you're able to make the change yourself, I would be happy to see a pull request. 

---
**TweakerZ** at 2025-02-06T17:26:31Z
@andersk how did you get your precise scaling value?

---
**andersk** at 2025-02-07T02:03:06Z
@TweakerZ You can see the exact value for the current mode in the output above:

```
x: 0, y: 0, scale: 1.7518248558044434, rotation: normal, primary: yes
```

For other modes, I needed to patch out the 2-decimal rounding from `impl Display for Mode`.

## How This Was Addressed

This bug is now fixed in the `modify --scale` path.

`gnome-randr` still keeps text query output readable, but it no longer requires users to guess the full exact internal Mutter scale float. Instead, it matches the user-provided value against the selected mode's advertised supported scales, prefers an exact match when present, and otherwise chooses the nearest supported scale within the same two-decimal precision users see in `query`.

Concrete file pointers:

- `src/cli/common.rs`
- `src/cli/modify/mod.rs`
- `src/cli/query.rs`

## How To Exercise And Test It

- list the supported scales for your connector:
  - `cargo run -- query CONNECTOR`
- if `query` shows a value like `x1.75`, try it directly:
  - `cargo run -- modify CONNECTOR --mode MODE --scale 1.75`
- confirm that values not shown by `query` still fail cleanly and point you back to the advertised list:
  - `cargo run -- modify CONNECTOR --mode MODE --scale 1.73`
