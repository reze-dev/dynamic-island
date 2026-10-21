# HANDOFF.md

Context file for a fresh session. Read this **and** `PLAN.md` before doing any
work.

Last updated: 2026-10-21
Plan status: Phases 0–2 largely done. Phase 3 (control centre) and Phase 5
(data) are partly done. Phase 6 (context) is started — the activity engine
exists; the signal sources do not.
Code written: the daemon, the island, and all nine panels.

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
mock in this directory. It is a *visual* reference only. Its panels are the
starting spec (`context | media | clipboard | calendar | notes | notifications
| network`); the theme switcher was added later, and **todos** is the ninth.

## 2. Where everything is

```
/home/reze/Projects/dynamic-island/
├── ctos-dynamic-island-context-aware-v2.html   the visual mock
├── ctos-material-stealth-fixed.html            the second visual reference
├── flake.nix                                  devshell + isled package + lib
├── PLAN.md                                    technical plan — read this
├── HANDOFF.md                                 this file
│
├── core/                                      the Rust daemon
│   ├── isle-core/src/
│   │   ├── live.rs                            the activity engine
│   │   ├── protocol.rs                        the wire format
│   │   └── state.rs                           the shared data model
│   └── isled/src/
│       ├── main.rs                            the CLI
│       ├── server.rs                          the IPC server and command table
│       ├── domain.rs                          all the mutable state, one lock
│       ├── store.rs                           JSON persistence
│       ├── settings.rs                        what the user owns
│       ├── wallpaper.rs                       the catalog and the engines
│       ├── ics.rs                             the calendar reader
│       ├── tz.rs                              the system timezone
│       ├── network.rs                         NetworkManager over D-Bus
│       ├── notify.rs                          the notification daemon
│       ├── stats.rs                           CPU and memory
│       ├── paths.rs                           socket path, argument parsing
│       └── data/                              todos, timer, notes, clipboard
│
├── ui/                                        the Quickshell config
│   ├── shell.qml                              the root surface
│   ├── Island.qml                             the morph engine
│   ├── IsledBridge.qml                        the daemon connection
│   ├── PanelHost.qml                          the panel router
│   ├── Theme.qml Format.qml Icons.qml         the design tokens
│   ├── PanelCatalog.qml IslandSettings.qml
│   ├── preview.qml                            photographs the island
│   ├── demo.qml                               photographs the *animation*
│   ├── components/                            Icon, Card, Chip, StatTile,
│   │                                           IconButton, Toggle, ProgressRing,
│   │                                           ActivityRow, SlideActivity, MiniBar
│   └── panels/                                one file per panel, nine of them
│
├── nix/
│   ├── module.nix                             Home Manager module
│   ├── isled-package.nix                      the package expression
│   └── keybinds.nix                           the keybinds, for both compositors
│
├── systemd/                                   dev user units
├── tools/
│   └── demo.sh                                the scripted live demo
└── justfile                                   the dev loop
```

**The authoring machine is the target machine.** NixOS 26.11, niri 26.04 and
hyprland all installed, `systemctl --user` running, Quickshell live.

## 3. Decision log

