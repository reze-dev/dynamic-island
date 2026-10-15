# Dynamic Island Shell — Implementation Plan

A context-aware, dynamically morphing island shell for niri and hyprland.
Reference mock: `ctos-dynamic-island-context-aware-v2.html` (visual reference only).

**Scope note:** this shell is the successor to Ryoku, not a companion to it.
`isled` is designed as the session's core daemon from day one. See §3.5.

**Environment note:** the authoring machine *is* the target — NixOS 26.11,
niri + hyprland, systemd user sessions. There is no separate staging box.
See §7 Phase 0.

| | |
|---|---|
| Display name | Dynamic Island Shell |
| Daemon | `isled` (Rust, no GUI) |
| UI | `isle` (Quickshell / QML) |
| IPC socket | `$XDG_RUNTIME_DIR/isled.sock` |
| State | `~/.local/state/isled/` |
| Data | `~/.local/share/isled/` |

---

## 1. Recommendation (TL;DR)

**Rust core daemon (`isled`) + Quickshell/QML UI (`isle`)**, talking over a Unix socket.

```
 compositor IPC        Linux services (D-Bus/CLI)      files
 (hyprctl/niri)              │                            │
        └──────────────┬─────┴─────────────┬──────────────┘
                       ▼                   ▼
                 ┌───────────┐       data dirs
                 │   isled   │  (todos, notes, clipboard, wallpaper catalog)
                 │  (Rust)   │──── SQLite / JSON / image cache
                 └─────┬─────┘
                       │ Unix socket, JSON lines
                       │  ├─ state snapshot on connect
                       │  ├─ event stream (unsolicited)
                       │  └─ commands (UI → core)
                 ┌─────▼─────┐
                 │   isle    │  island + 9 panels
                 │  (QML)    │
                 └────────────┘
```

Rationale: the morphing UI is a solved problem in QML and an unsolved problem in
Rust; the context engine is a backend problem that belongs in Rust with real
crates and real tests. Splitting along that line is cheaper than either pure
option.

### Effort comparison

| Approach | v1 (island + morph + media + notifs + control center) | All features incl. wallpapers |
|---|---|---|
| Pure Quickshell (shell out to CLIs) | ~4-6 days | **3-4 weeks** |
| **Rust core + Quickshell UI** | ~7-9 days | **4-5.5 weeks** |
| Pure Rust (winit + layer-shell + egui) | ~3-4 weeks | **8-12 weeks** |

---

## 2. Quickshell vs. native Rust

### 2.1 The deciding factor

The split between UI work and plumbing work is roughly **25% / 75%** for this project.

- **Plumbing** (75%): event aggregation, IPC parsing, D-Bus, persistence, timers,
  state machines, scheduling, HTTP/image fetching. Rust wins decisively — real
  crates, typed, testable headless.
- **UI** (25% by volume, but where the difficulty is): morphing geometry, text
  fields, IME, scrolling, HiDPI. QML wins decisively — it is declarative and
  already has the exact primitive this project is named after.

Pure Rust pays a large tax on that 25% to get a modest win on the 75%. The
hybrid pays a small tax (one IPC protocol) and gets both wins.

### 2.2 Feature-by-feature

| Capability | Quickshell | Pure Rust |
|---|---|---|
| Layer-shell surface | `PanelWindow`, built in | `layershellev` (crates.io) or `winit-layershell` (git-only). Manual anchor/margin/exclusive-zone plumbing. |
| **Morph animation** | `States` + `Behavior` on x/y/w/h/radius. ~10 lines. | Hand-rolled animation state machine + custom layout. Weeks. |
| Text input / IME | Built in, excellent | `egui` handles basic IME; CJK preedit, paste-into-field, undo: all manual. Weeks. |
| Free-form layout | `Positioners`, `Row`/`Column`/`Grid`, implicit sizing | `iced` is grid-based — you end up writing a custom widget system. `egui` is immediate-mode: no retained state, you manage animation yourself. |
| Notifications | `Services.Notifications` = **be** the daemon | Bind `org.freedesktop.Notifications` yourself |
| MPRIS | `Services.Mpris` (players, metadata, art, position) | `mpris` crate — fine, easy |
| Audio / BT / battery / network | `Io.Speakers`, `Services.Bluetooth`, `Io.Battery`, `Io.NetworkModel` | `pipewire`, `bluer`, UPower — all better, but more code |
| Compositor IPC | `Quickshell.Hyprland` built in; niri via raw socket | `niri-ipc` crate (exact-version pinning, *not* semver-stable). Hyprland has no equivalent of quality — shell out to `hyprctl` or use community `hyprland-rs`. |
| Screencast / meeting detection | `Wayland.ScreenCast` | `pipewire` + `zwlr-screencast` manual |
| HTTP + image decode | `QtNetwork`, QML `Image` | `reqwest`, `image` — better |
| Persistence | QML + `QtCore` settings, or shell out | `rusqlite` / `serde` — better |
| **Iteration speed** | **Hot reload** — edit QML, live re-render, seconds | Rebuild + relaunch, seconds-to-minutes |
| Tests | QML has essentially none | `cargo test` on the whole core, headless, no compositor needed |
| Footprint | ~120-200 MB RSS | ~30-60 MB |
| Packaging | Qt + quickshell deps | One static-ish binary |

### 2.3 What the Rust route actually costs you

The honest list of things you would be writing by hand that Qt gives you free:

1. **Layer-shell + focus lifecycle.** Anchors, exclusive zone, margins,
   keyboard interactivity, and — critically — releasing focus when the island
   collapses. Both niri and hyprland need *different* handling here and neither
   crate hides it.
2. **An animation system.** Easing, interruption, spring vs. curve, frame
   scheduling, and driving 60 fps at whatever the compositor will give you.
3. **Text layout and input.** Shaping, ellipsis, bidi, HiDPI, and a real text
   field with IME. Notes + calendar + todo entry all need this.
4. **A widget system.** The mock has pills, cards, lists, grids, segmented
   controls, progress bars, icon sets, scroll areas.

Items 3 and 4 are the killers — and you need them for panels 5, 6 and 7.

### 2.4 Prior art

**DankMaterialShell** (`AvengeMedia/DankMaterialShell`) is a Quickshell shell for
niri + hyprland that already ships: an island, notification center, control
center (network/BT/audio in-out/displays/night mode), notepad, khal calendar,
wallpaper switching with matugen theming, lock screen, and **a compiled core
daemon in Go exposed over IPC**. That is the architecture above, arrived at
independently by people who had already shipped it.

Read it before writing anything. It is effectively a free reference
implementation for ~60% of this feature list.

