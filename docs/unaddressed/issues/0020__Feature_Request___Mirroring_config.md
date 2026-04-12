# Issue #20: [Feature Request]: Mirroring config

**Opened by** Merrit **at** 2022-07-13T20:03:21Z

## Body

Hi :wave: nice tool you've made, thanks for your work!

Would be great to be able to configure display mirroring, as GNOME itself currently only allows either extended or mirror all.

Example: I have three displays attached and want to mirror 2 as my "primary", leaving the 3rd as an extended.

## Comments

---
**maxwellainatchi** at 2022-07-20T12:21:19Z
Hi! Looks like this doesn't seem to be supported by the dbus API I'm using to manipulate displays. I can take a look into how xrandr does it and see if I can find the matching Wayland API, but it's quite possible this won't be feasible under my current implementation.

---
**Merrit** at 2022-07-20T14:58:49Z
I am not super familiar, but I believe that it mirrors by specifying the x + y coodinates, and for mirrored displays they just use the same resolution and coordinates so that they overlap each other in the virtual display.

---
**maxwellainatchi** at 2022-07-21T16:12:35Z
Hmm if that's the case, then this is definitely implementable. As a temporary workaround the you can try using the branch in #18, which allows for changing the XY position of displays. 

I don't currently have a working Linux computer to test on so I can't test or make this change until I get it back up and running, but you can try that out for now. 

---
**Merrit** at 2022-07-22T16:14:59Z
Neat, I'll give that a try when I have a chance! :+1: 

---
**Merrit** at 2022-07-25T14:09:11Z
Sounds like the DBus method won't allow it unfortunately:

```shell
❯ ./gnome-randr modify -d '0,0,2' HDMI-2
setting displacement to x: 0, y: 0, scale: 2
D-Bus error: Logical monitors overlap (org.freedesktop.DBus.Error.InvalidArgs)
Logical monitors overlap
```

---
**maxwellainatchi** at 2022-08-06T13:48:01Z
Ah, that's unfortunate. I'm not well-versed in Linux APIs, I just learned what d-bus was for this project, so I don't know if I'll be able to implement it otherwise

---
**Merrit** at 2022-08-06T16:39:33Z
Yeah, as it stands I don't think there is a way to interact with it except for dbus, and if it forbids this config there isn't much to be done until they change things on their end.

Given that I'll go ahead and close this, hopefully the config options & API will improve in the future! :sunflower: :)

---
**maxwellainatchi** at 2022-08-10T13:04:58Z
There definitely is a way to interact with it besides d-bus - I'm pretty sure I can interface with Wayland directly. I'm just unsure of how to actually do that

---
**maxwellainatchi** at 2022-08-10T13:11:54Z
I'm pretty sure I can use [this crate](https://docs.rs/wayland-client/latest/wayland_client/) somehow, but I'd have to dig in to it and I don't currently have time for that. If I do in the future I'll update this issue

---
**tobiasgrosser** at 2022-10-14T06:21:12Z
Just adding a note, that I would be interested in this feature as well.

It seems gnome checks here for the overlapping monitors:

https://gitlab.gnome.org/GNOME/mutter/-/blob/62f4e0501fc19d68a0ee68645e5c9764ef8a3808/src/backends/meta-monitor-config-manager.c#L1647

I wonder what happens if one just disables the check. Could that work?

---
**maxwellainatchi** at 2022-11-23T03:12:14Z
I'm not actually directly interacting with mutter, but rather doing it via dbus. In any case, I'm not sure how you would be able to disable that check though without actually recompiling mutter.

---
**ozgedurgut** at 2023-10-23T10:43:38Z
I'm attaching two screens to the Khadas and I want it to be the same on both screens. So I want to make Mirror. I'm using Khadas vim4 and I can't use xrandr. I thought gnome-randr might work, is there an update?
