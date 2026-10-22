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

The visual reference is `dynamic-island-material.html` — a mock in this
directory. It is a *visual* reference only. Its panels are the starting spec
(`context | media | clipboard | calendar | notes | notifications | network`);
the theme switcher was added later, and **todos** is the ninth.

## 2. Where everything is

```
/home/reze/Projects/dynamic-island/
├── dynamic-island-material.html                the visual reference
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
│   ├── components/                            the Material component library:
│   │                                           Icon, Card, Chip, StatTile, Tile,
│   │                                           IconButton, Button, Toggle, TextField,
│   │                                           ProgressRing, ActivityRow, SlideActivity,
│   │                                           MiniBar, SectionHeader, Snackbar,
│   │                                           StateLayer, Ripple
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
| 2 | Naming: `isle` (UI), `isled` (daemon) | Chosen by user. The project is called **Dynamic Island**; the old "ctOS" naming, from a design the user has since dropped, is gone from the shell, the mock and the docs. |
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
| 17 | **The UI is Material 3** | The design target is `dynamic-island-material.html`: Material's colour roles, shape scale, type scale and state layers, with Material Symbols as the icon set. See §5.0. |

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

### 4.5 What the old shell owned, and what owns it now

| Bus name | Owned by |
|---|---|
| `org.freedesktop.Notifications` | the island, with `--claim-notifications` |
| `org.kde.StatusNotifierWatcher` | the island, with `--claim-tray` |

Both are asked for with `DoNotQueue` and without `ReplaceExisting`, so the
daemon refuses the name rather than taking it from whatever already has it.

### 4.6 What Ryoku owns that we inherit

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

### 4.7 Compositors

**Niri** — `$NIRI_SOCKET`, newline-delimited JSON. `Request::EventStream` (since
0.1.9) streams full state then deltas; no polling. Detect via `$NIRI_SOCKET`.

**Hyprland** — `hyprctl -j` for queries, `event/socket2` for push. Detect via
`$HYPRLAND_INSTANCE_SIGNATURE`.

### 4.8 Research traps — things that will waste your time

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

### 4.9 Quickshell constraints, found the hard way

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
- **`QtObject` has no default property**, so a bare `Timer { }` child of one
  fails with "Cannot assign to non-existent default property". Declare it
  as `property Timer foo: Timer { }` instead.
- **The bus runs its own executor**, which has no tokio runtime of its own.
  A `tokio::spawn` inside a `#[zbus::interface]` method panics with "there
  is no reactor running". Make the method `async` and await the work
  directly.
- **A file loaded as a root config does not get the parent directory's
  imports.** `import ".."` from a subdirectory resolves to nothing when
  quickshell loads that file directly, so the singletons and the types in
  `ui/qmldir` are not in scope. A file that is *imported* by another file
  (a panel, say) resolves them fine. The preview rig lives at `ui/preview.qml`
  for exactly this reason: it has to be in the same module as the island.

### 4.10 Blur: niri can, and Quickshell 0.3.2 can ask

niri 26.04 implements `ext-background-effect-v1` with blur, and also speaks
the older `org_kde_kwin_blur`.

**Quickshell 0.3.2 exports the QML type.** `Quickshell.Wayland` has a
`BackgroundEffect` attached property, and `Quickshell.Region` describes the
area to blur:

```qml
import Quickshell.Wayland

PanelWindow {
    BackgroundEffect.blurRegion: Region { item: island; radius: island.radius }
}
```

The region follows the item, so the blur follows the island as it morphs.
Setting `blurRegion` to null removes the blur, which is what the island does
when it is not open: the idle pill is a hole in the screen and there is
nothing behind it worth blurring.

Verified on niri 26.04: the expanded panel is genuinely frosted, with the
wallpaper showing through it.

Two things worth keeping straight:

- A translucent surface looks like blur and is not. The panel's own glass
  (`rgba(20, 20, 20, 0.88)`) and the compositor's blur are different
  mechanisms; the shell uses both.
