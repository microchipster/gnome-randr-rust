# Issue #13: Add an `--off` flag

**Opened by** maxwellainatchi **at** 2021-10-26T15:07:29Z

## Body



## Comments

---
**satmandu** at 2022-07-20T02:50:08Z
Is there a way to use this to turn off a display without specifying an `--off` flag?

---
**maxwellainatchi** at 2022-07-20T11:34:38Z
You can try to set the brightness to 0 using the experimental branch at https://github.com/maxwellainatchi/gnome-randr-rust/tree/adjust-brightness.

***IMPORTANT***: it has some (potentially major) downsides, read this issue comment (https://github.com/maxwellainatchi/gnome-randr-rust/issues/16#issuecomment-1077997077) for more information.

I can also work on implementing the `--off` flag.

---
**maxwellainatchi** at 2022-07-20T12:10:49Z
Looks like this should be pretty easy - unfortunately I don't have access to a computer running Linux right now, my desktop is suffering from fatal issues and my laptop is ARM based. 

I can try implementing it blindly and putting it up in a branch, if you want to test it!

---
**maxwellainatchi** at 2022-07-20T12:13:56Z
Actually, looks like it's even already supported - you should be able to just pass in `--mode -1` and it'll disable the display.

---
**satmandu** at 2022-07-20T12:36:20Z
> Actually, looks like it's even already supported - you should be able to just pass in `--mode -1` and it'll disable the display.

Do I need to build off the experimental branch for this to be supported?

```
.cargo/bin/gnome-randr
supports-mirroring: true
layout-mode: logical
supports-changing-layout-mode: true
global-scale-required: false
renderer: "native"
legacy-ui-scaling-factor: 1

logical monitor 0:
x: 0, y: 0, scale: 1.5, rotation: normal, primary: yes
associated physical monitors:
        HDMI-1 HLT T779-108x1920 0x88888800

HDMI-1 HLT T779-108x1920 0x88888800
  1920x1080@59.994136810302734  1920x1080       59.99*+         [x1.00+, x1.25, x1.50, x1.74, x2.00, x2.31]
display-name: "HLT"
is-builtin: false

.cargo/bin/gnome-randr --mode -1
error: Found argument '--mode' which wasn't expected, or isn't valid in this context

USAGE:
    gnome-randr [SUBCOMMAND]

For more information try --help
```

---
**satmandu** at 2022-07-20T12:40:35Z
```
.cargo/bin/gnome-randr modify HDMI-1 --mode -1
error: Found argument '-1' which wasn't expected, or isn't valid in this context

USAGE:
    gnome-randr modify <connector> --mode <mode>

For more information try --help
```

---
**maxwellainatchi** at 2022-07-20T12:56:02Z
You shouldn't need the experimental branch, the command should just be 

```
gnome-randr modify HDMI-1 --mode -1
```

As you did. I'm not sure why this isn't working, I'll have to dig in once I have a working computer to test on. My only guess right now is that you might need quotes around the `-1`, because it's probably trying to parse that as a flag. Try this:

```
gnome-randr modify HDMI-1 --mode "-1"
```

---
**satmandu** at 2022-07-20T13:03:05Z
```
.cargo/bin/gnome-randr modify HDMI-1 --mode "-1"
error: Found argument '-1' which wasn't expected, or isn't valid in this context

USAGE:
    gnome-randr modify <connector> --mode <mode>
```

I'll wait until you have a working computer...

But something about these steps did make the monitor freeze, but not turn off, so maybe something is happening.

---
**satmandu** at 2022-07-20T13:06:17Z
(This is on an arm64/Ubuntu raspberry pi 4 machine.)

---
**maxwellainatchi** at 2022-07-20T16:57:39Z
Ahhh, Ubuntu shouldn't actually need this utility. On Ubuntu xrandr is (fully?) bridged to Wayland, so you can just use xrandr to manipulate the displays instead.

---
**satmandu** at 2022-07-20T17:35:28Z
I don't think xrandr exposes the power functionality...

```
xrandr -q
Screen 0: minimum 16 x 16, current 2304 x 1440, maximum 32767 x 32767
XWAYLAND0 connected primary 2304x1440+0+0 (normal left inverted right x axis y axis) 330mm x 210mm
   2304x1440     59.91*+
   1920x1440     59.90  
   1600x1200     59.87  
   1440x1080     59.87  
   1400x1050     59.98  
   1280x1024     59.89  
   1280x960      59.94  
   1152x864      59.96  
   1024x768      59.92  
   800x600       59.86  
   640x480       59.38  
   320x240       59.52  
   1920x1200     59.88  
   1680x1050     59.95  
   1440x900      59.89  
   1280x800      59.81  
   720x480       59.71  
   640x400       59.95  
   320x200       58.96  
   2048x1152     59.90  
   1920x1080     59.96  
   1600x900      59.95  
   1368x768      59.88  
   1280x720      59.86  
   1024x576      59.90  
   864x486       59.92  
   720x400       59.55  
   640x350       59.77

 xrandr --output XWAYLAND0 --off
X Error of failed request:  BadMatch (invalid parameter attributes)
  Major opcode of failed request:  139 (RANDR)
  Minor opcode of failed request:  7 (RRSetScreenSize)
  Serial number of failed request:  20
  Current serial number in output stream:  22
```

---
**Conobi** at 2022-08-28T21:56:40Z
> As you did. I'm not sure why this isn't working, I'll have to dig in once I have a working computer to test on. My only guess right now is that you might need quotes around the `-1`, because it's probably trying to parse that as a flag. Try this:

To fix the wrong parsing from structopt when a minus-starting argument is given, the workaround is to do `--mode=-1` (see [this issue](https://github.com/TeXitoi/structopt/issues/129)).

But even this won't work: 
```
❯ gnome-randr modify HDMI-1 --mode=-1
setting mode to -1
D-Bus error: Invalid mode '-1' specified (org.freedesktop.DBus.Error.InvalidArgs)
Invalid mode '-1' specified
```

> ```
>  xrandr --output XWAYLAND0 --off
> X Error of failed request:  BadMatch (invalid parameter attributes)
>   Major opcode of failed request:  139 (RANDR)
>   Minor opcode of failed request:  7 (RRSetScreenSize)
>   Serial number of failed request:  20
>   Current serial number in output stream:  22
> ```

Same here, on Ubuntu 22.04. Can't use xrandr for setting a screen to off.

The off mode isn't well documented, and it's quite sad since it's one of the most useful options.


---
**saghm** at 2022-08-29T04:56:01Z
For my two monitor setup on my desktop (a 4K screen on the left and a 1440p on the left), I always use my right monitor connected to my work laptop during the day and then re-enable it for my desktop afterwards. Not sure if this will help anyone, but these are the invocations I've been using to do this:

```
# Turn off the right monitor
gnome-randr --output DP-1 --mode 2560x1440 --rate 165 --output DP-2 --off

# Turn the left right back on
gnome-randr --output DP-1 --mode 2560x1440 --rate 165 --output DP-2 --mode 3840x2160 --rate 60 --scale 1.5 --primary --right-of DP-1
```

---
**Eldiabolo21** at 2022-11-18T22:31:16Z
This feature would still be nice. 
`xrandr` under Ubuntu 22.04 with Wayland, does not work:
```
➜  ~ xrandr --output XWAYLAND0 --mode 1920x1440 --off
X Error of failed request:  BadMatch (invalid parameter attributes)
  Major opcode of failed request:  139 (RANDR)
  Minor opcode of failed request:  7 (RRSetScreenSize)
  Serial number of failed request:  20
  Current serial number in output stream:  21
```

I can reproduce the extact same results with `gnome-randar` and `--mode -1` (i.e. doesn't work) 

---
**Karol3500** at 2023-01-23T21:27:07Z
I get the same result with Arch:

```bash
$ gnome-randr modify DP-7 --mode=-1
setting mode to -1
D-Bus error: Invalid mode '-1' specified (org.freedesktop.DBus.Error.InvalidArgs)
Invalid mode '-1' specified
```