| # | Decision | Why |
|---|---|---|
| 1 | **Rust core daemon + Quickshell UI**, not pure Quickshell | Morphing is trivial in QML (`States` + `Behavior`) and weeks of work in Rust. Context/event aggregation is the reverse. |
| 2 | Naming: `isle` (UI), `isled` (daemon) | Chosen by user. Explicitly rejected "ctOS". |
| 3 | Wallpapers: **API + manual + browser-handoff** | User's choice. No scrapers. |
| 4 | Wallpaper **rendering** delegated to awww | awww draws pixels, our shell owns the catalog. Behind a trait so `mpvpaper`/`hyprpaper` stay swappable. |
| 5 | awww under `--namespace isle` | Isolates our daemon from any other awww, and namespaces the socket. |
| 6 | Keybinds route through `isled call`, never the UI | One definition serves both compositors; works from a terminal; the compositor never learns the UI protocol. |
| 7 | Home Manager custom `isle.target` | `programs.quickshell.systemd.target` takes a *single* target; there are two compositors. |
| 8 | 8 phases, each independently shippable | Scope is large. Stopping after any phase still yields a usable shell. |
| 9 | **Replace Ryoku; `isled` is the shell's core daemon from day one** | Designing it narrow means a second IPC revision and a state migration later. |
| 10 | **Track `nixos-unstable`; do not pin** | The goal is to depend only on what unstable carries. |
| 11 | **9 panels, not 8** | Todos has a full editor, so it is a real view. |
| 12 | **The activity engine lives in the daemon, not the UI** | The UI decides how an activity *looks*; the daemon decides which activity *wins*. One place for the collision rules. |
| 13 | **Persistence is JSON files, not SQLite** | The volumes are tiny and plain files are readable and repairable. A database would buy nothing and cost a build dependency. |
| 14 | **The system timezone is read from `/etc/localtime` directly** | No datetime crate. A `TZif` reader is ~200 lines and the calendar needs it. |
| 15 | **Quickshell's own services own the machine's state** | Network, bluetooth, battery and media come from `Quickshell.Networking`, `Quickshell.Bluetooth`, `Quickshell.Services.UPower` and `Quickshell.Services.Mpris`. The daemon only owns the shell's *own* state. |
| 16 | **The surface never resizes** | Quickshell's `PanelWindow` has no `namespace` and no input region, so the surface is full width and transparent and only the island inside it changes shape. |

### Rejected approaches — do not revisit without new information

- **Pure Quickshell**: 3-4 weeks, no Rust. Hybrid is only ~1 week more.
- **Pure Rust** (`winit` + layershell + `egui`/`iced`): 8-12 weeks. Killer is
  text input + IME and building a widget system from scratch.
- **AGS/GTK4, eww**: worse for free-form morphing.
- **HTML scraping for MoeWalls**: robots.txt disallows `/download*`; fragile.
- **Treating `swww` as a second wallpaper engine**: it is not a second engine.
  See §4.1 — it is the same project, renamed.
- **Taking on video wallpapers now**: awww cannot display video at all.

## 4. Verified research findings

### 4.1 awww — renamed from swww, and it cannot do video

`swww` was renamed **`awww`** on 2025-10-29 and shipped under the new name in
**0.12.0**. Same project, same author (LGFae). The GitHub repo `LGFae/swww` is
**archived**; the live tree is **Codeberg** `LGFae/awww`. Current release
**0.12.1**, present in `nixos-unstable`.

- **There is no `swww` package and no swww fallback engine.** Anything saying
  otherwise is a bug.
- **There is no video support, and there never was.** No mpv backend, no
  `awww vid`, no mp4/webm. Animation is image-sequence formats only: gif, apng,
  animated webp, avif (behind a build feature nixpkgs does not enable).
- **Subcommands:** `clear`, `restore`, `clear-cache`, `img`, `toggle`, `pause`,
  `unpause`, `kill`, `query`. Two binaries, both required: `awww` and
  `awww-daemon`. We are the client.
- **Socket:** `$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY}-awww-daemon[.<ns>].sock`.
  Dot separator, `.sock` suffix. The daemon's `--help` says `.socket` and is
  wrong — trust the observed path.
- **Client flags** (verified against `awww img --help`): `-n/--namespace` is on
  the *subcommand*, so the call is `awww img -n isle <path>`. Transitions are
  `--transition-type` plus `--transition-step` and `--transition-fps`;
  `--transition-duration` is a silent no-op on `simple`, which is the default.
  `--transition-type fade --transition-step 60 --transition-fps 60` is what
  `isled` uses.
- **`restore` is client-side**, replaying from the client's own cache. Our
  catalog prunes by LRU, so `isled` persists the current wallpaper and
  re-issues `img` itself.
