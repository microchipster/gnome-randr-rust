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