- The blur is the compositor's, so it needs the compositor's protocol. On a
  compositor without it, the island still works and the panel is just glass.

### 4.11 Two Quickshell limits that shape the whole UI

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

Every Quickshell surface is `namespace="quickshell"`, so a compositor rule
cannot single out the island's surface from another Quickshell shell's. The
blur does not need it — the surface asks for its own blur region — but a
compositor-side rule would.

Both limits disappear at Ryoku cutover.

### 4.12 The keyboard, the clicks, and niri's on-demand rule

This is what the first run on a real machine taught us, and it is worth
more than everything above it. The island ate every click and every
keystroke, and both faults were the surface's own.

**Niri gives the keyboard to a newly mapped on-demand layer surface.**
From niri's `src/handlers/layer_shell.rs`, on the commit that maps a
layer surface:

> Give focus to newly mapped on-demand surfaces. Some launchers like
> lxqt-runner rely on this behavior. While this behavior doesn't make
> much sense for other clients like panels, the consensus seems to be
> that it's not a big deal since panels generally only open once at the
> start of the session.

It is a big deal for a panel. A surface that is focusable from the start
takes the keyboard at login and holds it, and niri's focus resolution
puts the top layer's on-demand surface *above* the focused window, so
the terminal cannot have it back. And the keyboard only goes back to
whatever was clicked (`focus_layer_surface_if_on_demand`, in
`src/input/mod.rs`), so if the surface covers the click, the island
keeps it forever.

**The clickthrough mask.** `WindowInterface.mask` — inherited by
`PanelWindow` — sets the input region:

```qml
PanelWindow {
    mask: Region {
        regions: [
            Region { item: island; radius: island.radius },
            Region { item: snackbar }
        ]
    }
}
```

Without it the surface is as wide as the screen and as tall as the
tallest panel, so every click in that corner belongs to the island
rather than to the window underneath. With it, a click outside the
island lands where the reader clicked, and *that* is what returns the
keyboard.

**What the shell does now.** The surface is focusable only while a
panel is open, so it never takes the keyboard at startup and a
notification opening a panel does not take it either. The island asks
for the keyboard when the panel opens (`onFocusableChanged`), because
the click that opened it arrived before the window was focusable. The
panel gives it back when it closes.

On the wire, with `WAYLAND_DEBUG=1`:

```
zwlr_layer_surface_v1.set_keyboard_interactivity(0)   # startup: none
zwlr_layer_surface_v1.set_keyboard_interactivity(2)   # a panel opened: on demand
zwlr_layer_surface_v1.set_keyboard_interactivity(0)   # the panel closed: none
wl_surface.set_input_region(wl_region#...)            # the island's shape
```

Quickshell applies the change to a mapped surface, which the layer
shell protocol says it should not, but wlroots takes it.

**Esc needed the fields to stop eating it.** Every `TextField` consumed
Esc to clear itself, so a panel with a field in it could not be closed
at all: the key never reached the island. A field clears itself now,
and lets the key through when there is nothing left to clear.

**And the fields were bound to a property that typing breaks.** `text:
panel.filter` plus `onTextChanged: panel.filter = text` looks like a
two-way binding and is not: the first keystroke removes the binding, and
from then on the box and the state drift apart. Clearing the state
cleared the filter but not the box, the todo field showed a task that
was not there, and picking another note changed the title and left the
body of the last one on screen. The field is the state now, in all
three, and they cannot drift apart.

## 5. What was built, and what it looks like

### 5.0 The design system — Material 3

The design target is `dynamic-island-material.html` in the root of the repo:
a Material-dark mock of the same shell. The UI follows it.

**Colour.** Material's role names, in the dark scheme, anchored on the
mock's single light-blue accent: `primary` `#d2e3ff`, `onPrimary`
`#00325b`, `surface` `#111`, `onSurface` `#e3e3e3`, `onSurfaceVariant`
`#9aa0a6`, `outlineVariant` `#40474f`, and so on. Everything visual comes
from `ui/Theme.qml`; there is not one hard-coded colour left outside it.