- **Nothing is restored on monitor reconnect** — deliberately removed upstream.
- Cache is `$XDG_CACHE_HOME/awww/$PKG_VERSION`.
- `awww query --json` exists — use it, do not scrape the human output.
- Will not run on GNOME. Both targets do.

### 4.2 MoeWalls — compliant acquisition is not sufficient acquisition

robots.txt disallows `/download*`, listing pages are allowed. So: handoff only,
never a downloader. But MoeWalls is live-wallpaper oriented (MP4/WebM), and
awww cannot display video. `isled` therefore **rejects anything that is not a
renderable image at import time**, so the catalog never holds entries nothing
can draw.

### 4.3 Wallhaven API — works, no key for SFW

```sh
curl 'https://wallhaven.cc/api/v1/search?categories=100&purity=100&sorting=toplist&atleast=1920x1080&page=1'
```

- No API key for SFW. Always `purity=100`.
- Rate limit **45 req/min**. Debounce ≥350 ms, never poll, cache.
- **`colors[]` is a ready-made palette *before downloading anything*** — the
  theme switcher can preview a wallpaper's theme with zero image decode.
- **Research trap:** `https://wallhaven.cc/api/documentation` returns **404**.
  The API is fine. Do not conclude it is dead.

### 4.4 The machine, as actually measured (2026-10-14)

| Thing | Value |
|---|---|
| OS | NixOS 26.11, kernel 7.2.0, `linuxPackages_latest` |
| Compositor | niri 26.04 (default), hyprland also installed |
| Display | single `eDP-1`, 1920x1080@144Hz, **scale 1.25** (the system flake says 1.2) |
| GPU | **amdgpu iGPU drives the panel** (`/dev/dri/card1`), NVIDIA GTX 1650 Ti dGPU present |
| Quickshell | **0.3.0 installed**; devshell has **0.3.2** |
| Wallpaper engine | **none installed system-wide** — awww only exists in the devshell |
| `wl-clipboard` | **not installed system-wide** (devshell has it) |
| systemd user | running |
| rustc | 1.99.0 via rustup; 1.98.1 in the devshell |

### 4.5 What Ryoku owns that we inherit

**Portals are free.** `xdg-desktop-portal`, `-gnome`, `-gtk` are stock nixpkgs
systemd user units. ScreenCast, Notification, Secret, OpenURI all survive
Ryoku's removal.

| Ryoku unit | Holds | Becomes |
|---|---|---|
| `ryoku-shell.service` | `org.freedesktop.Notifications` + `org.kde.StatusNotifierWatcher` | `isle` + `isled` |
| `ryogami.service` | wallpaper daemon | `isled` wallpaper subsystem + awww |
| `hypridle.service` | idle + session lock | logind `IdleHint`; lock is later scope |

**The notification daemon and the tray are bus-name obligations.** Claim both
before removing Ryoku, or the gap is visible to every other program on the bus.

### 4.6 Compositors

**Niri** — `$NIRI_SOCKET`, newline-delimited JSON. `Request::EventStream` (since
0.1.9) streams full state then deltas; no polling. Detect via `$NIRI_SOCKET`.

**Hyprland** — `hyprctl -j` for queries, `event/socket2` for push. Detect via
`$HYPRLAND_INSTANCE_SIGNATURE`.

### 4.7 Research traps — things that will waste your time

1. `wallhaven.cc/api/documentation` → 404. The API is fine.
2. `gitlab.com/LGFae/awww` and `awww.gitlab.io` → 403 Cloudflare. Use Codeberg.
3. `search.nixos.org/backend/...` → **401**. Use `nix search nixpkgs <name>`.
4. `pkgs/by-name/<xx>/<name>/package.nix` 404 ≠ package missing.
5. `nix fmt` is broken in this repo's flake: it pipes the file to alejandra 4 on
   stdin, which dies with "unexpected end of file". Use
   `nix run nixpkgs#alejandra -- ./flake.nix` instead.
