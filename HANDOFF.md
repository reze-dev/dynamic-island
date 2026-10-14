# HANDOFF.md

Context file for a fresh session. Read this **and** `PLAN.md` before doing any
work.

Last updated: 2026-10-14
Plan status: revised — awww facts corrected, environment corrected, scope changed
Code written: flake + devshell only. No Rust yet.

---

## 1. What this project is

A context-aware, dynamically morphing island shell for Wayland compositors
**niri** and **hyprland**, in the spirit of Apple's Dynamic Island but for
Linux. A single surface at the top of the screen shrinks to a pill showing
whichever live activity matters most right now, and morphs open into one of
**nine** panels.

**It is also the successor to Ryoku shell.** That is the single most important
thing to understand about the scope. Ryoku was an evaluation, not the
destination — too many moving parts. `isled` is designed as the session's core
daemon from day one, not as a wallpaper/notification helper that later grows.
See `PLAN.md` §3.5.

The visual reference is `ctos-dynamic-island-context-aware-v2.html` — a React
mock in this directory. It is a *visual* reference only. Its 7 panels are the
starting spec (`context | media | clipboard | calendar | notes | notifications
| network`); the theme switcher was added later, and **todos** is the ninth —
it had a full editor in Phase 5 but no router entry until this revision.

## 2. Where everything is

```
/home/reze/Projects/dynamic-island/
├── ctos-dynamic-island-context-aware-v2.html   the visual mock
├── flake.nix                                  devshell — tracks nixos-unstable
├── PLAN.md                                    technical plan — read this
└── HANDOFF.md                                 this file

/home/reze/Michi/                               the user's NixOS system flake
```

**The authoring machine is the target machine.** NixOS 26.11, niri 26.04 and
hyprland all installed, `systemctl --user` running, Quickshell live. An earlier
version of this handoff said the box was Fedora/GNOME and the target was NixOS;
that was wrong and is fixed.

## 3. Decision log

| # | Decision | Why |
|---|---|---|
| 1 | **Rust core daemon + Quickshell UI**, not pure Quickshell | Morphing is trivial in QML (`States` + `Behavior`) and weeks of work in Rust. Context/event aggregation is the reverse. Project is ~25% UI / 75% plumbing. |
| 2 | Naming: `isle` (UI), `isled` (daemon) | Chosen by user from four options. Explicitly rejected "ctOS". |
| 3 | Wallpapers: **API + manual + browser-handoff** | User's choice. No scrapers. |
| 4 | Wallpaper **rendering** delegated to awww | awww draws pixels, our shell owns the catalog. Behind a trait so `mpvpaper`/`hyprpaper` stay swappable. |
| 5 | awww under `--namespace isle` | Isolates our daemon from any other awww, and namespaces the socket. |
| 6 | Keybinds route through `isled call`, never the UI | One definition serves both compositors; works from a terminal; compositor never learns the UI protocol. |
| 7 | Home Manager custom `isle.target` | `programs.quickshell.systemd.target` takes a *single* target; there are two compositors. Originally a workaround for Ryoku; now just correct. |
| 8 | 8 phases, each independently shippable | Scope is large. Stopping after any phase still yields a usable shell. |
| 9 | **Replace Ryoku; `isled` is the shell's core daemon from day one** | *(new, 2026-10-14)* Designing it single-purpose means a second IPC revision and a state migration later. Designing it wide costs empty stubs now. |
| 10 | **Track `nixos-unstable`; do not pin** | *(new, 2026-10-14)* Goal is to depend only on what unstable carries. The Ryoku pin existed only to keep Ryoku's glibc and mesa matched. Pinning is Ryoku's problem, not ours. |
| 11 | **9 panels, not 8** | *(new, 2026-10-14)* Todos has a full editor (add/complete/reorder) which is not a pill affordance, so it is a real view. The plan previously said 8 in §5.3 and 9 in the phases. |

### Rejected approaches — do not revisit without new information

