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

Thank  U :)



## Comments

---
**maxwellainatchi** at 2023-02-05T04:44:24Z
Hi @bzhpwr! Glad you like it. I made this tool mostly just to learn Rust, and as I no longer have access to a working Linux install, I'm not going to be contributing more to it for the time being. If I ever decide to fix my Linux install I'll work on it again. This definitely would be a great feature to add though!

Re: the `off` option, there's already an issue open for adding support (https://github.com/maxwellainatchi/gnome-randr-rust/issues/13).

---
**maxwellainatchi** at 2023-02-05T04:46:42Z
Oh also, someone has a PR open (https://github.com/maxwellainatchi/gnome-randr-rust/pull/18) to set the positioning, though not quite `--left-of` and `--right-of`. I didn't like how it was implemented so I didn't merge it, but you're welcome to use that branch if it's helpful.