6. **`lib.imap0` takes a list, not a count.** `imap0 = f: list: genList (n: f n
   (elemAt list n)) (length list)`. Use `lib.genList f count` for a count.
7. **A flake build of a local path sees the *git* tree, not the working
   directory.** `nix build .#isled` from a git repository copies only the
   committed files, so a new module is invisible to it until it is committed
   and the build fails with "file not found for module". `just release` uses
   `path:` on purpose so it builds what is on disk; `just release-committed`
   is the reproducible one.

### 4.8 Quickshell constraints, found the hard way

Quickshell 0.3's QML engine exposes `QtQuick` and `Quickshell.*`.

- **`QtQuick.Controls` is NOT linked.** No `ToolTip`, no `Button`, no
  `StackLayout`. Everything is built from `QtQuick` primitives. `QtQuick.Shapes`,
  `QtQuick.Layouts` and `QtQuick.Canvas` *are* available and are used.
- Use `Quickshell.Io.Socket`. Its `parser` is a `DataStreamParser`.
- `StdioCollector` is the usable one, but its `text` is **cumulative** — read it
  twice and you reprocess the buffer. Track a consumed offset and slice.
- `waitForEnd: false` is required or nothing arrives incrementally.
- `console.log` goes to the **DEBUG** category; use `quickshell -vv` to see it.
- A `QtObject` declared as a **child** of a `PanelWindow` is never
  instantiated. Assign it to a property instead.
- **`QtObject` has no default property**, so a bare `Timer { }` child of one
  fails with "Cannot assign to non-existent default property". Declare it as
  `property Timer foo: Timer { }` instead.
- **`PanelWindow` is not an `Item`**, so `Keys` cannot be attached to it. Put
  the key handling on the island, which is a `Rectangle`.
- `PanelWindow` has no `horizontalCenter` anchor and no `above` property.
  `width`/`height` are deprecated in favour of `implicitWidth`/`implicitHeight`.
- The property is `aboveWindows`, not `above`.
- **A file loaded as a root config does not get the parent directory's
  imports.** `import ".."` from a subdirectory resolves to nothing when
  quickshell loads that file directly, so the singletons and the types in
  `ui/qmldir` are not in scope. A file that is *imported* by another file
  (a panel, say) resolves them fine. The preview rig lives at `ui/preview.qml`
  for exactly this reason: it has to be in the same module as the island.

### 4.9 Blur: niri can, Quickshell cannot ask

niri 26.04 implements `ext-background-effect-v1` with blur. It can be forced
from the compositor side with a `layer-rule { background-effect { blur true } }`.

**The blocker is Quickshell.** As of 0.3.2 it has a private
`Quickshell/Wayland/_BackgroundEffect` C++ directory but exports **no**
QML-facing blur type, and `PanelWindow` has no `namespace` property, so every
Quickshell layer surface reports `namespace="quickshell"` and cannot be
targeted separately from Ryoku's.

Also note: a translucent surface looks like blur and is not — that is the
distinction worth keeping straight.

### 4.10 Two Quickshell limits that shape the whole UI

Both exist in Quickshell's *internal* `Quickshell/Wayland/_WlrLayerShell` but
are **not exposed on `PanelWindow` in 0.3.0 or 0.3.2**.

**1. There is no way to request compositor keyboard focus.**

`PanelWindowInterface` exposes only `anchors`, `margins`, `exclusiveZone`,
`exclusionMode`, `aboveWindows`, `focusable`. `WlrKeyboardFocus` is internal.
`focusable: true` gives on-demand focus, which niri grants **only on click**.

Measured consequence: a keybind-expanded panel cannot take focus.
`forceActiveFocus()` works *within* the QML window but does not make the window
itself focused, so typing goes to whatever was focused before. Clicking a text
field works. `Esc` only reaches the island once something inside it has been
clicked.

**2. The layer namespace cannot be set.**

Every Quickshell surface is `namespace="quickshell"`. This kills the
compositor-side blur plan while any other Quickshell shell is running.