- **Pure Quickshell**: 3-4 weeks, no Rust. Hybrid is only ~1 week more.
- **Pure Rust** (`winit` + layershell + `egui`/`iced`): 8-12 weeks. Killer is
  text input + IME and building a widget system from scratch.
- **AGS/GTK4, eww**: worse for free-form morphing.
- **HTML scraping for MoeWalls**: robots.txt disallows `/download*`; fragile.
- **Treating `swww` as a second wallpaper engine**: it is not a second engine.
  See §4.1 — it is the same project, renamed.
- **Taking on video wallpapers now**: awww cannot display video at all. Doing it
  means adopting `mpvpaper` as a genuinely separate daemon. Open decision #9.

## 4. Verified research findings

### 4.1 awww — renamed from swww, and it cannot do video

**This section replaces an earlier one that was wrong in two ways.**

`swww` was renamed **`awww`** on 2025-10-29 and shipped under the new name in
**0.12.0**. Same project, same author (LGFae). The GitHub repo `LGFae/swww` is
**archived**; the live tree is **Codeberg** `LGFae/awww`. Current release
**0.12.1**, present in `nixos-unstable`.

- **There is no `swww` package and no swww fallback engine.** `nix search nixpkgs
  awww` → 0.12.1. Anything saying otherwise is a bug.
- **There is no video support, and there never was.** No mpv backend, no
  `awww vid`, no mp4/webm. Grepping the source for `mpv|video|mp4|webm` hits
  only README prose pointing at **mpvpaper** as a separate tool. The earlier
  handoff here marked this "UNVERIFIED, believed `awww vid`" — that belief was
  wrong. Animation is image-sequence formats only: gif, apng, animated webp,
  avif (behind a build feature nixpkgs does not enable), plus static formats.
- **Subcommands:** `clear`, `restore`, `clear-cache`, `img`, `toggle`, `pause`,
  `unpause`, `kill`, `query`. Two binaries, both required: `awww` and
  `awww-daemon`. We are the client.
- **Socket:** `$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY}-awww-daemon[.<ns>].sock`.
  Dot separator, `.sock` suffix. Verified by running it:
  `wayland-1-awww-daemon.isle.sock`. **The daemon's `--help` says `.socket` and
  is wrong** — trust the observed path. Don't confuse it with the Wayland
  layer-shell namespace, which *is* concatenated (`awww-daemon` + `isle`).
- **`--format bgr`, not `rgb`.** Both 3-channel, ~3/4 the memory of `argb`, but
  `bgr` skips the byte swap `rgb` needs. `xrgb` is deprecated and warns.
- **`--transition-duration` is a silent no-op on the `simple` transition, which
  is the default.** Real knobs are `--transition-step` and `--transition-fps`.
- **`restore` is client-side**, replaying from the client's own cache of
  last-file-per-output. Do not rely on it — our catalog prunes by LRU, so
  `isled` must persist the current wallpaper and re-issue `img` itself.
- **Nothing is restored on monitor reconnect** — deliberately removed upstream.
  Re-issue `img` when `Compositor::outputs()` changes.
- Cache is `$XDG_CACHE_HOME/awww/$PKG_VERSION` (version-scoped since 0.12.0).
- `awww query --json` exists — use it, do not scrape the human output.
- `pause`/`unpause` are new in 0.12.1; useful for Phase 7 idle work.
- Will not run on GNOME (no `zwlr_layer_shell_v1`). Both targets do.
- Upstream ships a flake (`git+https://codeberg.org/LGFae/awww`) — the escape
  hatch if 0.12.1 has a bug. Do not track `main`; it runs ~3 months ahead of
  any release.

### 4.2 MoeWalls — compliant acquisition is not sufficient acquisition

robots.txt disallows `/download*`, listing pages are allowed. So: handoff only,
never a downloader. That much still holds.

**But MoeWalls is live-wallpaper oriented (MP4/WebM), and awww cannot display
video.** Handoff is the *compliant* mode, not a *sufficient* one. Until mpvpaper
is on the table, `isled` should reject video at import time so the catalog never
holds entries nothing can render. Open decision #9 decides whether handoff ships
at all.

