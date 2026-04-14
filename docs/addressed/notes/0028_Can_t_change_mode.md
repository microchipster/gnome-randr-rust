# Issue #28: Can't change mode

**Opened by** 0Raik **at** 2025-02-26T15:11:48Z

## Body

Using latest Manjaro Gnome 47 
First, thank you for the hard work to make xrandr possible for Wayland and Gnome!

I'm tring to change my VGA monitor to 640x480 and after running `gnome-randr`

```
\supports-mirroring: true
layout-mode: physical
supports-changing-layout-mode: false
global-scale-required: false

logical monitor 0:
x: 800, y: 0, scale: 1, rotation: normal, primary: yes
associated physical monitors:
	HDMI-1 GSM M2362D 0x0004b7ca

logical monitor 1:
x: 0, y: 480, scale: 1, rotation: normal, primary: no
associated physical monitors:
	DP-1 ITE DP2VGA V235 0x00000000

DP-1 ITE DP2VGA V235 0x00000000
              1920x1200@59.950	1920x1200 	59.95     	[x1.00+, x2.00]
              1920x1080@60.000	1920x1080 	60.00     	[x1.00+, x2.00]
              1920x1080@59.934	1920x1080 	59.93     	[x1.00+, x2.00]
              1680x1050@59.954	1680x1050 	59.95     	[x1.00+, x2.00]
              1680x1050@59.883	1680x1050 	59.88     	[x1.00+, x2.00]
              1600x1200@60.000	1600x1200 	60.00     	[x1.00+, x2.00]
              1600x1200@59.924	1600x1200 	59.92     	[x1.00+, x2.00]
              1600x1200@59.869	1600x1200 	59.87     	[x1.00+, x2.00]
               1600x900@60.000	1600x900  	60.00     	[x1.00+]
               1600x900@59.946	1600x900  	59.95     	[x1.00+]
               1600x900@59.825	1600x900  	59.82     	[x1.00+]
              1440x1080@59.989	1440x1080 	59.99     	[x1.00+, x2.00]
              1440x1080@59.912	1440x1080 	59.91     	[x1.00+, x2.00]
               1440x900@59.901	1440x900  	59.90     	[x1.00+]
               1440x900@59.887	1440x900  	59.89     	[x1.00+]
              1400x1050@59.978	1400x1050 	59.98     	[x1.00+]
              1400x1050@59.948	1400x1050 	59.95     	[x1.00+]
               1368x768@59.882	1368x768  	59.88     	[x1.00+]
               1368x768@59.853	1368x768  	59.85     	[x1.00+]
               1366x768@59.790	1366x768  	59.79     	[x1.00+]
              1280x1024@60.020	1280x1024 	60.02     	[x1.00+]
               1280x960@60.000	1280x960  	60.00     	[x1.00+]
               1280x960@59.939	1280x960  	59.94     	[x1.00+]
               1280x960@59.920	1280x960  	59.92     	[x1.00+]
               1280x800@59.910	1280x800  	59.91     	[x1.00+]
               1280x800@59.810	1280x800  	59.81     	[x1.00+]
               1280x720@60.000	1280x720  	60.00     	[x1.00+]
               1280x720@59.855	1280x720  	59.86     	[x1.00+]
               1280x720@59.745	1280x720  	59.74     	[x1.00+]
               1152x864@59.959	1152x864  	59.96     	[x1.00+]
               1152x864@59.801	1152x864  	59.80     	[x1.00+]
               1024x768@60.004	1024x768  	60.00+    	[x1.00+]
               1024x768@59.920	1024x768  	59.92     	[x1.00+]
               1024x768@59.870	1024x768  	59.87     	[x1.00+]
                800x600@60.317	800x600   	60.32     	[x1.00+]
                800x600@59.861	800x600   	59.86     	[x1.00+]
                800x600@59.837	800x600   	59.84*    	[x1.00+]
is-builtin: false
display-name: "Integrated Tech Express Inc 27\""
is-underscanning: false
```

I tried 

`gnome-randr modify DP-1 --mode 640x480@59.32 --persistent
`
`D-Bus error: Invalid mode '640x480@59.32' specified (org.freedesktop.DBus.Error.InvalidArgs)
Invalid mode '640x480@59.32' specified
`
Is there any way to make it a valid mode? `Xrandr` in Xorg can do it as well as Windows. My device might not EDID as a valid mode because of the VGA standard or because is cheaply made but 640x480 works everywhere except I can't make it work using Wayland.

Thanks for the support and help.

## Comments

---
**maxwellainatchi** at 2025-02-27T18:56:03Z
I don't really have a setup to test against anymore, unfortunately, but I don't think the dbus interface supports adding modes, only picking from the list

## How This Was Addressed

- documented this as a backend limit rather than leaving it in the active implementation backlog
- clarified in `README.md` that `gnome-randr` can only select modes that Mutter already exposes in `query` / `query --json`
- clarified that adding new modelines or arbitrary custom modes would require different backend support or upstream Mutter changes, not just another CLI flag in this repository

## How To Exercise And Test It

- inspect the modes Mutter currently exposes for a connector:
  - `cargo run -- query DP-1`
- try selecting only a mode that already appears in the query output:
  - `cargo run -- modify DP-1 --mode 1024x768@60.004 --dry-run`
- compare that with the backend-limit explanation in the README:
  - `rg -n "custom modelines|mode injection|Backend Limits" README.md`
