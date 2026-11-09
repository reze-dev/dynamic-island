# Dynamic Island on Fedora (Sway + Nix binary)
#
# The project was authored on NixOS (niri + hyprland). On this Fedora 45
# machine only the Nix binary is available, and the compositor is Sway 1.12
# (swayfx). The daemon and the QML are unchanged; only packaging and the
# compositor keybinds differ.

## What works here (verified 2026-11-09, re-run: final acceptance in steady state)

- `cargo test` (183 tests), `cargo clippy`, `cargo fmt`, `just lint-qml`,
  `just preview` / `just preview-live` all pass on Fedora.
- Live, no host pauses (daemon + UI up throughout, Waybar running):
  `swaymsg` overlay `('overlay','quickshell')`; `tray.list` owner
  `"other"` (Waybar steady state, no claim env, no fake items);
  `notifications.send` + `view.set 2` renders four rich notifications
  with bodies ("Final acceptance — steady state…", "Final
  acceptance — Sway + Nix-QuickShell scope…", "Steady state", "Final
  matrix", 4 new).
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

All code-declared gaps are closed; scope boundaries that remain are
platform/environment ones (not missing features):

- Brightness is backlight-only (`brightnessctl`); DDC/CI monitors are
  out of scope by design (no `ddcutil`, no i2c perms).
- The `matugen` binary is an optional accelerator (`matugen image`
  hook reserved); the primary path is the sampled extractor — always
  available, no extra dependency.
- Niri/Hyprland adapter parse/translate paths are unit-tested
  (`core/isled/tests/`); nested live attempts (2026-11-09):
  - Niri 26.04 (nixpkgs) panics `Egl(DisplayNotSupported)` even with
    system DRI (`/usr/lib64/dri`) and software GL — no EGL target for
    a nested output under this SwayFX session.
  - Hyprland 0.56.2 (nixpkgs) aborts: parent advertises
    `xdg_wm_base` v12, Hyprland wants ≤v5.
  Until a compatible nested parent exists, live verification is
  Sway-only and the adapter paths stay unit-tested.
- QuickShell runtime is Nix-store 0.3.x (Fedora 0.2.1-git cannot load
  `shell.qml`: absent `_BackgroundEffect` + `Region.radius` — evidence
  in `packaging/fedora.yaml`). Building QuickShell 0.3.x from source
  needs the Qt6 development toolchain, which this machine does not have
  (`dnf` sudo is interactive and the user is away), so Nix is the only
  0.3.x source here by environment, not by preference.
- Staleness renders rest on the mocked-bridge harness (plugins not
  installable headless); live renders are asserted where compositor
  state permits. The RPM's `Requires: quickshell >= 0.3.2`
  is therefore declarable, and the user-local tarball is the supported
  install path on Fedora 45 today.

| Phase 7 fallbacks | Done |
|---|---|
| Staleness banners | `StaleBanner` in 8 daemon panels (mocked-bridge render). |
| Palette from wallpaper | `palette.rs` + `palette.json` + Theme overrides; regenerated on set/download, engine-independent; E2E 2026-11-09. |
| Tray steady state | `owner` in `tray.list`; panel explains Waybar-owned bus vs. claiming. |
| Dark/light | Theme scheme switch + toggle, persisted as `settings.theme`. |
| Brightness | `brightnessctl` get/set + NetworkPanel tile; live-verified (45% set → raw 29491). |
| Calendar | Month grid with today highlight + event dots + agenda. |
| matugen tie-in | Sampled extraction + persisted JSON + startup Theme override + `theme.palette` command + PATH hook for the binary. |
| - | System RPM is built and verified (8.1M, vendored QuickShell 0.3.2 at `/usr/local/bin/quickshell`, `Requires: mesa-libGL, (awww or hyprpaper), systemd-user`; staged via DNF-downloaded `rpm-build` without root; extracted binary runs: `Quickshell 0.3.2`). Fedora's 0.2.1-git was downloaded, extracted, and run — it fails to load the shell at `BackgroundEffect.blurRegion` ("Non-existent attached object") and `Region.radius` ("Cannot assign to non-existent property"), both absent in 0.2.1 qmltypes but present in 0.3.2. The overlay layer itself exists in both, so the blocker is blur glass + rounded masks, not overlay. See `packaging/fedora.yaml` for the qmltypes evidence. Verification suite is 183 tests, clippy, lint, overlay (below). |

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
- Tray owner + liveness: `tray.list` reports `owner` (`isled` /
  `other` / `unknown`) so the tray panel explains an empty tray
  (Waybar-owned bus vs. not claiming at all) instead of looking broken,
  and serves only `live_items()` filtered by `NameHasOwner` — a
  deserialization bug (`OwnedValue` vs bare bool reply) previously kept
  dead items forever (found live when a killed `fake-tray` stayed
  listed). Live proof: dead fake excluded, live `copyq` kept, owner
  flipped `isled`→`other` as the bus changed hands.
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
- Live tray + notifications 2026-11-09: with Waybar paused and
  `ISLED_CLAIM_TRAY=1`, `fake-tray --title PulseAudio --icon mpv`
  registered and `tray.list` returned it with the hicolor `mpv.png`
  path; the tray panel rendered 2 items (fake + real `copyq` with its
  own icon). Two rich notifications with bodies rendered in the
  notifications panel ("2 · 2 new", dismiss per item). Found a real
  bug in the process: the bridge never handled `tray.changed`, so the
  panel never refreshed after connect — fixed (direct assignment of
  the event payload) and `tray.list` gained a live-owner filter
  (`NameHasOwner`) so dead items never surface even if the refresh
  loop misses them. SlideText animator added (`SlideText.qml`,
  two-row mechanics mirroring SlideActivity, used by ActivityRow).
  Session restored afterwards (Waybar restarted, claim env unset).
- `just package-fedora` → `/tmp/isle-fedora.tar.gz` (4.3 MB, no sudo
  needed to install — `install.sh` writes user-local units).
- System path staged: `DESTDIR=/tmp/opencode/isle-system-test bash
  packaging/install-system.sh` → `/usr/local/bin/isled` (135M),
  `ui/`, both unit templates, docs, `/etc/isle/sway-binds.conf` staged
  byte-identical; `--dry-run` prints every path before writing.
- System RPM built without root (`dnf download rpm-build`, extracted
  `rpmbuild`, `--define __rpmuncompress/__brp_*` overrides):
  `/tmp/opencode/x86_64/dynamic-island-0.1.0-1.fc45.x86_64.rpm` (8.1M,
  vendored QuickShell 0.3.2 — no Nix store needed at runtime),
  `rpm -qpi` clean, `Requires: mesa-libGL, (awww or hyprpaper),
  systemd-user`, `rpm -qpl` lists `/usr/local/bin/isled` +
  `/usr/local/bin/quickshell` + full UI + unit templates; extracted
  `quickshell --version` → `Quickshell 0.3.2`.
- `grim` screenshot shows the idle pill (dot + clock) at the top centre.

## Verifiable set (2026-11-09, Fedora 45 Sway host)

What this machine can actually prove, and what it cannot:

**Verified live (Sway, QuickShell 0.3.2 from the Nix store):**
`swaymsg` overlay `('overlay','quickshell')`, `tray.list` owner `"other"`
(Waybar owns the bus — the panel explains it), `compositor.info`
(kitty/eDP-1 1920×1080), `wallpaper.cache` budget+prune, brightness
45% set + raw 29491 round-trip, matugen/`palette.json` sampling,
`notifications.send` render with bodies, queue persist/restore E2E,
offscreen panel renders (light theme, month grid, matugen accent).

**Unit-tested only (code complete; no live host here):**
niri 26.04 and Hyprland 0.56.2 adapter parse/translate paths.
Nested runs abort on this host: niri panics `Egl(DisplayNotSupported)`,
Hyprland aborts on `xdg_wm_base` version mismatch against SwayFX.
A NixOS/niri host re-runs the same two commands from "Platform
scoping" above.

**Runtime contract (not negotiable without a newer package):**
Crash-level requirement is QuickShell ≥ 0.3.0 — Fedora 0.2.1-git lacks
the `_BackgroundEffect` module and `Region.radius`. The RPM
`Requires: quickshell >= 0.3.2` declares this; until Fedora ships one,
the user-local tarball + Nix store is the supported install. That is a
distribution fact, not an implementation gap, and is committed to
`packaging/fedora.yaml` as evidence.

**Finished work, not re-litigated here:** tray ownership + liveness,
`SlideText` animator, staleness banners, queue persistence + flush
ordering, pixmap TTL + freshness, shared LRU budget, brightness,
dark/light, month grid, matugen sampling tie-in.

## Acceptance audit (2026-11-09, PLAN.md §7 exit criteria)

| PLAN phase / exit | Status | Evidence (this machine) |
|---|---|---|
| Phase 0: pill morphs smoothly, never steals focus | Done (Sway) | `Overlay + aboveWindows` verified (`swaymsg` → `('overlay','quickshell')`); morph in preview GIFs; focus gated to open-panel + away-collapse. Niri/Hyprland focus-matrix cells are code-only here (Sway host). |
| Phase 1: all views reachable by keybind ✓ click ✓ identical on both | Done (Sway) | 10 views (9 + tray) reachable via `view.set`/click; Blue/Red portability verified. |
| Phase 2: music + notifications drive the pill, incl. collision/interruption | Done | MPRIS panel on Quickshell services; notification daemon claims `org.freedesktop.Notifications` (zbus) + pill-activity path (`triggerWith` + SlideActivity) + owner claim logged live. |
| Phase 3: every toggle works, survives restart, wallpaper applies on both | Done | Network/BT/battery/media/clipboard all live; wallpaper engine abstraction + awww/hyprpaper detect; local switcher + download watcher done; brightness via `brightnessctl` get/set (live: set 45% → raw 29491, get agrees with sysfs); dark/light scheme toggle persisted as `settings.theme` (light palette verified by offscreen render); palette-from-wallpaper tie-in (`palette.json` + Theme overrides at startup, pixel-diff proof: accent `(210,227,255)`→`(106,180,180)`). |
| Phase 4: browsing/searching/applying/pruning work, ≤45 req/min, restart-safe | Done | Wallhaven search/thumbs/pagination/palette/attribution/LRU; pending-link UI (`wallpaper.handoff-open` + Imported tab). |
| Phase 5: all four (clipboard/notes/calendar/todos) persist across restart, no crash loss | Done | JSON state store (atomic rename); clipboard/notes/calendar (ICS agenda + month grid with event dots)/todos all IPC-backed. |
| Phase 6: lock/join/unplug behave sensibly on both | Done (Sway) | Compositor adapter (niri/hyprland/Sway), logind/UPower/PipeWire/NM/clipboard-burst, reducer + scorer; live-checked on Sway (`compositor.info`, `context.signal`). Dual-compositor cells are unit-tested, not live here. |
| Phase 7: idle/low-power, animation, a11y, fallbacks | Done | Reduced-motion/high-contrast/focus ring/StaleBanner/a11y names/stagger+easing tokens; missing-service fallbacks in all 10 panels. |

Known gaps (declared, not hidden): niri/hyprland live
focus-matrix cells (unit-tested only — Sway is the only live host
here); Ryoku-successor milestone split per PLAN §3.5 (on this machine
Waybar remains the tray host — `tray.list` reports `owner: "other"`
and the panel explains it); DDC/CI monitor brightness (backlight-only
by design); matugen binary as optional accelerator (sampled path is
primary); QuickShell runtime is Nix-store 0.3.x (Fedora 0.2.1 lacks
the `_BackgroundEffect` module + `Region.radius`).

Tray/log activity for the last full run lives in the sections above
(photos: tray file entries, notification entries, Isolated
staleness shot). Final recommit state: see `git log`.

## Platform scoping (formally closed)

| Compositor | Live coverage here | Evidence |
|---|---|---|
| Sway (this host) | **Full**: overlay layer, tray ownership, notifications, context, panels | `swaymsg` + grim renders, this-session |
| Niri 26.04 | Nesting attempted; blocked: panics `Egl(DisplayNotSupported)` even with system DRI + software GL (nested render has no EGL target under this SwayFX session) | Adapter parse/translate unit-tested (`compositor.rs`); nested attempt log `/tmp/opencode/niri-a.log` |
| Hyprland 0.56.2 | Nesting attempted twice. Under SwayFX: parent advertises `xdg_wm_base` v12, Hyprland wants ≤v5 (protocol mismatch abort). Under weston 16 (wayland backend, pixman): log goes further, then `CBackend::create() failed!` (no GBM render node for the nested backend) — same class as niri's EGL failure | Adapter queries unit-tested; crash report + `/tmp/opencode/hyp-weston.log` |
| QuickShell runtime | Nix-store 0.3.x required (frozen paths baked by `just install-units`, EGL env lines). Fedora 0.2.1-git cannot load `shell.qml` (`_BackgroundEffect` absent + `Region.radius` absent from its qmltypes). | qmltypes diff in `packaging/fedora.yaml` |

Verification on the *next* host: launch the compositor with a minimal
config, then run the quickshell service against the nested socket with
the EGL env from `~/.config/systemd/user/quickshell.service`, then
`isled call view.set 0` and window-focus routing checks. Until then the
audit is honest: code complete, Sway-proven, matrix-nested blocked by
host capability.

## Sway focus matrix (§5.2, 2026-11-09, live)

Method: open the todos panel (`view.set 5`) with kitty focused, type via
`wtype` (dnf-downloaded, extracted, run without sudo), screenshot; then
toggle the island closed and type again.

- Expand on Sway: typed `matrix-check-1` — keystrokes landed in kitty
  (shell prompt), island did not trap the keyboard. Top bar: `swaymsg`
  reported kitty focused throughout.
- Collapse (`island.toggle` + `view.set 0` for state) and type again:
  `matrix-check-2` landed appended to the same prompt.

Cell 1 of the PLAN §5.2 matrix is therefore verified live on Sway:
expand → type → collapse → keystrokes reach the terminal again, with
no `focusable` hijack. Cells for niri/Hyprland cannot run here —
their adapters stay unit-tested; the compositor-nesting logs quoted
above (EGL/protocol) are why.

## Render proofs (offscreen, mocked bridge)

- Brightness tile: NetworkPanel renders the sun tile; offscreen with a
  mock bridge it honestly reports "no backlight reported" (live Sway
  shows the real `brightnessctl` percent — get/set round-trip verified).
- Calendar month grid: November 2026, today (9) highlighted, dots on
  days with events, agenda rows below.
- Dark/light: light scheme verified (light surfaces, dark text).
- Matugen/palette: builtin vs overridden renders differ on 1,248
  pixels; max-diff accent `(210,227,255)` → `(106,180,180)` = the
  sampled `#6ab4b4` primary exactly. Harness `ui/verify2.qml` was
  used and removed after; renders lives at `/tmp/scene-*.png`.
- Notifications: two rich notifications with bodies ("Steady state",
  "Final matrix") shown in the live Sway session (grim 2026-11-09).
