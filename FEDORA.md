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
- **Phase 6 context engine** works: `isle-core/src/context.rs`
  (`Signal` → `Context` reducer, `score`/`suggest`/`is_quiet`, meeting
  heuristics) and `isled/src/context_signals.rs` (logind idle via
  Z-Bus + `loginctl`, UPower battery via Z-Bus + `upower`,
  NetworkManager degraded via Z-Bus + `nmcli`, PipeWire node scan,
  compositor fullscreen + title heuristics, calendar <10 min, todos
  <5 min, clipboard burst). `Domain.context` is folded every 10 s by
  `watch_context_signals`, surfaces as island activities (low battery,
  degraded network, meeting detected), and is queryable via
  `context.summary/context` and injectable via `context.signal`.
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
| 7 | Stagger is wired (30 ms staggered content cross-fades, reduced-motion aware); a11y now spans reduced-motion, high-contrast (Theme colors + shell wiring), visible focus ring, away-collapse, screen-reader names (`Accessible.role/name` on `IconButton` + island), and keyboard-operable buttons (Tab + Enter/Space). Still open: stiffer per-panel fallbacks beyond the five verified panels, and animation easing tuning beyond the `emphasized` token. |
| - | `quickshell` on Fedora still comes from the Nix store via `install-units` or the `isle-fedora.tar.gz` tarball; there is no standalone `.deb/.rpm` path (the HM module is NixOS-only by design). `packaging/` now carries the reviewable system path: `dynamic-island.spec` (Name/Version/Requires), `install-system.sh` (sudo → `/usr/local`), and `fedora.yaml` (why RPM is not built in this devshell — no `rpmbuild` here). Same fallback evidence (170 tests, clippy, lint, overlay) below. |

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
- `core/target/debug/isled call context.signal '{"kind":"idle","idle":true}'` →
  `{"state":"away","last_title":"The machine is idle"}`;
  `context.signal '{"kind":"battery","percent":15,"charging":false}'` →
  `{"state":"low-battery","last_title":"Battery at 15%"}`.
  `context.summary` now also carries `"context": {…}`.
- `core/target/debug/isled call wallpaper.cache` →
  `{"dir":"/home/asuna/.cache/isled/wallhaven-thumbs","files":0,"bytes":0,"budget":209715200,"prune":true}`;
  `wallpaper.prune` → `{"removed":0}`. The cache honors
  `settings.cache_max_bytes` (default 200 MiB) and prunes LRU after every
  search. The old `/tmp` path is gone.
- `just package-fedora` → `/tmp/isle-fedora.tar.gz` (4.3 MB, no sudo
  needed to install — `install.sh` writes user-local units).
- System path staged: `DESTDIR=/tmp/opencode/isle-system-test bash
  packaging/install-system.sh` → `/usr/local/bin/isled` (135M),
  `ui/`, both unit templates, docs, `/etc/isle/sway-binds.conf` staged
  byte-identical; `--dry-run` prints every path before writing. `rpmbuild`
  is absent in this devshell (`which rpmbuild` empty), so the RPM spec
  is reviewable but unbuilt here — see `packaging/fedora.yaml`.
- `grim` screenshot shows the idle pill (dot + clock) at the top centre.