Both limits disappear at Ryoku cutover.

## 5. What was built, and what it looks like

### 5.1 The island — four appearances, three sizes

| Appearance | Size | What it shows |
|---|---|---|
| **idle** | 132×36 | A coloured dot and the clock. Nothing else. |
| **peek** | 320×42 | On hover: the last thing that happened, plus a sliver of CPU and memory. |
| **triggered** | 380×56 | An event grew the island. It shows the glyph, the headline and the supporting line, then folds back on its own. |
| **full** | 520 × per-panel | A panel. Reached by a keybind or by clicking the island. |

The dot's colour is the *tone* of whatever the island last reacted to:
neutral, info (lavender), good (green), warn (amber) or critical (red). A
sticky activity (music playing, a timer running) sets the tone and keeps
setting it; a transient one (a notification) sets it and then lets the
ambient condition take it back.

The morph is `Behavior on width/height/radius` with `Easing.OutQuart` over
380 ms. The activity content **slides** rather than cross-fading, because a
cross-fade reads as "a new window opened" and a slide reads as "the same thing
saying something else" — `ui/components/SlideActivity.qml`.

### 5.2 The panels

| # | Panel | Where its data comes from |
|---|---|---|
| 1 | **context** | The daemon's `context.summary`: vitals, what is happening now, the timer, the next todo. |
| 2 | **media** | `Quickshell.Services.Mpris`. No Rust involved. |
| 3 | **notifications** | The daemon, which owns `org.freedesktop.Notifications`. |
| 4 | **clipboard** | The daemon's history. `wl-copy` to paste back, `wl-paste` to capture. |
| 5 | **calendar** | The daemon reads `.ics` files and lays the next fourteen hours out by the hour. |
| 6 | **todos** | The daemon's list, plus the pomodoro. A deadline can be written inline: `standup 15m`. |
| 7 | **notes** | Markdown files in a directory. Point it at an Obsidian vault and the panel offers to open the note there. |
| 8 | **network** | `Quickshell.Networking` and `Quickshell.Bluetooth`, plus `Quickshell.Services.UPower` for the battery. |
| 9 | **theme** | The daemon's wallpaper catalog: a grid of thumbnails, a directory to change, and the rotation switch. |

### 5.3 The activity engine

`core/isle-core/src/live.rs`. Everything the island can react to becomes an
`Activity` with a kind, a tone and a dwell time.

- The **kind** decides the priority, and the priority is the collision rule
  from `PLAN.md` §6.4: media > notification > timer > meeting > low battery >
  focus > todo > clipboard > network > system.
- The **tone** decides the dot's colour.
- A **sticky** activity describes a condition rather than an event. It stays
  in the ambient pool, keeps colouring the dot, and is what the peek shows.
- A **transient** activity grows the island, is shown for its dwell time, and
  is then forgotten.

`ActivityQueue` keeps two views: the ambient pool (ranked) and a bounded list
of recent activities. `expire()` drops the transients that have outlived their
dwell and leaves the sticky ones alone.

The daemon is the single funnel. Notifications arriving on the bus, a todo
coming due, the pomodoro changing phase, and the UI pushing a media player
all go through the same `trigger_activity` path, and the island only ever
learns about them from two events:

- `activity.changed` — `{ ambient, recent }`, the state.
- `activity.triggered` — one activity, the event that grew the island.

### 5.4 The command surface

Everything the UI can ask for. `isled call <name> [args]`; the args are JSON,
or a bare number where that reads better.

```
ping                    system.stats              view.set               view.prev / view.next
island.toggle           island.focus

activity.list           activity.trigger          activity.dismiss       activity.clear
todos.list              todos.add                todos.toggle           todos.remove
todos.reorder           todos.clear_done
timer.status            timer.start              timer.pause            timer.resume
timer.stop              timer.durations
notes.list              notes.get                notes.set              notes.new
notes.remove            notes.dir                notes.obsidian
clipboard.list          clipboard.add            clipboard.remove       clipboard.pin
clipboard.clear         clipboard.paste          clipboard.capture
wallpaper.list          wallpaper.set            wallpaper.setdir       wallpaper.rescan
wallpaper.loop          wallpaper.next           wallpaper.status
calendar.events         calendar.dir
settings.get            settings.set
network.wifi            context.summary
notifications.dismiss   notifications.clear       notifications.action    notifications.send
```