> QML reserves the `on` prefix for signal handlers, so a property called
> `onPrimary` would be read as a handler for `primaryChanged` and would
> never be given a value — it would silently stay black. The colours
> therefore live under their own names (`primaryInk`) and the Material names
> are aliases onto them.

**Shape.** Material's scale: 4, 8, 12, 16, 28, and full. The island's panel
is a 28; a chip is an 8.

**Type.** Material's scale, as `Theme.type.<role>.{size, line, weight,
tracking}`: `titleMedium` is 16/500, `bodyMedium` is 14/400, `labelSmall`
is 11/500 with half a pixel of tracking.

**State.** Material's state layers — an 8% wash on hover, 10% on press, 16%
when dragged — plus the ripple that spreads from the point of contact.
`ui/components/StateLayer.qml` and `ui/components/Ripple.qml`.

**Motion.** Material's emphasized curve is `cubic-bezier(0.2, 0, 0, 1)`. QML
will not take a JavaScript function for an easing curve, so the built-in
`Easing.OutCubic` stands in for it; it passes through the same three points
(0.60, 0.88, 0.98 against 0.61, 0.88, 0.98).

**Icons.** Material Symbols, Rounded style, and the icons are a *font* — the
`Material Symbols Rounded` family, drawn as a ligature. A font is the best
renderer there is for a glyph: it is hinted for the size it is drawn at, it
takes its colour from the text, and it comes with the variable axes the
icons were designed around, so `FILL` gives the filled variant of any icon
and `wght` its weight.

The text is drawn with `renderType: Text.NativeRendering`, which uses the
platform's font engine. Qt's other path turns the glyph into a path and
rasterises that, which flattens the curves at the wrong resolution and
gives the icons exactly the ragged edges the font was meant to avoid.

`ui/Icons.qml` holds the translation between the shell's own kebab-case
names and the names the font knows, and it holds the path data for the
fallback: a machine without the font installed draws the icons from that
instead, which is the same artwork, taken from the font's own outlines, but
rougher. The target system has the font — it is in
`Michi/modules/system/base.nix` — and this repository's dev shell pulls it
in, so the fallback is only for a machine that has neither.

**Blur.** The expanded panel is glass: `rgba(20, 20, 20, 0.88)` over
whatever the compositor blurs behind it. See §4.10.

### 5.1 The island — four appearances, three sizes

| Appearance | Size | What it shows |
|---|---|---|
| **idle** | 140×34 | A coloured dot and the clock. Nothing else. |
| **peek** | 300×42 | On hover: the last thing that happened, plus a sliver of CPU and memory. |
| **triggered** | 300×42 | An event grew the island. It shows the glyph, the headline and the supporting line, then folds back on its own. |
| **full** | 480 × per-panel | A panel. Reached by a keybind or by clicking the island. |

The dot's colour is the *tone* of whatever the island last reacted to:
neutral, info (the accent blue), good (green), warn (amber) or critical
(red). A sticky activity (music playing, a timer running) sets the tone and
keeps setting it; a transient one (a notification) sets it and then lets the
ambient condition take it back.

The morph is `Behavior on width/height/radius` with Material's emphasized
curve over 500 ms. The activity content **slides** rather than cross-fading,
because a cross-fade reads as "a new window opened" and a slide reads as
"the same thing saying something else" — `ui/components/SlideActivity.qml`.

### 5.2 The panels

| # | Panel | Where its data comes from |
|---|---|---|
| 1 | **context** | The daemon's `context.summary`: vitals, what is happening now, the timer, the next todo. |
| 2 | **media** | `Quickshell.Services.Mpris`. No Rust involved. |
| 3 | **notifications** | The daemon, which owns `org.freedesktop.Notifications`. |
| 4 | **clipboard** | The daemon's history, which fills itself: `wl-paste --watch` runs a hook that files each selection, so a multi-line selection survives intact. `wl-copy` to paste back. |
| 5 | **calendar** | The daemon reads `.ics` files and lays the next fourteen hours out by the hour. |
| 6 | **todos** | The daemon's list, plus the pomodoro. A deadline can be written inline: `standup 15m`. |
| 7 | **notes** | Markdown files in a directory. Point it at an Obsidian vault and the panel offers to open the note there. |
| 8 | **network** | `Quickshell.Networking` and `Quickshell.Bluetooth`, plus `Quickshell.Services.UPower` for the battery. |
| 9 | **theme** | The daemon's wallpaper catalog: a grid of thumbnails, a directory to change, and the rotation switch. |
| — | **tray** | The daemon is a StatusNotifierHost. Reached with `Mod+Ctrl+T` rather than a number, because a system tray is a different kind of thing from a panel. |

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
tray.list               tray.activate           tray.secondary
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
    niri's, mako is gone, and the island owns both
    `org.freedesktop.Notifications` and `org.kde.StatusNotifierWatcher`.
    **It still needs `nixos-rebuild switch`, and then a logout or a
    restart of the session** — the island's target hangs off the graphical
    session, so it starts with the session rather than with a rebuild.
12. ~~**The system tray.**~~ Done: the daemon is a StatusNotifierHost, and
    the panel draws the items. `just tray` puts a fake item in it.
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

### 8.1 After the Material pass — what is next

The UI is Material 3 now (§5.0) and the icons are the Material Symbols
font. The seeded preview (`just preview-live` — panels with real data in
them) is what finds layout faults: an empty panel is a forgiving thing.
The live machine found the rest (§4.12).

**Done:**

- The calendar's empty state sat on top of the hour list. It takes the
  list's place now, and the list hides itself when there is nothing in it.
- The notes list had `elide` on a title that wrapped by default, so
  titles ran to three lines and overflowed their card.
- Nothing selected the first note, so the editor opened blank. It does
  now — but only when the selected note has gone away, because the
  daemon re-reads the notes directory on a timer and a refresh must not
  yank the note out from under the reader.
- "Clear all" sat below the right-aligned count. The count and the
  action share the header's trailing edge now.
- The header had a fifth fault: ten navigation buttons and a panel name
  do not fit in 480 pixels, and "notifications" came out as "notificati…".
  The panel is 520 wide now.
- The snackbar was built and wired to nothing. `IsledBridge.announce()`
  says it, `callAndSay()` says it and waits for the daemon's answer, and
  the clipboard, todos, network, notifications and theme panels use it.
  The preview rig photographs it, since it lives in the shell and not in
  the island.
- `FocusProbe.qml` and `FocusProbeItem.qml` are gone. Nothing
  instantiated them, and `TextField.qml` covers the same ground.
- Small things the renders turned up: the bluetooth section was headed
  "Devices", which is what the Ethernet section is for; it also inherited
  the height of the empty Ethernet section and sat out of line with the
  Networks header beside it; and four panels said "1 ITEMS".
- **The island ate every click and every keystroke on the real
  machine.** The surface had no input region, so every click in the top
  of the desktop was the island's, and niri gives the keyboard to a
  newly mapped on-demand layer surface, so it took the keyboard at login
  and held it. Fixed: a clickthrough mask, and focusable only while a
  panel is open. See §4.12 for the whole story.
- **The text fields swallowed Esc**, so a panel with a field in it could
  not be closed at all. And the fields were bound to a property that
  typing breaks, so clearing the state did not clear the box.
- The mock's page header is gone: the "ctOS" brand plate and the corner
  clock. The mock is a demo page with its own system bar; a real desktop
  already has one, and the island's pill shows the time anyway. The
  ctOS name is gone from the shell, the mock and the docs; the project
  is called Dynamic Island.

**Next:**

1. **The focus matrix, properly.** The chain is now right — click the
   field, type, Esc clears it, Esc closes the panel, and the keyboard
   goes back to the terminal — but none of it has been tested end to end,
   because that needs synthetic input and `/dev/uinput` is root-only.
   `tools/fake-input.py` is the injector, written and ready; it needs
   `/dev/uinput` to be readable. Everything else about the keyboard is
   verified from niri's source and on the wire.
2. **A keybind.** The island opens by being clicked and nothing else.
   There is no `bind` in the niri config, and no global hotkey, so the
   only way to open it is the mouse. A key to open it, and the same key
   to close it, would make the whole thing usable without the mouse.
3. The media panel has never been photographed with anything in it — it
   reads MPRIS, and there is no fake player the way there is a fake tray
   item. Worth an `examples/fake-mpris.rs`, the way `fake-tray.rs` is
   there for the tray.
4. The open decisions in §9 are stale: #1, #4 and #5 are answered by
   code that already exists, and #7 ("Blur on niri: flat for now") is
   wrong — the blur is real (§4.10). Close them out.
5. Ryoku cutover, which would also lift §4.11's two Quickshell limits.

## 9. Open decisions — ask before the relevant phase

1. Adopt a pre-existing default-namespace awww daemon, or always
   `--namespace isle`? — **always `--namespace isle`.** The island starts
   its own awww daemon in that namespace, so there is exactly one and it
   cannot collide with anyone else's.
2. Phase 2 read-only calendar, or defer the whole calendar module to Phase 5?
   — **done, the calendar is implemented.**
3. Calendar source: own ICS parsing, `khal`, or `vdirsyncer`?
   — **own ICS parsing, done.**
4. Todos: plain SQLite, or Taskwarrior-compatible export?
   — **plain SQLite.** The export is a separate job; nothing needs it yet.
5. Claim `org.freedesktop.Notifications` on day one, or shadow Ryoku's until
   cutover?
   — **claim it.** Ryoku is gone; the island owns the bus. It asks for the
   name without queueing, so if something else has taken it, it says so
   rather than stealing it.
6. Wallpaper cache cap — 10 GB default, or tune to the actual disk?
   — **10 GB, and leave it.** It is a ceiling, not a quota.
7. Blur on niri: external implementation, or flat styling? — **the real
   thing.** niri 26.04 has `ext-background-effect-v1`, Quickshell 0.3.2 can
   ask for it, and the island asks (§4.10). The glass is a dark tint over
   it: the mock's `.88` over a black page, a little less here, because the
   backdrop is a real screen and not a page.
8. **Keybind scheme.** `Mod+Ctrl+1..9` + `Mod+Ctrl+J` + `Mod+Ctrl+K`. Decided
   in commit `88e9a4e`; `PLAN.md` and older copies of this file still say
   `Mod+Alt`, which is wrong — `Mod` resolves to Super, and `Mod+Alt` collides
   with Ryoku's move-window-to-workspace.
   — **still the scheme, and now the only way in.** `Mod+Ctrl+J` is
   `island.toggle`: the same key opens and closes the island, and it works
   whether or not the island has the keyboard. The binds exist in
   `nix/keybinds.nix` and the Michi flake merges them into the niri config,
   but the config on the machine was built before they did, so the island
   currently opens by being clicked and nothing else.
9. **Video wallpapers at all?** awww cannot do it; mpvpaper is a separate
   daemon. Decides whether browser handoff ships.
   — **not yet.** The browser handoff ships for stills; video is a separate
   daemon and a separate decision.

## 10. Honest effort estimate

Phases 0–3 (~2 weeks) already yield a genuinely usable island, and most of
that is now built. Phases 4–6 are the remote wallpaper sources, the fuller
data model and the context engine.

**That estimate does not include removing Ryoku.** Porting its session
management, tray, notification ownership and idle handling is additional work on
top. Treat "island done" and "Ryoku gone" as separate milestones.
