# Issue #16: Feature request: set brightness

**Opened by** danilobjr **at** 2022-03-23T20:55:08Z

## Body

Hello, @maxwellainatchi. Thank you very much for this amazing tool! It works like a charm in my new Wayland env.

I'd like to know if is possible to implement brightness control and if you think that could be a good idea.

### Motivation

My PC is quite old with a very obsolete AMD GPU:

```shell
$ lspci
...
01:00.0 Display controller: Advanced Micro Devices, Inc. [AMD/ATI] Opal XT [Radeon R7 M265/M365X/M465]
```

And I can't change brightness through system settings and neither by set `/sys/class/backlight/acpi-video0/brightness`.

Because of this I've been using [Brightness Controller](https://github.com/LordAmit/Brightness) for a quite few years and it works very well in Xorg envs. I [saw that they use `xrandr` to set brightness](https://github.com/LordAmit/Brightness/blob/master/src/init.py#L267). So maybe your tool could help on this as well for Wayland envs.

I'm not saying that you should implement something to work in conjunction with Brightness Controller, it's just that your tool could control brightness as `xrandr` does.

### Suggestion

```shell
# based on gnome-randr usage
USAGE:
    gnome-randr modify [FLAGS] [OPTIONS] <connector>

# suggestion
$ gnome-randr modify --brightness 0.75 HDMI-1
```

As you already know, in `xrandr` we have:

```shell
xrandr --output HDMI-1 --brightness 0.75
```

### My setup

| Info | Value |
|-|-|
| Distro | Manjaro 21.2 |
| Display server | Wayland |
| Gnome | 41.3 |
| gnome-randr | 0.1.1 | 

### Query output

```shell
$ gnome-randr query
supports-mirroring: true
layout-mode: physical
supports-changing-layout-mode: false
global-scale-required: false
legacy-ui-scaling-factor: 1

logical monitor 0:
x: 0, y: 0, scale: 1, rotation: normal, primary: yes
associated physical monitors:
	HDMI-1 DEL Inspiron 5348 0x002206f2

HDMI-1 DEL Inspiron 5348 0x002206f2
                  1920x1080@60	1920x1080 	60.00*+   	[x1.00+, x2.00]
  1920x1080@59.940200805664062	1920x1080 	59.94     	[x1.00+, x2.00]
                  1920x1080@50	1920x1080 	50.00     	[x1.00+, x2.00]
  1680x1050@59.883251190185547	1680x1050 	59.88     	[x1.00+, x2.00]
   1440x900@59.901458740234375	1440x900  	59.90     	[x1.00+]
   1360x768@60.015163421630859	1360x768  	60.02     	[x1.00+]
  1280x1024@60.019741058349609	1280x1024 	60.02     	[x1.00+]
                   1280x720@60	1280x720  	60.00     	[x1.00+]
   1280x720@59.940200805664062	1280x720  	59.94     	[x1.00+]
                   1280x720@50	1280x720  	50.00     	[x1.00+]
   1024x768@60.003841400146484	1024x768  	60.00     	[x1.00+]
    800x600@60.316539764404297	800x600   	60.32     	[x1.00+]
                    720x576@50	720x576   	50.00     	[x1.00+]
is-builtin: false
display-name: "Dell Inc. 23\""
```

Thanks in advance.

## Comments

---
**maxwellainatchi** at 2022-03-23T22:58:15Z
Hi there! Thanks for opening an issue. I'm looking into this now.

Looks like `xrandr` sets the brightness by setting the monitor's gamma across all channels, is that what you're looking for?

---
**maxwellainatchi** at 2022-03-24T19:24:29Z
alright, @danilobjr there's an experimental branch up here https://github.com/maxwellainatchi/gnome-randr-rust/tree/adjust-brightness - it's not 100% working though, for reasons I have yet to figure out.

<h1>READ THIS BEFORE TRYING IT!</h1>
If your RGB gamma channels are all equal, you're fine to use this, it works correctly. However, if you have an adjustment to them (e.g. color correction, night light), it doesn't correctly recalculate the brightness and will mess with the color a lot. I'm working on figuring out why, since I copied the `xrandr` formula and translated the math into Rust almost exactly.