Also worth reading: `end-4/dots-hyprland`, `caelestia-dots`, `noctalia`.

### 2.5 When pure Rust would still be right

- You want **one static binary, no Qt**, and will accept a smaller feature set.
- You are specifically trying to learn Rust — the plumbing is where Rust shines
  and this project is 75% plumbing.
- You only need the island + media + notifications, and can live without rich
  text entry (no notes, no calendar editing, no todo input).

If you pick pure Rust, the stack is: `winit` + `winit-layershell` +
`egui`/`eframe` (or `iced` if you want retained state and will accept writing a
custom layout). Budget +4-7 weeks over the hybrid and cut panels 5/6/7 first.

---

## 3. Core architecture

### 3.1 Process split

**`isled` (Rust, no GUI)**
- Owns *all* state. The UI is stateless-with-respect-to-truth.
- Source implementations behind one `EventSource` trait.
- Persistence in `~/.local/state/isled/` (SQLite for clipboard/history/todos/
  wallpaper catalog, plain text for notes, JSON cache for calendar/media).
- Single-threaded event loop over `tokio`; each source is a task feeding a
  broadcast channel.

**`isle` (QML)**
- Owns *presentation only*: morph geometry, animation, panel layout, focus.
- Connects on startup, receives a full state snapshot, then applies deltas.
- Sends commands (set volume, toggle wifi, create todo, apply wallpaper).
- Optimistic UI where latency matters (volume slider), reconcile from events.

### 3.2 IPC protocol

Unix socket at `$XDG_RUNTIME_DIR/isled/isled.sock` (matching `RuntimeDirectory`
in §9.3; overridable with `$ISLED_SOCKET`). Newline-delimited JSON.

Three message shapes, discriminated by `t`:

- `{"t":"state", ...}` — full snapshot, sent once on connect.
- `{"t":"event", "topic":"volume", "data":{...}}` — unsolicited delta.
- `{"t":"cmd","id":7,"name":"volume.set","args":{"level":0.4}}` — request, with
  `{"t":"reply","id":7,"ok":true}` following.

Keep it this dumb. No codegen, no versioning ceremony until you have two
clients. Topics map 1:1 to Rust enums so adding a field is non-breaking.

**The UI side is constrained by Quickshell, not by us.** Its QML engine exposes
only `QtQuick` and `Quickshell.*`; `QtNetwork` is not linked, so `Socket` and
`TextStream` from Qt simply do not exist. The only route is
`Quickshell.Io.Socket`, whose read side takes a `DataStreamParser`. Two sharp
edges, both hit in practice:

- `StdioCollector` is the easy parser, but its `text` is **cumulative**, not
  per-chunk. Re-reading it reprocesses the whole buffer every time. Track how
  much has been consumed and slice. (`SplitParser` has no default property and
  builds its children dynamically, so you cannot just declare one inline.)
- `waitForEnd` defaults such that nothing is delivered incrementally; set it to
  `false` or the stream appears dead.

`isled` must handle `SIGTERM` and unlink the socket on the way out — systemd
sends SIGTERM on every stop, and a leaked socket is a stale-socket race on the
next start. There is also a `trigger_shutdown` seam for tests and future
`isled reload`.

### 3.3 Directory layout

```
dynamic-island-shell/
  core/                 Rust workspace
    isled/              daemon binary
    isle-core/          lib: sources, state, scoring, persistence, wallpaper
  ui/                   Quickshell config
    shell.qml           root, PanelWindow
    Island.qml          morph engine
    panels/             one file per panel
    components/         Pill, Card, Icon, SegmentedControl, ...
  nix/                  flake, module
```

### 3.4 Backend selection per feature