### 4.3 Wallhaven API — works, no key for SFW

```sh
curl 'https://wallhaven.cc/api/v1/search?categories=100&purity=100&sorting=toplist&atleast=1920x1080&page=1'
```

Returns `id`, `url`, `short_url`, `path`, `thumbs.{small,large,original}`,
`resolution`, `file_size`, `file_type`, **`colors`** (5 hex values), `category`,
`created_at`, `views`, `favorites`, `source` (artist page).

- No API key for SFW. A key is only needed for NSFW, which is not implemented.
  Always `purity=100`.
- Rate limit **45 req/min**. Debounce ≥350 ms, never poll, cache.
- **`colors[]` is a ready-made palette *before downloading anything*** — the
  theme switcher can preview a wallpaper's theme with zero image decode.

**Research trap:** `https://wallhaven.cc/api/documentation` returns **404**. The
API is fine. Do not conclude it is dead. `robots.txt` is empty.

### 4.4 The machine, as actually measured (2026-10-14)

| Thing | Value |
|---|---|
| OS | NixOS 26.11, kernel 7.2.0, `linuxPackages_latest` |
| Compositor | niri 26.04 (default), hyprland also installed |
| Display | single `eDP-1`, 1920x1080@144Hz, **scale 1.25** |
| GPU | **amdgpu iGPU drives the panel** (`/dev/dri/card1`), NVIDIA GTX 1650 Ti dGPU present |
| Quickshell | **0.3.0 installed**; devshell has **0.3.2** |
| Wallpaper engine | **none installed** — awww only exists in the devshell |
| `wl-clipboard` | **not installed** system-wide (devshell has it) |
| systemd user | running |
| rustc | 1.99.0 via rustup; 1.98.1 in the devshell |

Consequences worth remembering:

- **Fractional scale 1.25 on amdgpu** is exactly where awww's bug history is
  worst — 0.9.4 and 0.10.2 were both scaling fixes. Test early.
- **One scaled laptop panel means no local test surface for multi-output
  wallpaper placement.** Write that path defensively.
- `user.kdl` at `~/.config/niri/` is the last include in Ryoku's `config.kdl`
  and wins over `settings.kdl`/`rebinds.kdl`. `rebinds.kdl` is **generated** and
  overwritten by `ryoku-wm-niri` — never hand-edit it. All of this becomes
  moot once Ryoku is gone (§3.5 of PLAN).
- `Super+J` → `ryotunes` and `Super+1..9` → workspace focus were both already
  bound, which is why the plan's `1-8, Super+J` never worked. Open decision #8.

### 4.5 What Ryoku owns that we inherit

**Portals are free.** `xdg-desktop-portal`, `-gnome`, `-gtk` are stock nixpkgs
systemd user units (`linked-runtime`), not Ryoku's. ScreenCast, Notification,
Secret, OpenURI all survive Ryoku's removal. Only `-hyprland` needs enabling for
the Hyprland session. This is the biggest "are we sure?" and the answer is yes.

Actually inherited:

| Ryoku unit | Holds | Becomes |
|---|---|---|
| `ryoku-shell.service` | `org.freedesktop.Notifications` + `org.kde.StatusNotifierWatcher` | `isle` + `isled` |
| `ryogami.service` | wallpaper daemon | `isled` + awww |
| `hypridle.service` | idle + session lock | logind `IdleHint`; lock is later scope |

`ryoku-rashin`, `ryoku-eq`, `ryoku-ai-usage`, `ryoku-materialize`,
`ryoku-palette-bridge`, `ryoku-bluetooth-reset` just go away.

The notification daemon and the tray are **bus-name** obligations. Claim both
before removing Ryoku, or the gap is visible to every other program on the bus.

### 4.6 Compositors

