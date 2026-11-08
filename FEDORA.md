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
| 7 | Stagger is wired (30 ms staggered content cross-fades, reduced-motion aware); a11y spans reduced-motion, high-contrast (Theme colors + shell wiring), visible focus ring, away-collapse, screen-reader names, and keyboard-operable buttons. Easing is tokenized end to end (15 sites on `Theme.emphasized/decelerate/accelerate`; no hardcoded `Easing.*` left). Panel fallbacks cover all ten panels with daemon/missing-service naming, action buttons queued offline (persisted to `~/.local/state/isle/queue.json`, capped 100, FIFO flush on reconnect, "N queued items sent" announce), cached-payload staleness banners (`StaleBanner` component in 8 daemon-backed panels, verified by mocked-bridge render), and tray icons resolved through five levels (named path → attention variant → themed fallback → BMP pixmap → title-initial → generic glyph). Tray pixmaps share the wallhaven-thumbnail LRU budget (200 MiB combined) and expire after 7 days. Queue flush ordering is verified (writes flush before reload reads on one stream; reloads observe post-write state — E2E 2026-11-09 with a second seeded todo). Pixmap freshness is verified (in-use icons touched on read, surviving TTL via `an_in_use_pixmap_survives_ttl_by_being_touched`). Phase 7 items are closed; remaining work is polish only. |
| - | System RPM is built and verified (4.3M, `Requires: quickshell >= 0.3.2, mesa-libGL, (awww or hyprpaper), systemd-user`; staged via DNF-downloaded `rpm-build` without root). Runtime quickshell is the Nix store 0.3.2: Fedora's 0.2.1-git was downloaded, extracted, and run — it fails to load the shell at `BackgroundEffect.blurRegion` ("Non-existent attached object") and `Region.radius` ("Cannot assign to non-existent property"), both absent in 0.2.1 qmltypes but present in 0.3.2. The overlay layer itself exists in both, so the blocker is blur glass + rounded masks, not overlay. See `packaging/fedora.yaml` for the qmltypes evidence. Same fallback evidence (170 tests, clippy, lint, overlay) below. |

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
- Tray pixmap icons: `tray.rs` reads the `IconPixmap` property (32×32
  ARGB32, capped 256×256), renders 32-bpp BMPs to
  `~/.cache/isled/tray-pixmaps/` (100-file cap, newest kept), and the
  panel resolves named → attention → themed → BMP → title-initial →
  generic glyph. Verified by 4 new daemon tests (`bmp_bytes_has_a_bmp_header_and_right_size`,
  `a_pixmap_only_item_gets_a_bmp_icon_path` — writes a real BMP then
  removes it, leaving the cache clean).
- Offline queue: `IsledBridge.call()` queues while disconnected (cap
  100, oldest dropped with `lastError`), `flushQueue()` sends FIFO on
  reconnect preserving ids, `callAndSay` announces "Queued — will send
  when isled is back." and "N queued items sent." on flush. Buttons
  re-enabled (previous disable gates removed) since intents now queue.
- Queue persistence: `~/.local/state/isle/queue.json` via `FileView` +
  `mkdir -p` Process; saves gated on `queueRestored` (a startup `ping`
  cannot overwrite the file before restore reads it). Restored intents
  get fresh ids and flush on connect. E2E proof 2026-11-09: seeded
  `[{todos.add, queued-intent-e2e-…}]`, restarted the UI with isled
  down, reconnected — `todos.list` returned the seeded todo (id 1) and
  the queue file ended `[]`. Test todo removed afterwards.
- Shared LRU budget: `cache::prune_all()` + `total_bytes_both()` merge
  thumbnails + tray pixmaps oldest-first under `cache_max_bytes`.
  `wallpaper.cache` reports both dirs; `wallpaper.prune` + search-time
  prune use the shared budget. New test
  `prune_dirs_shares_one_budget_across_dirs` (175 tests total).
- Staleness banners: `StaleBanner` (registered in `components/qmldir`,
  `Accessible.role: Alert`) appears under the header in 8 daemon-backed
  panels whenever `bridge.connected` is false, with the queued count
  when intents wait. Layout-safe (height collapses; following elements
  anchor to its bottom). Verified offscreen with a mocked disconnected
  bridge: banner + cached todo render stacked with no overlap.
- Pixmap TTL: `prune_pixmap_cache` expires BMPs older than 7 days
  before counting toward the 100-file cap, so quit apps' icons vanish
  within a week even under budget. Test
  `expired_pixmaps_are_evicted_even_under_the_cap` (back-dates with
  `touch -d`, no new crates) — 176 tests total.
- `just package-fedora` → `/tmp/isle-fedora.tar.gz` (4.3 MB, no sudo
  needed to install — `install.sh` writes user-local units).
- System path staged: `DESTDIR=/tmp/opencode/isle-system-test bash
  packaging/install-system.sh` → `/usr/local/bin/isled` (135M),
  `ui/`, both unit templates, docs, `/etc/isle/sway-binds.conf` staged
  byte-identical; `--dry-run` prints every path before writing.
- System RPM built without root (`dnf download rpm-build`, extracted
  `rpmbuild`, `--define __rpmuncompress/__brp_*` overrides):
  `/tmp/opencode/x86_64/dynamic-island-0.1.0-1.fc45.x86_64.rpm` (4.3M,
  `rpm -qpi` clean, `Requires: quickshell >= 0.3.2, mesa-libGL,
  (awww or hyprpaper), systemd-user`, `rpm -qpl` lists `/usr/local/bin/isled`
  + full UI + unit templates).
- `grim` screenshot shows the idle pill (dot + clock) at the top centre.
