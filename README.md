# Mouse Gestures

Mouse gestures working under Wayland and X without the dependency on Xlib[^1], which will happily work on Compositors without XWayland.

This was initially an experiment in Go, which I ported to Rust for absolutely no reason.

It is meant to be run as a headless service. 

## Features

- Hotplug support
- Run key and mouse combination globally or application specific[^2]
- Overload the same gestures with modifiers
- Run commands
- Combinations and commands can run sequentially
- Mouse buttons can simulate key and mouse combinations

## How it works

This program will create two evdev devices.
A virtual mouse and a virtual keyboard.

Further it will grab all connected mice and keyboards and will forward every event to the appropriate virtual devices.

In this way every event can be intercepted.

## Configuration

There are command line flags to print out a trail, key combinations and to get an app id, which can be used to create a [config](./resources/config.json) file, which is located at `XDG_CONFIG_HOME`, which is usually `~/.config/mouse-gestures`.

## Alternatives

[https://github.com/thjaeger/easystroke](https://github.com/thjaeger/easystroke):

The original. Works only on X and hasn't received an update for over 9 years.

[https://github.com/jersou/mouse-actions](https://github.com/jersou/mouse-actions):

This was the main inspiration for this and I took the recognition algorithm from it.

[https://github.com/jersou/mouse-actions](https://github.com/ThatOtherAndrew/Hexecute):

Eyecandy. Needs to be launched manually


[^1]: There is a binding to Xlib, but it is only used under X, to get the active app id.
[^2]: Since wayland has a stricter security policy, than X, the compisitor has to provide some kind of IPC mechanism to get the active window. See [wm.conf](./resources/wm.conf)