**Niri** — `$NIRI_SOCKET`, newline-delimited JSON. `Request::EventStream` (since
0.1.9) streams full state then deltas; no polling. `niri-ipc` crate pins to the
exact niri version. Detect via `$NIRI_SOCKET`.

**Hyprland** — `hyprctl -j` for queries, `event/socket2` for push. No good Rust
crate. Detect via `$HYPRLAND_INSTANCE_SIGNATURE`.

Schemas unrelated → the `Compositor` trait in PLAN §6.2.

### 4.7 Research traps — things that will waste your time

1. `wallhaven.cc/api/documentation` → 404. The API is fine.
2. `gitlab.com/LGFae/awww` and `awww.gitlab.io` → 403 Cloudflare. Use Codeberg.
3. `search.nixos.org/backend/...` → **401, auth required**. Use
   `nix search nixpkgs <name>`.
4. `pkgs/by-name/<xx>/<name>/package.nix` 404 ≠ package missing. `by-name` only
   covers migrated packages.
5. DuckDuckGo HTML endpoint returns nothing parseable here — use the
   `websearch` tool, or fetch GitHub/Codeberg/nixpkgs raw URLs directly.
6. `nix fmt` is broken in this repo's flake: it pipes the file to alejandra 4 on
   stdin, which dies with "unexpected end of file". Use
   `nix run nixpkgs#alejandra -- ./flake.nix` instead.

## 5. Prior art — read before writing code

**DankMaterialShell** (`AvengeMedia/DankMaterialShell`) is a Quickshell shell for
niri + hyprland that already ships an island, notification center, control
center, notepad, khal calendar, wallpaper switching with matugen, and lock
screen — **and solved its architecture the same way this project did**: a
compiled core daemon over IPC (theirs is Go, ours is Rust). Free reference for
~60% of the feature list.

Also worth skimming: `end-4/dots-hyprland`, `caelestia-dots`, `noctalia`.

**Ryoku itself is also prior art**, and it is running on this machine right now.
Read `~/.config/niri/` and the Ryoku NixOS modules before reinventing session
bootstrap, since we are replacing it.

## 6. Known design risks, ranked

1. **Layer-shell keyboard focus differs between niri and hyprland.** Island takes
   focus, you collapse it, focus stays trapped. Highest-probability way to lose a
   week. Phase 0, matrix in PLAN §5.2.
2. **Morph animation vs. surface reconfigure.** Two approaches (A: animate the
   real surface; B: fixed transparent surface + animated inner rect).
3. **Fractional scale 1.25 on a laptop panel** — awww's weakest area, and the
   only display config we can test on.
4. **No multi-output test surface.** Per-output wallpaper placement is written
   blind.
5. **Blur: niri has it (since 26.04), Quickshell does not expose it.**
   See §4.9. Not a compositor problem any more.
6. Quickshell QML API churn. Devshell (0.3.2) and session (0.3.0) currently
   disagree.
7. Ryoku cutover: a bus name claimed late is a session with no notifications.

## 7. What to do first, in this order

Phase 0 from PLAN §7. Do not start a later phase before the current phase's
exit criterion is met.

Done so far:

1. ~~Confirm the NixOS box has niri + hyprland~~ — both present.
2. ~~`nix search nixpkgs swww`~~ — moot, it is `awww` 0.12.1 and exists.
3. **awww probed and working** — see §4.2. Fractional scale 1.25 on amdgpu
   renders fine; `--format bgr`/`rgb` kill the daemon, so do not force a format.
4. **`isled` scaffolded** — IPC handshake, snapshot, unsolicited events,
   `isled call`. 22 tests, clippy clean, builds via `nix build .#isled`.
5. **Quickshell skeleton** — `ui/` loads clean, maps a `Top`/`OnDemand` layer
   surface, connects to `isled`, and is drivable from a terminal.
6. **`nix/module.nix` written and evaluating** against HM master.

Still open:

7. **Look at the island.** `cd` the repo, `nix develop`, then
   `cargo run --manifest-path core/Cargo.toml` in one shell and
   `quickshell -p ./ui` in another. Click the pill to morph. Judge the
   animation by eye — this is the part that cannot be automated.
