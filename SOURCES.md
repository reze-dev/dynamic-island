# Wallpaper Sourcing Policy

This file records the sourcing constraints for the Dynamic Island Shell's
wallpaper catalog. It exists so the constraints are visible to future
maintainers, not hidden in code comments or someone's head.

## Policy

- **Wallhaven:** Official API only (`https://wallhaven.cc/api/v1`). No HTML
  scraping. No automated download of download endpoints that are disallowed
  by `robots.txt`. Rate limit: 45 req/min unauthenticated; the client paces
  searches at ~1.3s regardless. SFW (`purity=100`) is the default. Sketchy
  (`110`) and NSFW (`111`) are explicit opt-ins, never defaults: they need
  a Wallhaven API key (from `wallhaven.cc/settings`, stored in the local
  settings file) and are refused client-side without one. The key is sent
  only to `wallhaven.cc` and is never placed in the search cache key or
  logs.
- **MoeWalls and similar no-API sites:** Browser handoff only (`BrowserHandoff`
  acquisition mode). No automated downloads, including via scrapers. Their
  `robots.txt` explicitly disallows `/download*`. Listing pages are allowed.
- **Video files (MP4/WebM):** Rejected at import time. `awww` does not support
  video; the engine cannot render them. Handoff for video sources is not
  sufficient — it produces undisplayable catalog entries. Handled by rejecting
  video at import with a clear message.
- **NSFW paths:** Explicit opt-in only, key-gated (above). The default tier
  is SFW in both the daemon default and the panel, and an unknown tier is
  rejected, never guessed.
- **Attribution retained:** Artist (`source`) and wallpaper page (`url`) kept
  in the catalog and surfaced in the theme panel.
- **BoundedUrl:** All remote URLs must pass scheme+host allowlist, max bytes,
  and timeout checks. Content-type sniffed against magic bytes before any
  write. Path confinement enforced on every write.

See `PLAN.md` §4.2 and §4.9 for the technical design behind these constraints.