| Feature | Backend |
|---|---|
| Wi-Fi / ethernet | NetworkManager over D-Bus (`nm-dbus`) — not `nmcli` parsing |
| Bluetooth | `bluer` crate (Bluez D-Bus) |
| Volume / in-out | `pipewire` + `pw-volume` (same pipe as Qt's backend) |
| Brightness | `brightnessctl` subprocess (sysfs is a mess of edge cases) |
| Dark/light mode | `gsettings` color-scheme + Qt `palette` + emit a reload for other apps |
| Wallpaper rendering | `awww` (default), `mpvpaper` (video, if ever), `hyprpaper` — see §4 |
| Wallpaper catalog | local FS + Wallhaven API + browser-handoff — see §4 |
| Theming | palette from API/image, `matugen` to push to Qt/GTK/terminals |
| Media | `mpris` crate |
| Notifications | `zbus` implementing `org.freedesktop.Notifications` on the session bus — you become the daemon, see §3.6 |
| Clipboard | `wl-clipboard` watch on primary selection; paste-back via `wtype` |
| Calendar | ICS via `icalendar`/`chrono`; optionally `khal` |
| Todos | Own SQLite table; optional `taskwarrior` export |
| Weather | Open-Meteo (free, no API key) + `geoclue`/manual coords |
| CPU / mem | `sysinfo` crate |
| Workspaces / focus | compositor adapter (§6.2) |
| Meetings | `zwlr-screencast` node presence + window-title heuristics |
| System tray | `org.kde.StatusNotifierWatcher` on the session bus — inherited from Ryoku, see §3.5 |
| Idle / lock | logind `IdleHint`; lock screen is later scope |

---

### 3.5 Replacing Ryoku, not merely coexisting with it

**This shell is the successor to Ryoku.** Ryoku was an evaluation, not the
destination. The island is the *first* surface onto that shell, not the whole
of it. `isled` is therefore designed as the session's core daemon from day one
rather than as a single-purpose wallpaper/notification helper that later grows.
Designing it narrow means a second IPC revision and a state migration later;
designing it wide costs only some empty stubs now.

**Portals are not a dependency.** `xdg-desktop-portal`, `-gnome` and `-gtk` are
stock nixpkgs systemd user units (`linked-runtime`), not Ryoku's. ScreenCast,
Notification, Secret and OpenURI all keep working when Ryoku goes. Only
`-hyprland` needs enabling for the Hyprland session. This is the single biggest
"are we sure?" and the answer is yes.

What we do inherit:

| Ryoku unit | What it holds | Becomes |
|---|---|---|
| `ryoku-shell.service` | owns `org.freedesktop.Notifications` **and** `org.kde.StatusNotifierWatcher`; is the QML shell | `isle` + `isled` |
| `ryogami.service` | wallpaper daemon | `isled` wallpaper subsystem + awww |
| `hypridle.service` | idle detection + session lock | logind `IdleHint` in `isled`; lock screen is later scope |

`ryoku-rashin`, `ryoku-eq`, `ryoku-ai-usage`, `ryoku-materialize`,
`ryoku-palette-bridge`, `ryoku-bluetooth-reset` simply go away.

Note the notification daemon and the tray are *bus-name* obligations, not UI.
Whoever holds `org.freedesktop.Notifications` owns the session's notification
policy, and some apps behave differently depending on whether a tray host
exists. Both must be claimed before Ryoku is removed, or the gap is visible to
every other program on the bus.

**Consequences for the rest of this plan:**

- §9.4's keybind story gets *simpler*. We own the compositor config outright
  rather than threading binds around Ryoku's regenerated `rebinds.kdl`.
- §9.3's custom `isle.target` is no longer a workaround for Ryoku's single
  `systemd.target`; it is just correct for two compositors.
- §7's estimate in §10 does **not** cover porting Ryoku's session management.
  Treat "island done" and "Ryoku gone" as separate milestones; the former is
  Phases 0-6, the latter is additional work.
### 3.6 Owning `org.freedesktop.Notifications`

Notifications are a *bus-name* obligation, not a UI feature. Whoever holds the
name owns the session's notification policy; an app that starts while nobody
holds it typically queues or drops silently. This section records what it took
to claim it, because two of the three facts below are wrong in the obvious
direction and cost real time to find.

**zbus's `request_name` steals.** `Connection::request_name` is documented as
requesting "exclusive ownership without queueing (using `RequestNameFlags::
ReplaceExisting` and `RequestNameFlags::DoNotQueue` flags) since that is the
most typical case". `ReplaceExisting` means the name moves to the new owner if
the old owner permits replacement — silently, with no warning from zbus.
Calling it is how you quietly take over a live session. Always use
`request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())`, which
adds the queue suppression without the replacement: a taken name is an error
you can log rather than a name you just took. Verified against the live bus —
with Ryoku holding `org.freedesktop.Notifications`, isled logs `could not
claim ...: name already taken on the bus`, keeps serving IPC, and the owner is
unchanged.

**Owning the name is the only way to see notifications.** The spec has no
broadcast of `Notify` calls, so a client cannot observe other apps'
notifications. There is no passive mode: either you hold the name and receive
everything, or you see nothing. This is why `features.notifications` in the
snapshot means "we hold the name", not "notifications are implemented" — the UI
needs to tell the difference so it can explain an empty panel honestly instead
of implying nothing is happening.

**Promotion happens once, at ingest.** Hints arrive as `a{sv}`, whose values are
structurally untyped; each client layer renames things. Promote the handful the
UI actually reads — `urgency`, `category`, `desktop-entry` (with or without a
`.desktop` suffix), `transient` — into first-class fields at ingest and keep the
raw map alongside, rather than re-parsing the map in the view layer. Also note
`desktop-entry` may or may not carry the suffix; strip it once.

Array-valued hints are dropped: zvariant's `downcast_ref` has no container
support, and the hints this shell reads are all scalars, strings or booleans.
The raw keys are still kept, which is enough to know `image-data` was present —
pixel decoding is deliberately not done here.

**Actions and replaces.** Spec `actions` is one flat alternating array of
`(action_key, text)` pairs, so a malformed odd-length array must drop the
trailing element rather than panic. `Notify` takes `replaces_id`; a replace of a
still-live id reuses that id in place, while a replace of an expired or unknown
id is a new notification with a fresh id. Expiry is the daemon's job — a
background sweep removes expired entries and emits `NotificationClosed`, since
otherwise a timeout of 0 (the common case, meaning "never") means the store
grows forever.

The tray is a separate, still-outstanding obligation: §3.5 lists
`org.kde.StatusNotifierWatcher` alongside notifications, and only the latter is
done. Some apps change behaviour based on whether a tray host exists, so the
gap is visible even though it is quieter.


---

## 4. Wallpaper subsystem

Three separate concerns, deliberately kept apart:

```
   SOURCE                CATALOG                 ENGINE
 (finds things)   ──▶  (owns truth)   ──▶   (draws pixels)
 wallhaven api          SQLite                  awww
 local fs               on disk                  hyprpaper
 url-paste                                     mpvpaper (video)
 browser-handoff
```

The shell is the catalog owner. The engine is an external process we drive. They
never learn about each other.

### 4.1 The two traits

```rust
trait WallpaperSource {
    fn id(&self) -> &'static str;
    fn modes(&self) -> &[Acquire];              // what this source permits
    fn search(&self, q: &Query) -> Result<Vec<WallpaperRef>>;
    fn resolve(&self, r: &WallpaperRef) -> Result<Acquisition>;
}

trait WallpaperEngine {
    fn id(&self) -> &'static str;
    fn probe(&self) -> bool;                    // is it running?
    fn outputs(&self) -> Result<Vec<OutputInfo>>;
    fn set(&self, path: &Path, o: &SetOpts) -> Result<()>;
    fn restore(&self) -> Result<()>;
    fn clear(&self, color: &Rgb) -> Result<()>;
}
```

Sources answer *what should I show and where do the bytes come from*. Engines
answer *how does it appear on screen*. This is why you can swap awww for
hyprpaper without touching the catalog, and why a source can be added without
knowing which engine is active.

### 4.2 Acquisition modes and why there are three

| Mode | Who can do it | Mechanism |
|---|---|---|
| `Direct { url }` | Wallhaven | Official API returns a direct file URL. We stream it to a temp file. |
| `UrlPaste { url }` | Anyone | User pastes a direct image link. Same download path. |
| `BrowserHandoff { page }` | Any site | We open the page, user saves it, a watcher on `~/Downloads` imports it. |

A source declares which modes it supports. Wallhaven gets `Direct` +
`UrlPaste`. MoeWalls and similar get `BrowserHandoff` only.

This is not over-caution — MoeWalls' `robots.txt` contains `Disallow: /download*`.
Their listing pages are crawlable; their download endpoints are explicitly not.
A wallpaper manager that bulk-downloads from those endpoints is both fragile
(breaks when their theme changes) and a clear ToS violation. The handoff mode
gives you the site without the problem.

**But handoff alone does not make MoeWalls usable, and the plan previously got
this wrong.** MoeWalls is live-wallpaper oriented: MP4/WebM. awww cannot
display video at all (§4.4), so an imported `.mp4` lands in the catalog and
then has nothing to draw it. Handoff is the *compliant* acquisition mode, not a
*sufficient* one. Until we take on mpvpaper, the honest options are:

- drop site handoff entirely and ship Wallhaven + local + manual import, or
- keep handoff but have `isled` **reject video at import time** with a clear
  message, so the catalog never accumulates entries the engine cannot render.

The second is cheap and is what we should do by default. Do not ship a
handoff tab that silently imports undisplayable files.

Enforce the modes in the type system, not in review comments:

```rust
pub struct BoundedUrl { /* scheme+host allowlist, max bytes, timeout */ }
```

Plus content-type sniffing against actual image magic bytes, and path
confinement on every write. A misbehaving or compromised source must not be
turnable into an arbitrary downloader.

### 4.3 Wallhaven (verified against the live API)

Base `https://wallhaven.cc/api/v1`. **No API key required for SFW** — a key is
only needed for NSFW sections, which we will not implement.