8. **The focus matrix** (PLAN §5.2) on both compositors. Nothing is proven
   about focus-release-on-collapse yet, and it is the top-ranked risk.
9. **Verify environment forwarding** — see §4.7. Until `isled` reports
   `compositor=niri` from a systemd unit rather than `unknown`, the unit wiring
   is unproven.

**Phase 0 exit:** a pill that morphs smoothly and never steals focus, on both.

### 4.2 awww, as measured on this machine

- `awww-daemon --namespace isle --layer background` starts and binds
  `wayland-1-awww-daemon.isle.sock`.
- `awww query -n isle` → `eDP-1: 1536x864, scale 1.25` — **the fractional-scale
  path works**, which was the main worry (awww's 0.9.4 and 0.10.2 were scaling
  fixes).
- **Pixel formats, measured:**

  | `--format` | result |
  |---|---|
  | omitted (default) | works |
  | `argb` | works |
  | `abgr` | works |
  | `rgb` | **daemon dies** — `format Rgb888 not supported` |
  | `bgr` | **daemon dies** — `format Bgr888 not supported` |

  So the 3-channel memory saving is unavailable here. Do not pass `--format`.
- `awww img` on a real wallpaper succeeds.
- `awww img` on a `.mp4` fails, as expected — `~/Pictures/livewalls` is full of
  `moewalls-*.mp4` and none of it is usable.
- Backgrounding the daemon from a shell that then exits kills it. That is what
  `awww-daemon.service` is for; do not test it that way.

### 4.9 Blur: niri can, Quickshell cannot ask

The plan previously claimed niri has no blur. **That is wrong.** niri 26.04 —
the exact version installed — implements `ext-background-effect-v1` with blur,
xray, saturation and noise. Blur arrived in that release and was the most
upvoted niri feature to that point.

Two ways to get blur on niri:

1. Client requests it via `ext-background-effect` (needs client support).
2. **Compositor forces it** with a rule — works on *any* layer surface whether
   or not the client knows the protocol:
   ```kdl
   layer-rule {
       match namespace="quickshell"
       background-effect { blur true }
   }
   ```

**The blocker is Quickshell.** As of 0.3.2 it has a private
`Quickshell/Wayland/_BackgroundEffect` C++ directory but exports **no**
QML-facing blur type — `Quickshell.Wayland` only offers `WlSessionLock` and
`WlSessionLockSurface`.

Worse, `PanelWindow` has no `namespace` property (only `anchors`, `margins`,
`exclusiveZone`, `exclusionMode`, `aboveWindows`, `focusable`), so every
Quickshell layer surface reports `namespace="quickshell"`. Ryoku's surfaces and
ours are indistinguishable to a `layer-rule`. While Ryoku is running, the only
targetable rule is `match layer="top"`, which blurs everything on that layer.

So: what the user currently sees in niri is almost certainly Ryoku's own
translucency, not compositor blur. A translucent surface looks like blur and is
not — that is the distinction worth keeping straight.

Also note: the property is `aboveWindows`, not `above` (which does not exist),
and `width`/`height` on `PanelWindow` are deprecated in favour of
`implicitWidth`/`implicitHeight`.

### 4.7 Quickshell constraints, found the hard way

Quickshell 0.3's QML engine exposes only `QtQuick` and `Quickshell.*`.
`QtNetwork` is not linked, so `Socket`/`TextStream` are undefined.

- Use `Quickshell.Io.Socket`. Its `parser` is a `DataStreamParser`.
- `StdioCollector` is the usable one, but its `text` is **cumulative** — read it
  twice and you reprocess the buffer. Track a consumed offset and slice.
- `waitForEnd: false` is required or nothing arrives incrementally.
- `SplitParser` has no default property; you cannot declare a child parser.
- `console.log` goes to the **DEBUG** category; use `quickshell -vv` to see it.
  Default log level hides it, which looks exactly like dead code.
