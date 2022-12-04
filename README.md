# Swayless

**Swayless** is an attempt to bring [Suckless's DWM](https://dwm.suckless.org/)
tag system to sway.

This helps you configure sway to work a bit more like
[Suckless's DWM](https://dwm.suckless.org/). This means workspaces are more like
tags. Each screen has 9 tags (workspaces are name-spaced on a per-screen basis).
Each window/container is associated with 1 tag. You can see multiple tags at
once in your screen.

It may also work with i3, but this is untested.

The initial code was based on [Swaysome](https://gitlab.com/hyask/swaysome) (by
[Skia](https://gitlab.com/hyask)), but it has nothing to do with it now.

## Usage

Install it using `cargo install --path .`.

Then configure your sway. Mine looks like this:

```bash
# Init workspaces for every screen
exec "swayless init"

## Workspace keybinds

# switch to workspace
bindsym $mod+1 exec "swayless focus 1"
bindsym $mod+2 exec "swayless focus 2"
bindsym $mod+3 exec "swayless focus 3"
bindsym $mod+4 exec "swayless focus 4"
bindsym $mod+5 exec "swayless focus 5"
bindsym $mod+6 exec "swayless focus 6"
bindsym $mod+7 exec "swayless focus 7"
bindsym $mod+8 exec "swayless focus 8"
bindsym $mod+9 exec "swayless focus 9"

# move to workspace
bindsym $mod+Shift+1 exec "swayless move 1"
bindsym $mod+Shift+2 exec "swayless move 2"
bindsym $mod+Shift+3 exec "swayless move 3"
bindsym $mod+Shift+4 exec "swayless move 4"
bindsym $mod+Shift+5 exec "swayless move 5"
bindsym $mod+Shift+6 exec "swayless move 6"
bindsym $mod+Shift+7 exec "swayless move 7"
bindsym $mod+Shift+8 exec "swayless move 8"
bindsym $mod+Shift+9 exec "swayless move 9"

# bring workspace here
bindsym $mod+Control+1 exec "swayless move-workspace-here 1"
bindsym $mod+Control+2 exec "swayless move-workspace-here 2"
bindsym $mod+Control+3 exec "swayless move-workspace-here 3"
bindsym $mod+Control+4 exec "swayless move-workspace-here 4"
bindsym $mod+Control+5 exec "swayless move-workspace-here 5"
bindsym $mod+Control+6 exec "swayless move-workspace-here 6"
bindsym $mod+Control+7 exec "swayless move-workspace-here 7"
bindsym $mod+Control+8 exec "swayless move-workspace-here 8"
bindsym $mod+Control+9 exec "swayless move-workspace-here 9"

# focus next/prev output
bindsym $mod+Comma focus output left
bindsym $mod+Period focus output right

# move container to next/prev output
bindsym $mod+Shift+Comma exec "swayless prev-output"
bindsym $mod+Shift+Period exec "swayless next-output"

# switch to the previous tab of the current output
bindsym $mod+Tab exec "swayless alt-tab"
```

## Commands

- `alt-tab` - Go to the previous tag on the current container
- `focus [name]` - Focus to another workspace on the same output
- `help` - Print this message or the help of the given subcommand(s)
- `init [name]` - Initialize the workspaces for all the outputs (it's the server
  that handles the commands)
- `move [name]` - Move the focused container to another workspace on the same
  output
- `move-workspace-here [name]` - Move all containers in workspace to current
  workspace
- `next-output [name]` - Move the focused container to the next output
- `prev-output [name]` - Move the focused container to the previous output

## Details

The `init` command initializes a server that listens to the requests that come
from the other commands. As such, you have to have the server running to run the
other commands. I did it like this because I needed to keep state.

Things like [Waybar](https://github.com/Alexays/Waybar) communicate with sway
through the socket. Besides resorting to changing the source code, it would be
difficult to exchange these calls for calls to `swayless`. With this, the `init`
server also listens for workspace focus change events in sway and intercepts
them. It's like these programs used `swayless`.

### Missing features compared to DWM

- Containers/Windows can only belong to 1 tag;
- Tag 0 (show all tags) isn't implemented.