```
GET /search?categories=100&purity=100&sorting=toplist&atleast=1920x1080&page=1
```

Response fields worth persisting: `id`, `url` (page), `short_url`, `path` (full
image), `thumbs.small` / `.large` / `.original`, `resolution`, `file_size`,
`file_type`, `colors[]`, `category`, `created_at`, `views`, `favorites`,
`source` (artist's page when applicable).

Two things this buys us:

- **`colors[]` — a free palette.** The API returns five hex colors per wallpaper
  *before you download anything*. That means the theme switcher can render a
  live theme preview for a wallpaper it hasn't fetched yet. No image decode, no
  matugen round-trip, instant response. For local wallpapers we fall back to
  running the palette extraction on the file.
- **`thumbs.small` — a cheap grid.** Sub-10 KB JPEGs for the picker. Only pull
  full-size when the user actually applies one.

Rate limit is 45 req/min unauthenticated. Practical guardrails: debounce search
input ≥350 ms, never poll, cap page size, and cache every query in SQLite with
a TTL so repeat visits are free.

Attribution: keep the artist page (`source`) and wallpaper page (`url`) in the
catalog and surface them in the panel. The API terms expect a link back.

### 4.4 Engines

**`awww` — the default, and the only real one.**

`awww` was renamed from `swww` on 2025-10-29 and shipped under the new name in
**0.12.0**. There is no separate `swww` project any more — the GitHub repo
`LGFae/swww` is archived and the live tree is **Codeberg** `LGFae/awww`. Any
reference to "swww as a fallback engine" is a bug: it is the same binary.
Current release **0.12.1**, present in `nixpkgs-unstable`.

It is a daemon plus a thin CLI socket client. We are the client, not the daemon.
Two binaries, both required: `awww` and `awww-daemon`.

```
awww img      [-o <outputs>] [--transition-type <t>] [--transition-step <n>]
              [--transition-fps <n>] [--resize <r>] [--crop-gravity <g>] <path>
awww clear    [<rrggbb[aa]>]
awww restore
awww query    [--json]
awww pause | unpause | toggle | kill
awww clear-cache
```

Socket: `$XDG_RUNTIME_DIR/${WAYLAND_DISPLAY}-awww-daemon[.<ns>].sock` —
a dot separator before the namespace, and a `.sock` suffix. Verified on
0.12.1 at `common/src/ipc/socket.rs:89`; **the daemon's own `--help` text says
`.socket` and is stale**, so trust the observed path, not the help.

Keep the two namespaces distinct: the socket *filename* uses `.<ns>`, while the
Wayland layer-shell namespace is `awww-daemon` concatenated with `ns` (so
`--namespace isle` gives a surface namespace of `awww-daemonisle`).

Daemon flags that matter: `--format argb|abgr|rgb|bgr`, `--layer background|bottom`,
`--namespace`, `--no-cache`.

- **Do not pass `--format`.** Omit it and let awww negotiate; it defaults to
  `argb`. The 3-channel formats save ~3/4 of the memory, but **they are not
  universally available** and forcing one can kill the daemon outright:
  measured on the target machine (niri 26.04, amdgpu, eDP-1 at scale 1.25),
  `--format bgr` and `--format rgb` both die at startup with
  `WAYLAND PROTOCOL ERROR: format Bgr888/Rgb888 not supported`, while the
  default, `argb` and `abgr` all work. Probe before assuming a saving is
  available, and prefer a working default over a 25% memory optimisation.
- **Animation quality is `--transition-step` + `--transition-fps`, not
  `--transition-duration`.** `--transition-duration` exists but is a silent
  no-op on the `simple` transition — which is the *default*. If you set a
  duration and nothing happens, you are on `simple`.
- **`restore` is client-side**, not a daemon operation: the client reads its own
  cache (last file per output) and re-issues `img`. It therefore depends on the
  file still existing, and re-caches animation frames. Since our catalog prunes
  by LRU (§4.6), `isled` must persist the current wallpaper itself and re-issue
  `img` at startup rather than delegating to `awww restore`.
- **No image is restored on monitor reconnect.** That was deliberately removed
  upstream. `isled` must re-issue `img` when `Compositor::outputs()` changes —
  which §6.2 already needs the adapter for, so this is free.
- Cache lives at `$XDG_CACHE_HOME/awww/$PKG_VERSION` (version-scoped since
  0.12.0), so a NixOS upgrade invalidates it. We own our own catalog, so this
  only costs a one-off re-cache.
- `awww query --json` exists — use it for the health check rather than parsing
  the human-readable output.
- `pause`/`unpause` (new in 0.12.1) are useful for the idle/low-power work in
  Phase 7.
- It will not run on GNOME, which does not implement `zwlr_layer_shell_v1`.
  Irrelevant here — both target compositors do.

**Supported image formats:** avif (behind a build feature nixpkgs does not
enable), jpeg, jpegxl (static), png, apng, gif, animated webp, tiff, tga,
pnm, bmp, farbfeld, svg (static).

**There is no video support and there never was.** No mpv backend, no
`awww vid`, no mp4/webm. Animation is image-sequence formats only. The upstream
README recommends **mpvpaper** as a separate tool for video wallpapers — see
below.

- **Use `--namespace isle`.** It isolates our daemon from any other awww
  instance the user might run, and namespaces the socket name too.

**Lifecycle policy**: we own a namespaced instance, started by
`awww-daemon.service` (§9.3). At startup `isled` probes for a default-namespace
awww daemon and adopts it if one answers, rather than starting a second one.
Record which path was taken in settings so behavior is predictable across
reboots.

**`mpvpaper` — the video engine, if we ever want one.** A separate daemon
backed by mpv, in nixpkgs, driven over its own socket. It is the only way to
display MP4/WebM wallpapers. It is *not* an awww backend and cannot be swapped in
as one; it would be a second `WallpaperEngine` impl behind the same trait,
selected per-wallpaper by `file_type`. Treat it as scope, not as a fallback.

**`hyprpaper` — Hyprland-only last resort.** Driven via `hyprctl hyprpaper`.
Present only so the Hyprland path has a fallback if awww is unavailable.

Auto-detection: probe in order, first available wins, user can override in
settings. If none is available, the wallpaper panel shows a clear "no engine"
state rather than silently doing nothing.

### 4.5 Catalog schema

```sql
wallpapers(
  id           TEXT PRIMARY KEY,   -- 'wallhaven:jel1jq' | 'local:<sha1>' | 'import:<uuid>'
  source       TEXT NOT NULL,      -- wallhaven | local | url | import
  remote_id    TEXT,               -- wallhaven id, NULL otherwise
  title        TEXT,
  artist       TEXT,
  page_url     TEXT,               -- attribution, open-in-browser
  file_path    TEXT NOT NULL,      -- path under the data dir
  thumb_path   TEXT,
  bytes        INTEGER,
  width        INTEGER,
  height       INTEGER,
  file_type    TEXT,               -- image/jpeg | image/png | image/gif | video/mp4
  palette      TEXT,               -- JSON array of hex colors
  tags         TEXT,
  favorites    INTEGER DEFAULT 0,
  use_count    INTEGER DEFAULT 0,
  last_used_at INTEGER,
  created_at   INTEGER
)
settings(key TEXT PRIMARY KEY, value TEXT)   -- current wallpaper, engine, per-source flags
```

### 4.6 Storage layout and pruning

```
~/.local/share/isled/wallpapers/    full-size downloads (LRU-pruned cache)
~/.local/share/isled/thumbs/       grid thumbnails (kept, small)
~/.local/share/isled/originals/    anything imported from local dirs (never auto-pruned)
```

Prune `wallpapers/` by LRU above a configurable cap (default 10 GB), skipping
anything favorited. `originals/` is user data and is never touched by the
pruner. Every prune is logged so a surprise deletion is diagnosable.

### 4.7 Theming tie-in

Applying a wallpaper is one user action with four consequences:

1. `engine.set(path)` — awww draws it
2. palette → `isled` → IPC push → UI repaints (instant; often free from `colors[]`)
3. optionally `matugen` writes Qt/GTK/terminal themes
4. `gsettings` color-scheme flips for light/dark

Because step 2 can use `colors[]`, the theme switcher can show a **live preview
of a wallpaper's theme before committing** — including for wallhaven results
that have never been downloaded. This is the cheapest "wow" feature in the
project and it falls out of the architecture rather than being bolted on.

### 4.8 Panel UI

One `theme` panel (the mock's keybind), three tabs: **Local** · **Wallhaven** ·
**Imported**.

- Grid of `thumbs.small` images — fast, no full downloads for browsing.
- Hover reveals title / artist / resolution. Click applies. Star to favorite.
- Trash icon on cached items only; originals can't be trashed.
- Debounced search field.
- Handoff tab shows a "waiting for save…" row per pending link that disappears
  the moment the `~/Downloads` watcher imports the file.
- If the engine is unreachable, the grid stays browsable and applying shows
  "saved, but wallpaper engine is down" — the catalog is still useful offline.

### 4.9 Sourcing policy (keep this auditable)

- **Wallhaven**: official API only. Never scrape HTML. Obey 45 req/min via
  debounce + cache + zero polling.
- **MoeWalls and other no-API sites**: browse/handoff only. No automated
  downloads of any kind, including via a scraper.
- **No NSFW paths.** `purity=100` unconditionally.
- **Attribution retained** and displayed: artist + source page.
- Write this policy down in `SOURCES.md` next to the code, so the constraint
  is visible to future maintainers rather than living in someone's head.

---

## 5. The morph engine

Two risks. Both must be resolved in Phase 0 before any panel work.

### 5.1 Animation approach

Two candidates, prototype both:

- **A — animate the real surface.** `Behavior on width/height` on the root
  `Rectangle`. Every frame re-configures the layer surface. Simple, but you may
  see jank and compositor-side commit throttling on large deltas.
- **B — fixed-size transparent surface, animate an inner rect.** Allocate the
  surface at the maximum expanded size with a transparent background; animate
  only the inner island rectangle. Zero re-configures, perfectly smooth.
  Cost: you must maintain `inputRegion` so clicks outside the visible island
  pass through to the desktop, and you need a click-away catcher.

Recommendation: try A first (much less code). Fall back to B if A janks.
Independent of compositor, so this is a one-time experiment.

Easing: Apple's island uses a spring. In QML, `SpringAnimation` or
`SmoothedAnimation` with a tuned `duration` + `type: Easing.OutQuart`. Keep one
shared easing token so all morphs feel identical.

**Quickshell constrains this.** `PanelWindow` has no `horizontalCenter` anchor
and no `above` property. The island is centred by anchoring the window
`top`/`left`/`right` (full-width surface) and centring the island item inside,
which means **the surface is full width and only the inner rectangle resizes** —
i.e. approach B is what falls out of the box. Getting a surface that actually
tracks the island means dropping the `left`/`right` anchors and positioning
manually, which is not obviously worth it. `Behavior on width`/`height` on the
inner rect still gives the morph; what is lost is the ability to compare A
against B on the surface-reconfigure cost, which is the thing A vs B was
supposed to measure. **This is the biggest unresolved Phase 0 question and it
needs eyes on the real thing.**

### 5.2 Focus lifecycle

The failure mode: the island asks for keyboard focus, the compositor grants it,
you collapse the island, and focus stays trapped. Both niri and hyprland need
*different* fixes and neither is obvious.

Test matrix, in this order:

1. Expand island on niri → type → collapse → keystrokes reach the terminal again.
2. Same on hyprland.
3. Rapid expand/collapse cycling on both (race check).
4. Expand while a fullscreen game is running (does the island appear at all?).
5. Both compositors: expand with mouse only, never touch the keyboard.

Quickshell's `FocusScope` handles the request side. The release-on-collapse
side is the part to get right. Note that niri's `WlSessionLock` keyboard
handling has known quirks if you extend this to a lock screen later.

### 5.3 Panel router

One state machine. **Nine views**, not eight — the earlier draft listed eight
here while the phases and risk table said nine, because todos had a full
editor built in Phase 5 but no router entry. Todos is a real panel (add,
complete, reorder is not a pill affordance), so it is a view:

```
1 context   2 media   3 notifications   4 clipboard   5 calendar
6 todos     7 notes   8 network         9 theme
```

- `compact` — collapsed pill, shows the highest-priority live activity
- `expanded` — has a `view`, one of the nine above
- Transitions: keybind, click on pill, click on module button, Esc to collapse.
  External IPC command can also force a view (used by the context engine and by
  the wallpaper apply action).

**Keybinds are now our choice** (§3.5) — no longer something to thread around
another shell's generated config. The mock's plain `1-7` is not viable: in both
niri and hyprland `Mod+1..9` is workspace switching. Proposal is `Mod+Alt+1..9`
for direct view selection plus `Mod+Alt+J` to toggle expand, all of which are
unbound by default. Confirm before Phase 1 wires them; it is open decision #8.

Morph on **view change only within expanded**, and **size change** on
compact↔expanded. Content cross-fades on opacity with a slight scale, staggered
~30 ms after the geometry starts moving — that delay is what makes it feel
like a physical object rather than a page transition.

---

## 6. The context engine

Feature #1 in the original list, and the least UI-shaped thing in the project.
Design it first; implement it last.

### 6.1 Pipeline

```
sources ──▶ normalized events ──▶ context reducer ──▶ score ──▶ suggested activity
                                                                    │
                                              live activity queue ◀──┘
```

Each source emits a normalized `Signal { kind, urgency, payload, ts }`. The
reducer folds signals into a `ContextState` (idle / busy / presenting /
meeting / low-battery / away). The scorer ranks candidate activities and emits
a suggestion. The UI's live-activity queue consumes suggestions with dwell
timing.

### 6.2 Compositor adapter

One trait, two impls, because the schemas are unrelated:

```rust
trait Compositor {
    fn focused_window(&self) -> Option<Window>;   // title, app_id, ws
    fn workspaces(&self) -> Vec<Workspace>;
    fn outputs(&self) -> Vec<Output>;             // name, scale, refresh
    fn subscribe(&self) -> Receiver<CompositorEvent>;
}
```

- `Hyprland`: `hyprctl -j`, or the `event/socket2` event stream for push rather
  than poll. No good Rust crate — shell out or use community `hyprland-rs`.
- `Niri`: `$NIRI_SOCKET` with `EventStream` request (added in niri 0.1.9).
  Push-based, sends full state up-front then deltas — no polling needed. Rust
  bindings exist (`niri-ipc`) but pin the **exact** version; it follows niri's
  version and adds fields in patch releases.

Detect at startup via `$HYPRLAND_INSTANCE_SIGNATURE` / `$NIRI_SOCKET`.

Bonus: `outputs()` from this adapter is exactly what the wallpaper engine
needs for per-monitor wallpaper placement, so wallpaper selection reuses it.

### 6.3 Signal sources, in priority order

| Signal | Source | Triggers |
|---|---|---|
| Meeting / presenting | PipeWire screencast nodes + window title | island expands, mic-cue |
| Focus change | compositor event stream | context panel highlights |
| Screen lock / unlock | logind `LockHint` | collapse island, mute previews |
| Battery critical | UPower | persistent low-battery activity |
| Fullscreen | compositor IPC | suppress non-urgent activities |
| Calendar event starting | your own calendar module | reminder activity |
| Long idle | logind `IdleHint` | break reminder |
| Network quality | `NetworkManager` | degraded-connection activity |
| Clipboard burst | clipboard watcher | paste-your-stuff activity |

### 6.4 Live activity priority

The compact pill shows exactly one thing. Collision rules:

```
media (playing) > notification (new) > timer > meeting > low battery
                > focus session > upcoming todo > default (clock / weather)
```

Each has a dwell time and a "sticky" flag (don't evict a music pill mid-track;
do evict a transient toast). Switching between activities animates the pill's
content with a vertical slide, not a cross-fade — that reads as "the pill
changed what it's saying" rather than "a new window opened".

---

## 7. Phases

Each phase has an exit criterion. Do not start a phase before the previous one
meets its criterion.

### Phase 0 — Risk retirement (1-2 days)
- **The authoring environment is already the target.** NixOS 26.11, niri 26.04
  and hyprland all installed, `systemctl --user` running, Quickshell live. There
  is no Fedora/GNOME staging box and no cross-machine deployment step. An
  earlier draft of this plan assumed one and it is simply wrong.
- The system flake (`/home/reze/Michi`) is currently pinned to a Ryoku-supplied
  nixpkgs rev purely to keep Ryoku's glibc and mesa matched. **Once Ryoku is
  gone that pin goes away** and the system tracks `nixos-unstable` like
  everything else. This project's own flake already tracks unstable.
- Confirm `isled` builds as a Nix package and `isled.service` starts at login.
- Confirm `awww-daemon` runs and that we can reach its namespaced socket.
  `awww` is in `nixpkgs-unstable` at 0.12.1; it is not installed system-wide yet,
  so the devshell carries it.
- **Probe the display edge cases early.** The test machine is a single
  `eDP-1` at 1920x1080@144Hz with **fractional scale 1.25**, on the amdgpu iGPU
  with an NVIDIA dGPU present. Fractional scaling is where aww's bug history is
  worst (0.9.4, 0.10.2 were both scaling fixes), and multi-output placement
  paths in this plan will have no local test surface at all.
- Both morph approaches (A and B), timed by eye.
- The full §5.2 focus matrix on both compositors.
- Scaffold the repo, `isled` skeleton, IPC handshake, one hello-world event.

**Exit:** a pill that morphs smoothly and never steals focus, on both.

### Phase 1 — Skeleton (2-3 days)
- `PanelWindow` with anchors/exclusive-zone settled.
- Panel router, all 9 views as empty styled containers.
- Theme tokens (color, radius, spacing, easing) as a single QML singleton.
- Packaging: flake + Home Manager module per §9 — `programs.quickshell`,
  `isled.service`, `awww-daemon`, and the shared `isle.target`.
- Keybinds in **both** composulators, both routed through `isled call` (§9.4).

**Exit:** all views reachable by keybind and by click; identical on both.

### Phase 2 — Live activity core (2-3 days)
- Priority queue + dwell rules + slide transition.
- Notification daemon replacement; notification arrives as a pill activity.
  Shipped: `org.freedesktop.Notifications` via zbus with opt-in claiming (§3.6),
  plus the notifications panel (view 3) reading live events. Still owed: the
  pill-activity path, where an arriving notification grows the compact pill,
  and `org.kde.StatusNotifierWatcher` (§3.5).
- MPRIS media → pill activity, full media panel.
- Basic calendar of *events* (read-only) — no editing yet.

**Exit:** music + notifications both drive the pill correctly, including
collision and interruption behaviour.

### Phase 3 — Control center (3-4 days)
- Control center panel: network, BT, volume in/out, brightness, dark mode.
- Wallpaper engine abstraction + awww integration + hyprpaper fallback.
- Local wallpaper switcher over the catalog.
- Download-dir watcher for imports.
- matugen tie-in; persist choices; apply to Qt + GTK + terminals.

**Exit:** every toggle works, survives a restart, and wallpaper changes apply
on both compositors.

### Phase 4 — Remote wallpaper sources (3-4 days)
- Wallhaven adapter: search, thumbnails, pagination, cache with TTL.
- Debounced search + 45 req/min compliance.
- Free palette from `colors[]`; live theme preview before download.
- Attribution display; LRU pruning.
- Browser-handoff adapters (MoeWalls and friends) + pending-link UI.

**Exit:** browsing, searching, applying and pruning all work; no request
budget violations; the catalog survives restart.

### Phase 5 — Data (4-5 days)
- Clipboard manager (watch, dedupe, pin, search, paste-back).
- Quick notes with autosave.
- Calendar full: month grid, agenda, ICS source.
- Todos: list, add/complete/reorder, surfacing in the pill.

**Exit:** all four persist across restart; no data loss on crash.

### Phase 6 — Context (4-6 days)
- Compositor adapter (both impls) with the shared trait.
- logind, UPower, PipeWire, clipboard-burst signals.
- Reducer + scorer.
- Default expanded panel: weather, CPU, memory, workspaces, plus whatever the
  context engine suggests.

**Exit:** lock the screen, join a call, unplug the charger — the island behaves
sensibly in each case on both compositors.

### Phase 7 — Polish (ongoing)
- Idle/low-power behavior, animation tuning, a11y, fallbacks for missing
  services (no NM, no BT daemon, no MPRIS player, no wallpaper engine).

**Totals: ~4-5.5 weeks** for Phases 0-6. Phases 0-3 alone (~2 weeks) give you a
genuinely usable island.

---

## 8. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Layer-shell keyboard focus differs per compositor | High | Phase 0 focus matrix; it's the first thing tested |
| Morph animation janks on large size changes | Medium | Prototype A vs B in Phase 0 |
| QML GC pauses on low-end GPUs | Medium | Avoid per-frame allocations in animation paths; profile with `QSG_RENDER_LOOP=basic` |
| Two compositors drift apart | Medium | Adapter trait + a test harness that runs the same assertions against both |
| `niri-ipc` breaks on niri upgrade | Low | Pin exact version; adapter returns partial data rather than failing |
| Notification daemon replaces mako/dunst | Medium | Keep it opt-in until stable; other tools will notice the portal |
| awww unavailable, or behaves differently across versions | Medium | `mpvpaper`/`hyprpaper` impls behind the same trait; pin the awww version you test against |
| Handoff imports files the engine cannot render | Medium | Reject video at import time (§4.2); never let the catalog accumulate undisplayable entries |
| Removing Ryoku breaks a bus name mid-session | Medium | Claim `org.freedesktop.Notifications` and `org.kde.StatusNotifierWatcher` before removing it, not after (§3.5) |
| Single scaled laptop panel gives no multi-output test surface | Medium | Per-output wallpaper placement must be written defensively and tested on a second machine or a nested session |
| Remote wallpaper sources break or violate ToS | Medium | API-only for Wallhaven, handoff-only elsewhere, modes enforced in the type system, `SOURCES.md` |
| Disk growth from wallpaper cache | Low | LRU prune with cap; favorites and originals exempt |
| Scope creep — 9 panels + wallpaper system + porting Ryoku's session | High | Phases are independently shippable; "island done" and "Ryoku gone" are separate milestones (§3.5) |

---

## 9. NixOS packaging

**Target: NixOS**, with niri and hyprland both installed on the same machine and
selected per session. This *is* the authoring machine — see §7 Phase 0; there
is no separate staging box.

### 9.1 Package availability (verified against `nixos-unstable`, 2026-10-14)

| Package | Version | Notes |
|---|---|---|
| `quickshell` | 0.3.2 | installed system-wide at **0.3.0**; devshell has 0.3.2 — see §9.6 |
| `niri` | 26.04 | |
| `hyprland` | 0.56.2 | |
| `awww` | **0.12.1** | packaged — no source build needed |
| `matugen` | 4.2.0 | |
| `wl-clipboard`, `wtype` | 2.3.0, 0.4 | |
| `brightnessctl` | 0.5.1 | |
| `cava`, `cliphist` | 1.0.0, 0.7.0 | optional |
| `khal`, `vdirsyncer` | 0.14.1, — | optional, calendar |
| `mpvpaper` | present | only if video wallpapers are ever wanted (§4.4) |

There is no `swww` row because `swww` no longer exists as a separate package
(§4.4). `nix search nixpkgs awww` → 0.12.1.

`awww` being packaged is the important one — it removes the main build risk from
the wallpaper subsystem.

### 9.2 Repo layout

```
dynamic-island-shell/
  flake.nix              plain path: flake, consumed by the target's flake
  nix/
    module.nix           HM module: programs.quickshell + isled + isle.target
    isled-package.nix    rustPlatform.buildRustPackage for the daemon
  core/
    isled/               daemon binary
    isle-core/           lib: sources, state, scoring, persistence, wallpaper
  ui/
    shell.qml            root, PanelWindow
    Island.qml           morph engine
    panels/              one file per panel
    components/          Pill, Card, Icon, SegmentedControl, ...
```

Keep this a plain `path:` flake that the target imports, rather than a flake
input with `follows`. Self-referential flakes add a circular-input headache for
no benefit at this size.

### 9.3 Home Manager wiring

**Home Manager's systemd user-unit options have changed shape.** The classic
`wantedBy` / `runtimeDirectory` / `serviceConfig` sugar on
`systemd.user.services.<name>` no longer type-checks on HM master: those options
are now freeform submodules typed as systemd *sections*, and
`systemd.user.units` is gone entirely. Express units as sections instead:

```nix
systemd.user.services.isled = {
  Unit = { Description = "..."; After = [ "graphical-session-pre.target" ]; };
  Service = { ExecStart = lib.getExe cfg.package; RuntimeDirectory = "isled"; };
  Install = { WantedBy = [ "isle.target" ]; };
};
```

This is why `nix/module.nix` does not use `programs.quickshell.systemd` at all —
it writes its own `quickshell.service` so that ordering against `isled.service`
is ours to control, and so the module does not depend on an interface that is
actively churning. `programs.quickshell`'s `configs`/`activeConfig` options are
still fine for deploying the UI; only its systemd integration is bypassed.

**The dual-compositor problem.** You have two compositors and one session at a
time. A custom `isle.target` with
`WantedBy = ["niri-session.target" "hyprland-session.target"]` is the right
answer, and now it is also the simple one rather than a workaround.

Units defined by our own module:

- **`isle.target`** — pulls in everything below, for both session targets.
- **`isled.service`** — `After`/`Wants graphical-session-pre.target`,
  `Restart = "on-failure"`, `RestartSec = 2`,
  `RuntimeDirectory = "isled"` so the socket is `%t/isled/isled.sock`.
  SIGTERM is handled and removes the socket on the way out (see §3.2).
- **`quickshell.service`** — `Requires = ["isled.service"]` and
  `After = ["isled.service"]`. **The UI must never run without the core.**
- **`awww-daemon.service`** — `awww-daemon --namespace isle --layer background`,
  **no `--format`** (§4.4). Settles §4.4's lifecycle question in favour of "we
  own it"; the adopt-an-existing-daemon path remains as a startup probe.

**Environment forwarding is unproven and must be verified on first boot.** The
units use `ImportEnvironment` for `WAYLAND_DISPLAY`, `NIRI_SOCKET`,
`HYPRLAND_INSTANCE_SIGNATURE` and friends. If the systemd user manager does not
already have them, the compositor adapter falls back to `Unknown` and the UI
shows `compositor=unknown`. Check with
`systemctl --user show isled -p Environment` before assuming it works.

`isled` itself: `rustPlatform.buildRustPackage` with
`cargoLock.lockFile = ./Cargo.lock`. One binary, no codegen, no crate2nix yet.

### 9.4 Keybinds go through `isled`, not the UI

Both compositors' configs bind to `isled call <topic>` — never directly to the
UI or the socket.

- Hyprland: `programs.hyprland.settings.bind`
- Niri: `keybinds` in the niri config / flake module

`isled` then pushes a view change to whichever UI is connected. Three wins for a
little indirection: one keybind definition serves both compositors, the same
commands work from a terminal or script, and the compositor never needs to know
the UI's protocol.

Since Ryoku is going away (§3.5) we own these config files outright. That is a
strict improvement over the alternative, which was to add lines to a
`user.kdl` that loads after another shell's generated `rebinds.kdl` and would
lose any edit made to that generated file by hand. The specific scheme is open
decision #8 — plain `Mod+1..9` is already taken by workspace switching in both
compositors.

### 9.5 Nix gotchas to expect

- `QT_QPA_PLATFORM=wayland` on the quickshell unit.
- Don't mix a distro Qt with the nixpkgs `quickshell`; plugin paths will fight you.
- systemd **user** services need `WAYLAND_DISPLAY`. Ordering after the WM target
  plus `graphical-session-pre.target` covers it; don't hardcode the display name.
- **Blur: niri has it, the plan was wrong to say otherwise.** niri 26.04
  implements `ext-background-effect-v1` with blur, xray, saturation and noise —
  it landed in exactly this release. Blur can be forced from the compositor side
  with a `layer-rule { background-effect { blur true } }`, which works on any
  layer surface regardless of client support.
  **The blocker is Quickshell, not niri.** As of 0.3.2 Quickshell has a private
  `_BackgroundEffect` C++ type but exports **no** QML-facing blur type, and
  `PanelWindow` has no `namespace` property, so every Quickshell layer surface
  is `namespace="quickshell"` and cannot be targeted separately from Ryoku's.
  Revisit when Quickshell exposes it, or fall back to a `layer="top"` rule that
  blurs every top-layer surface.
- awww needs a live Wayland display; same ordering requirement as quickshell.

### 9.6 Pinning

**Track `nixos-unstable`; do not pin.** An earlier draft recommended pinning a
commit because niri moves fast and Quickshell's QML API churns. That advice is
withdrawn: the goal is to depend only on what `nixos-unstable` carries, and a
pin is just a version of Ryoku's problem — it goes stale and forces a
coordinated bump of unrelated things.

The real risk is narrower than "unstable breaks": it is **Quickshell's QML API
changing between minor versions**. Handle it by keeping the QML surface small
and by testing against the version actually running, not by freezing the world.

One live mismatch to be aware of: the devshell has Quickshell **0.3.2** while
the installed session runs **0.3.0**. `quickshell -c isle` in the devshell is a
different build from the one the compositor starts. This closes itself when the
system moves to unstable; until then, treat QML changes as suspect if the
devshell and the session disagree.

### 9.7 Before Phase 0, on the NixOS box

```sh
nix develop                                    # awww 0.12.1, quickshell, rustc
awww-daemon --namespace isle --layer background &
awww query --json -n isle                      # names + dimensions — our health check
niri msg --json outputs
hyprctl version
wlr-randr                                      # fractional scale on this box
quickshell --config isle                       # once the UI skeleton exists
```

Note the client and daemon must be given the **same** `--namespace`, and the
socket is `wayland-1-awww-daemon.isle.sock`. Do not background the daemon from
a shell that then exits — it dies with its parent. That is what
`awww-daemon.service` is for.

Then run the §5.2 focus matrix on both compositors.

---

## 10. Open decisions

1. **awww lifecycle** — confirmed as "we own a namespaced instance" in §9.3.
   Remaining question: should `isled` *also* adopt a user's pre-existing default
   awww daemon, or always use `--namespace isle` unconditionally? (§4.4)
2. **Should Phase 2 keep read-only calendar** (as written) or hold the whole
   calendar module for Phase 5?
3. **Calendar source**: own ICS parsing, `khal`, or `vdirsyncer` first?
4. **Todos format**: plain SQLite, or Taskwarrior-compatible export?
5. **Notification daemon**: we are replacing Ryoku's, so the question is
   narrower than "replace mako/dunst or run alongside" — it is whether to claim
   `org.freedesktop.Notifications` on day one or shadow Ryoku's until the cutover.
6. **Wallpaper cache cap** — 10 GB default, or tune to the target disk?
7. **Blur strategy.** niri *can* blur (§9.5) — the question is whether to wait for
   Quickshell to expose `BackgroundEffect`, force a `layer="top"` rule, or ship
   flat. Not a compositor capability question any more.
8. **Keybind scheme** — `Mod+Alt+1..9` + `Mod+Alt+J` proposed; `Mod+1..9` is
   taken by workspace switching in both compositors. (before Phase 1)
9. **Video wallpapers** — ship without them (awww cannot display video, §4.4),
   or take on `mpvpaper` as a second engine? The answer decides whether browser
   handoff is worth keeping at all. (before Phase 4)