`view.set` accepts either `{"index": 3}` or a bare `3`, so a compositor
keybind is one argument with no quoting to get wrong.

### 5.5 The state on disk

```
~/.local/state/isled/     todos.json  timer.json  clipboard.json  settings.json
~/.local/share/isled/     (wallpaper cache, reserved)
~/Documents/isle-notes/   the notes, as markdown
```

Writes are atomic: the payload lands in a temporary file which is renamed
over the target, so a crash mid-write leaves the previous file intact.

## 6. Prior art — read before writing code

**DankMaterialShell** (`AvengeMedia/DankMaterialShell`) is a Quickshell shell
for niri + hyprland that already ships an island, notification center, control
center, notepad, khal calendar, wallpaper switching with matugen, and lock
screen — **and solved its architecture the same way this project did**: a
compiled core daemon over IPC (theirs is Go, ours is Rust). Free reference for
~60% of the feature list.

Also worth skimming: `end-4/dots-hyprland`, `caelestia-dots`, `noctalia`.

**Ryoku itself is also prior art**, and it is running on this machine right
now. Read `~/.config/niri/` and the Ryoku NixOS modules before reinventing
session bootstrap, since we are replacing it.

## 7. How it is installed

The shell is a Home Manager module in this repo, and the target system
flake imports it:

```nix
# in the system flake
isle.url = "github:reze-dev/dynamic-island";

# in a home module
imports = [ inputs.isle.homeModules.default ];
programs.isle = {
  enable = true;
  claimNotifications = true;            # the bus name is free now
  sessionTargets = [ "graphical-session.target" ];
  compositorService = "niri.service";  # the island needs WAYLAND_DISPLAY
};
```

`nix/keybinds.nix` produces the compositor binds; the system flake merges
them into niri's own.

Two things about the targets are worth knowing, because both were wrong
before they were right:

- **`graphical-session.target` is the target that exists.** The
  compositor-specific targets some compositors ship do not exist on
  NixOS, and a unit whose `WantedBy` names a target that does not exist
  is never started at all — not started *later*, never.
- **The island has to wait for the compositor.** It needs
  `WAYLAND_DISPLAY`, and the compositor is what puts it there. Started at
  the same time as the session, the island can win the race and find no
  display. `compositorService` names the compositor's unit so the island
  waits for it.

The daemon is also put on the user's `PATH`, because the keybinds run
`isled call ...` by name and a service's binary is not on anyone's path.

## 8. What to do first, in this order

Done:

1. ~~Confirm the NixOS box has niri + hyprland~~ — both present.
2. ~~`nix search nixpkgs swww`~~ — moot, it is `awww` 0.12.1 and exists.
3. ~~awww probed and working~~ — fractional scale 1.25 on amdgpu renders fine.
4. ~~`isled` scaffolded~~ — IPC handshake, snapshot, unsolicited events,
   `isled call`. 168 tests, clippy clean.
5. ~~Quickshell skeleton~~ — loads, connects, and drives the island.
6. ~~`nix/module.nix` written and evaluating~~ against HM master.
7. ~~**The broken wiring is fixed.**~~ `Island.qml` referenced an identifier
   called `root` that did not exist, and `PanelHost.qml` looked for an
   `isled` property on its parent that was not there — so the context panel
   showed dashes and the notifications panel was never fed. Both were dead
   code paths that had never been run.
8. ~~**The morph actually animates now.**~~ The default configuration bound the
   island's width to its own implicit width, which is a binding loop, so the
   pill snapped. The surface is now fixed and only the island inside it
   changes shape.