For now if you want to test it out, you can do `gnome-randr adjust --brightness [number]` where number is between 0 and 1. I put it under the `adjust` subcommand because it actually works differently from the other commands, and `--persistent` has no meaning here.  

---
**danilobjr** at 2022-03-24T22:08:21Z
Hey. Thanks for the answer.

> Looks like xrandr sets the brightness by setting the monitor's gamma across all channels, is that what you're looking for?

Yes. It sounds pretty good to me.

> alright, @danilobjr there's an experimental branch up here https://github.com/maxwellainatchi/gnome-randr-rust/tree/adjust-brightness

Awesome! I'll try it right now and give you some feedback soon

---
**danilobjr** at 2022-03-24T22:34:35Z
I've just tried it and it works very very well!

About redshift tool (built in on Manjaro), indeed I had to turn it of. Set brightness works, but after a moment the system corrects redshift automatically and then brightness resets. But for me, it's ok already.

I'm waiting to update to the next release. :rocket: 

Thank you so much! 

---
**NicoForce** at 2022-06-23T02:49:41Z
It's been a while since the last release, the brightness change works flawlessly, the only thing missing is a way to query the current brightness from the monitors.

Hope this project is not dead as it's been really helpful.

---
**maxwellainatchi** at 2022-07-08T12:27:16Z
@NicoForce I had to step away for a bit, but I'm back now, so not dead. 

However the brightness change doesn't work flawlessly, which is what's preventing me from merging it into the main branch. It works just fine if your color profile is basically white, but if it has any sort of color adjustment (e.g. due to night shift), it doesn't adjust it correctly. See https://github.com/maxwellainatchi/gnome-randr-rust/issues/16#issuecomment-1077997077 for more information. 

Since it has the potential to mess up your color profile, I don't want to merge it until that's fixed.

---
**zampitek** at 2023-09-05T20:46:26Z
Hello, it's been more than a year since this feature has been released, and still not in the main branch. I read your previous posts, and I totally understand your concerns about the color profile and I was thinking: isn't there a way to detect if the color profile is not adapt and, in case, prevent the command to be executed? It would be really nice to have this feature in the main branch, especially for the ones who, like me, have issues with increasing the screen brightness beyond the max one of the OS.

I hope to have a feedback about this suggestion soon :)

---
**maxwellainatchi** at 2023-09-22T20:39:05Z
@zampitek I believe the problem is less one of a technical nature and more of a mathematical nature - I think I'm being provided all the information necessary to adjust it appropriately, but the formula I've found only handles it correctly when there isn't a color profile applied. I'm not very strong in math, never did linear algebra/differential equations, so I don't really know how to correct for that. One day I might decide to look into it, but not at the current moment. 

You're welcome to try and correct my math though if you're more comfortable with it than me!

## How This Was Addressed

This feature is shipped under `modify`, not the old experimental `adjust` command.

The real fix was not to keep trying to infer a simplified gamma curve from the current LUT. Instead, the implementation now reads the current CRTC gamma ramp from Mutter and applies brightness directly to the existing LUT so Night Light, ICC profiles, and other non-neutral ramps are preserved rather than reconstructed incorrectly.

Concrete file pointers:

- `src/cli/brightness.rs`
- `src/cli/modify/mod.rs`
- `src/cli/query.rs`
- `src/display_config/proxied_methods.rs`

The landed behavior includes:

- `gnome-randr modify [CONNECTOR] --brightness <factor>`
- `--filter linear|gamma|filmic`
- query-time reporting of the current managed brightness and filter state
- baseline reuse logic so repeated absolute brightness calls do not compound unexpectedly when the current gamma still matches the last tool-managed state

## How To Exercise And Test It

- inspect current brightness state:
  - `cargo run -- query --summary`
- preview a brightness change without applying it:
  - `cargo run -- modify --dry-run --brightness 1.25 --filter filmic`
- apply a real brightness change on one connector:
  - `cargo run -- modify eDP-1 --brightness 1.25 --filter filmic`
- verify the reported state after applying it:
  - `cargo run -- query eDP-1 --summary`
- practical correctness check when Night Light or another color adjustment is active:
  - enable the external color adjustment first
  - run `gnome-randr modify CONNECTOR --brightness 0.75`
  - confirm the color cast stays intact while only brightness changes
