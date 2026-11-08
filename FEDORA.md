# Dynamic Island on Fedora (Sway + Nix binary)
#
# The project was authored on NixOS (niri + hyprland). On this Fedora 45
# machine only the Nix binary is available, and the compositor is Sway 1.12
# (swayfx). The daemon and the QML are unchanged; only packaging and the
# compositor keybinds differ.

## What works here (verified 2026-11-09)

- `cargo test` (161 tests), `cargo clippy`, `cargo fmt`, `just lint-qml`,
  `just preview` / `just preview-live` all pass on Fedora.
- `just install-units` writes `~/.config/systemd/user/isled.service` and
  `quickshell.service` with store paths baked in, and `just start` runs
  them under systemd user. Both services report `active (running)` and
  the IPC socket replies to `isled call ping` / `context.summary`.
- The island maps as layer **`overlay`** on Sway (checked with
  `swaymsg -t get_tree`): `overlay quickshell`. This is new
  (`ui/shell.qml` sets `WlrLayershell.layer: WlrLayer.Overlay` + 
  `aboveWindows: true`) — previously the surface sat at `top`, where
  another overlay could cover it and steal clicks from the island.
- Wallhaven remote source (Phase 4) works: `wallpaper.search` and
  `wallpaper.download` return live results, thumbnails are cached under
  `~/.cache/isled/wallhaven-thumbs/`, and downloads land in the wallpaper
  directory.
- **Phase 6 compositor adapter** works: `isle-core/src/compositor.rs`
  normalizes outputs/workspaces/focused window; `isled/src/compositor.rs`
  implements Niri (`$NIRI_SOCKET` `EventStream` JSON), Hyprland
  (`hyprctl -j` + `event/socket2` `action>>data` lines) and Sway
  (`swaymsg` + `get_tree`). `Compositor::Sway` now detects `$SWAYSOCK`.
  The server supervises streams with 2 s reconnect backoff, applies focus
  events as island activities, and exposes `compositor.info` /
  `context.summary` `compositor`.
- **Browser handoff** works: `isled/src/handoff.rs` polls `~/Downloads`
  in the 1 s pulse, waits out partial writes, rejects video, imports
  images into the catalog, and emits `wallpaper.changed` +
  a "Wallpaper imported" activity. `wallpaper.handoff-open` opens a page
  and `wallpaper.handoff-status` reports pending count.
- `awww` 0.12.1 is available via the Nix devshell (`nix develop`).

## Fedora-with-Nix quirks

### EGL
The Nix `quickshell` ships its own libEGL but no Mesa vendor stack, so a
bare launch fails:

```
WARN: qt.qpa.wayland: EGL not available
ERROR: Failed to create graphics context ... "Failed to initialize graphics backend for OpenGL."
```

`just install-units` appends three `Environment=` lines to the generated
`quickshell.service` that point glvnd at the Nix Mesa build:

```
Environment=LD_LIBRARY_PATH=<nix-mesa>/lib:<nix-libglvnd>/lib
Environment=__GLVND_LIBRARY_PATH=<nix-mesa>/lib/libEGL_mesa.so
Environment=__GLX_VENDOR_LIBRARY_NAME=mesa
```

Mesa 26.2.5 carries its own `50_mesa.json` vendor JSON and DRI drivers,
which is why this works where the system `/usr/lib64/dri` does not.
On a full NixOS machine these lines are unnecessary but harmless — NixOS
already supplies a matched Mesa through its own module.

### HUD / GNOME not required
Compositor detection (`Compositor::detect`) reads `$NIRI_SOCKET` and
`$HYPRLAND_INSTANCE_SIGNATURE`. Sway sets neither, so the daemon reports
`compositor=unknown` in the snapshot. Everything that does not depend on
compositor events (panels, IPC, wallpapers, notifications, tray) works.
Per-compositor signals (workspaces, outputs, screencast) are Phase 6
work, which is also absent on NixOS — no regression.

### Waybar coexist
Waybar sits at layer `top`; the island sits at `overlay`, so the island
always renders above Waybar. Both share the top edge: expect the pill to
overlap the bar visually near the centre. If you prefer, disable Waybar's
top bar or leave it — the island's input mask only covers the island
and snackbar, so mouse clicks elsewhere still reach Waybar.

## Sway setup

1. Include the binds:

   ```
   include /home/asuna/Projects/dynamic-island/tools/sway-binds.conf
   ```

   in `~/.config/sway/config` (or copy the lines).
2. `nix develop` then `just build && just start`.
3. Click the island to open a panel; `Mod+Ctrl+J` toggles it.

## What is still missing (vs PLAN.md)

| Phase | What is missing |
|-------|-----------------|
| 4 | Pending-link UI in the theme panel (daemon side done); LRU pruning of the downloaded cache |
| 6 | logind Idle/LockHint, UPower low-battery, PipeWire screencast meeting detection, network-quality and clipboard-burst signals, reducer + scorer |
| 7 | Idle/low-power behavior, animation tuning, a11y, fallbacks for missing services (no NM, no BT daemon, no MPRIS player, no wallpaper engine — some of these already degrade honestly in the UI) |
| - | `quickshell` on Fedora still comes from the Nix store via `install-units`; there is no standalone `.deb/.rpm` path (the HM module is NixOS-only by design) |

## Verified runs

- `swaymsg -t get_tree` → `layer_shell_surfaces` contains
  `('overlay', 'quickshell', None)`.
- `core/target/debug/isled call wallpaper.search` → 24 items, page 1/47,
  a 22 KB thumbnail on disk.
- `core/target/debug/isled call compositor.info` → real Sway data:
  `focused_window={app_id:kitty, title:'OC | Greeting', workspace:'OC | Greeting'}`,
  `outputs=[{name:eDP-1, scale:1.0, width:1920, height:1080, refresh_hz:144000}]`,
  `workspaces=[{name:1, focused:true, output:eDP-1}, {name:2, focused:false}]`.
  `context.summary` now includes the same `compositor` block.
- `core/target/debug/isled call wallpaper.handoff-status` → `{"dir":"/home/asuna/Downloads","pending":0}`.
- `grim` screenshot shows the idle pill (dot + clock) at the top centre.