- A `QtObject` declared as a child of `PanelWindow` is **never instantiated**.
  Assign it to a property instead.
- `PanelWindow` has no `horizontalCenter` anchor and no `above` property, and
  rejects `width`/`height` in favour of `implicitWidth`/`implicitHeight`.

### 4.8 Home Manager's systemd options moved

`systemd.user.services.<name>` and `systemd.user.targets.<name>` are now
freeform submodules typed as systemd *sections*. The classic
`wantedBy`/`runtimeDirectory`/`serviceConfig` sugar no longer type-checks, and
`systemd.user.units` is gone. Write units as `Unit = { }` / `Service = { }` /
`Install = { }`. `nix/module.nix` does this, and deliberately bypasses
`programs.quickshell.systemd` for the same reason.

## 8. Open decisions — ask before the relevant phase

1. Adopt a pre-existing default-namespace awww daemon, or always
   `--namespace isle`? (before Phase 3)
2. Phase 2 read-only calendar, or defer the whole calendar module to Phase 5?
3. Calendar source: own ICS parsing, `khal`, or `vdirsyncer`?
4. Todos: plain SQLite, or Taskwarrior-compatible export?
5. Claim `org.freedesktop.Notifications` on day one, or shadow Ryoku's until
   cutover? (before Phase 2)
6. Wallpaper cache cap — 10 GB default, or tune to the actual disk?
7. Blur on niri: external implementation, or flat styling? (before Phase 1)
8. **Keybind scheme.** `Mod+1..9` is workspace switching in both compositors.
   `Mod+Alt+1..9` + `Mod+Alt+J` proposed. (before Phase 1)
9. **Video wallpapers at all?** awww cannot do it; mpvpaper is a separate
   daemon. Decides whether browser handoff ships. (before Phase 4)

## 9. Honest effort estimate

4-5.5 weeks for Phases 0-6, one person. Phases 0-3 (~2 weeks) already yield a
genuinely usable island.

**That estimate does not include removing Ryoku.** Porting its session
management, tray, notification ownership and idle handling is additional work on
top. Treat "island done" and "Ryoku gone" as separate milestones.

Every phase is independently shippable. The most likely reason this slips is
scope — nine panels plus the wallpaper system plus a shell migration — not
difficulty in any one piece.
### 4.10 Two Quickshell limits that shape the whole UI

Both of these exist in Quickshell's *internal*
`Quickshell/Wayland/_WlrLayerShell` module but **are not exposed on
`PanelWindow` in 0.3.0 or 0.3.2**. Verified by trying both — each fails with
"Cannot assign to non-existent property".

**1. There is no way to request compositor keyboard focus.**

`PanelWindowInterface` exposes only `anchors`, `margins`, `exclusiveZone`,
`exclusionMode`, `aboveWindows`, `focusable`. `WlrKeyboardFocus` (None /
Exclusive / OnDemand) is internal. `focusable: true` gives on-demand focus,
which niri grants **only on click**.

Measured consequence: a keybind-expanded panel cannot take focus.
`forceActiveFocus()` works *within* the QML window but does not make the window
itself focused, so typing goes to whatever was focused before — usually the
terminal. Clicking a text field works. `Esc` only reaches the island once
something inside it has been clicked.

So the Phase 0 goal "never steals focus" is trivially met, because the island
never has focus at all. Text entry (todos, notes, calendar) will be
click-to-focus until Quickshell exposes `keyboardFocus`, or we patch it.

**2. The layer namespace cannot be set.**

Every Quickshell surface is `namespace="quickshell"`; `PanelWindow` has no
`namespace` property. This kills the compositor-side blur plan while any other
Quickshell shell is running — a `layer-rule` on `namespace="quickshell"` would
hit Ryoku's surfaces too. The only targetable rule is `match layer="top"`,
which blurs everything on that layer.

Both limits disappear at Ryoku cutover, which is another reason the cutover
matters more than the island itself.