9. ~~**All nine panels exist and are reachable.**~~
10. ~~**The activity engine exists and the island reacts to it.**~~

Still open, in this order:

11. ~~**Wire the shell into the system flake.**~~ Done: `/home/reze/Michi`
    imports the island, the module is enabled, the keybinds are merged into
    niri's, mako is gone and the island owns the notification bus name.
    **It still needs `nixos-rebuild switch`, and then a logout or a
    restart of the session** — the island's target hangs off the graphical
    session, so it starts with the session rather than with a rebuild.
12. **The system tray.** `org.kde.StatusNotifierWatcher` has no owner on
    this machine now. The old shell was the last one, so tray icons have
    nowhere to appear. The plan lists it as an obligation the island
    inherited (§3.5), and it is the one thing Ryoku did that the island
    does not yet.
13. **Look at the island.** There are three ways, in increasing order of
    how much they tell you:

    - `just preview` — photographs the island in each of its appearances
      without a compositor, a daemon or a screenshot tool.
    - `just preview-live` — the same, but against a seeded daemon: real
      wallpapers, a real calendar, real notes, todos and notifications, so
      every panel is photographed with something in it. **An empty panel is a
      forgiving thing — it hides every layout fault there is.** This is the
      one that found the nine visual bugs fixed in the last commit.
    - `just demo-anim` — renders the island headless, photographs it *while
      the `Behavior` animations are running*, and assembles the frames into
      `/tmp/isle-demo/island.gif`. The morph is the one thing a screenshot
      cannot show.
    - `just demo` — the real thing: the daemon on a scratch socket, the
      real shell on the real screen, and a scripted sequence of events with
      narration. This is the one to judge the animation by.

    To poke at it by hand, `just trigger media|notification|todo|clipboard|
    timer|panel` fires a single event at the running daemon.
14. **The focus matrix** (`PLAN.md` §5.2) on both compositors. Nothing is
    proven about focus-release-on-collapse, and it is the top-ranked risk.
15. **Ryoku cutover.** The daemon can already claim
    `org.freedesktop.Notifications` (opt in with `--claim-notifications`),
    but `org.kde.StatusNotifierWatcher` is still owed.
16. **Verify environment forwarding** — until `isled` reports
    `compositor=niri` from a systemd unit rather than `unknown`, the unit
    wiring is unproven.

## 9. Open decisions — ask before the relevant phase

1. Adopt a pre-existing default-namespace awww daemon, or always
   `--namespace isle`? (before Phase 3)
2. Phase 2 read-only calendar, or defer the whole calendar module to Phase 5?
   — **done, the calendar is implemented.**
3. Calendar source: own ICS parsing, `khal`, or `vdirsyncer`?
   — **own ICS parsing, done.**
4. Todos: plain SQLite, or Taskwarrior-compatible export?
5. Claim `org.freedesktop.Notifications` on day one, or shadow Ryoku's until
   cutover?
6. Wallpaper cache cap — 10 GB default, or tune to the actual disk?
7. Blur on niri: external implementation, or flat styling? — **flat for now.**
8. **Keybind scheme.** `Mod+Ctrl+1..9` + `Mod+Ctrl+J` + `Mod+Ctrl+K`. Decided
   in commit `88e9a4e`; `PLAN.md` and older copies of this file still say
   `Mod+Alt`, which is wrong — `Mod` resolves to Super, and `Mod+Alt` collides
   with Ryoku's move-window-to-workspace.
9. **Video wallpapers at all?** awww cannot do it; mpvpaper is a separate
   daemon. Decides whether browser handoff ships.

## 10. Honest effort estimate

Phases 0–3 (~2 weeks) already yield a genuinely usable island, and most of
that is now built. Phases 4–6 are the remote wallpaper sources, the fuller
data model and the context engine.

**That estimate does not include removing Ryoku.** Porting its session
management, tray, notification ownership and idle handling is additional work on
top. Treat "island done" and "Ryoku gone" as separate milestones.
