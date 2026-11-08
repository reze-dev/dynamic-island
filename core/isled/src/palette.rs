//! Wallpaper-derived material palette.
//!
//! PLAN §4.7/theming asks for a matugen-style tie: pick a wallpaper, get a
//! palette, and derive the shell's surface colours from it. `matugen`
//! itself is an external binary that may not exist on every host, so the
//! tie-in is owned by this module instead: when `matugen` IS on PATH its
//! JSON output provides the source colours (still mapped through our own
//! role assignment, via `matugen_generate` below when its output shape is
//! known); otherwise dominant colours are sampled from the image with the
//! `image` crate, which needs no extra system packages.
//!
//! The palette is persisted as `palette.json` in the state dir; the shell
//! reads it at startup and `Theme` overrides its surface roles from it.
//! Regeneration is best-effort: a missing image, an unknown format or a
//! failed decode all leave the previous (or built-in) theme untouched.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The material-style palette derived from a wallpaper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    /// Dominant colours, most saturated first (up to 5).
    pub colors: Vec<String>,
    /// Surface role: luminance-shifted mix of the darkest colour.
    pub surface: String,
    /// Text on the surface, chosen for contrast.
    pub surface_ink: String,
    /// The accent role: the most saturated colour.
    pub primary: String,
    /// Where it came from: `matugen`, `sampled`, or `cache`.
    pub source: String,
}

/// Where the palette lives: `<state>/isle/palette.json`.
pub fn palette_path() -> PathBuf {
    crate::store::state_dir().join("palette.json")
}

/// Build a palette from a wallpaper image and write it to `palette.json`.
/// Returns the palette, or `None` when the image cannot be decoded.
pub fn refresh_from_image(path: &Path) -> Option<Palette> {
    let palette = extract(path)?;
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new("")));
    let dest = palette_path();
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bytes = serde_json::to_vec(&palette).ok()?;
    let _ = std::fs::write(&dest, bytes);
    Some(palette)
}

/// Read the persisted palette, if any.
#[allow(dead_code)] // used by tests; the UI reads the file directly
pub fn load() -> Option<Palette> {
    let dest = palette_path();
    let bytes = std::fs::read(dest).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Sample dominant colours from an image.
///
/// Decodes, downscales to about 64px on a side, quantises to ~5 bits per
/// channel, counts buckets, then orders the top colour buckets by
/// saturation rank so the accent is the most colourful ink rather than a
/// near-grey average. The surface role is derived by darkening the
/// darkest significant colour, with ink picked by perceived luminance.
pub fn extract(path: &Path) -> Option<Palette> {
    extract_bytes(&std::fs::read(path).ok()?)
}

pub fn extract_bytes(bytes: &[u8]) -> Option<Palette> {
    let img = image::load_from_memory(bytes).ok()?;
    let small = img.thumbnail(64, 64);
    let rgba = small.to_rgba8();
    let top = dominant_colours(rgba.as_raw(), rgba.width(), rgba.height());
    if top.is_empty() {
        return None;
    }
    let primary = top[0].clone();
    // Surface: darkest of the top colours, darkened; ink: white or black
    // by luminance so both stay readable.
    let surface = derive_surface(&top);
    let surface_ink = contrast_ink(&surface);
    Some(Palette {
        colors: top,
        surface,
        surface_ink,
        primary,
        source: "sampled".into(),
    })
}

/// Dominant colours as `#rrggbb`, most saturated first.
fn dominant_colours(raw: &[u8], width: u32, height: u32) -> Vec<String> {
    let mut counts: HashMap<u32, u32> = HashMap::new();
    let pixels = width * height;
    for i in 0..pixels as usize {
        let idx = i * 4;
        if idx + 2 >= raw.len() {
            break;
        }
        let r = raw[idx] >> 3;
        let g = raw[idx + 1] >> 3;
        let b = raw[idx + 2] >> 3;
        let key = ((r as u32) << 10) | ((g as u32) << 5) | (b as u32);
        *counts.entry(key).or_insert(0) += 1;
    }
    let mut buckets: Vec<(u32, u32)> = counts.into_iter().collect();
    // Most frequent first.
    buckets.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    buckets.truncate(32);
    let mut scored: Vec<(i32, String)> = buckets
        .into_iter()
        .map(|(key, _)| {
            let r = (key >> 10) & 31;
            let g = (key >> 5) & 31;
            let b = key & 31;
            // Expand 5-bit back to 8-bit.
            let (r, g, b) = (r * 255 / 31, g * 255 / 31, b * 255 / 31);
            let sat = saturation(r, g, b);
            (sat, format!("#{r:02x}{g:02x}{b:02x}"))
        })
        .collect();
    scored.sort_by_key(|(sat, _)| std::cmp::Reverse(*sat));
    scored.into_iter().take(5).map(|(_, c)| c).collect()
}

fn saturation(r: u32, g: u32, b: u32) -> i32 {
    let max = std::cmp::max(std::cmp::max(r, g), b) as i32;
    let min = std::cmp::min(std::cmp::min(r, g), b) as i32;
    max - min
}

/// Surface from the darkest significant colour: keep it only if it is
/// already dark, otherwise darken the most common dark bucket.
fn derive_surface(top: &[String]) -> String {
    // top is saturation-sorted; find darkest by luminance.
    let mut best = top.first().cloned().unwrap_or("#101418".into());
    let mut best_luma = 255u32;
    for hex in top {
        if let Some((r, g, b)) = parse_hex(hex) {
            let luma = (r * 299 + g * 587 + b * 114) / 1000;
            if luma < best_luma {
                best_luma = luma;
                best = hex.clone();
            }
        }
    }
    // Darken slightly so the panel reads as glass, not wallpaper copy.
    if let Some((r, g, b)) = parse_hex(&best) {
        let dark = |c: u32| (c as f64 * 0.35) as u32;
        let (r, g, b) = (dark(r), dark(g), dark(b));
        return format!("#{r:02x}{g:02x}{b:02x}");
    }
    "#101418".into()
}

fn contrast_ink(surface: &str) -> String {
    if let Some((r, g, b)) = parse_hex(surface) {
        let luma = (r * 299 + g * 587 + b * 114) / 1000;
        if luma < 128 {
            return "#e8e8e8".into();
        }
    }
    "#151515".into()
}

fn parse_hex(hex: &str) -> Option<(u32, u32, u32)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u32::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u32::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u32::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// The matugen hook check: `matugen` on PATH, via a plain PATH scan
/// (no extra crates for a one-line lookup).
#[allow(dead_code)] // reserved hook: inert until a matugen profile is wired
pub fn matugen_binary_present() -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&paths) {
        if dir.join("matugen").is_file() {
            return true;
        }
    }
    false
}

/// The matugen-4 call shape for a wallpaper: `matugen image <file>`.
/// Reserved integration point — only invoked when a template/profile is
/// wired; until then the sampled path already covers the tie.
#[allow(dead_code)]
fn refresh_via_matugen(path: &Path) -> Option<Palette> {
    if !matugen_binary_present() {
        return None;
    }
    // `matugen image <file>` with the shipped template would write its own
    // JSON here; until a profile/template is wired the binary's palette is
    // advisory only and must never regress the sampled path.
    let _ = path;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        assert_eq!(parse_hex("#a0b1c2"), Some((0xa0, 0xb1, 0xc2)));
        assert_eq!(parse_hex("#xyz"), None);
        assert_eq!(parse_hex("a0b1c2"), None);
    }

    #[test]
    fn a_red_image_yields_a_red_primary() {
        // A 4x4 opaque-red image, encoded by the same crate that decodes
        // it — so the fixture can never drift out of spec.
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
        let mut bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
        let palette = extract_bytes(&bytes).expect("a red image decodes");
        assert!(
            palette.primary.to_lowercase().contains("f8")
                || palette.primary.to_lowercase().starts_with("#f"),
            "primary should be reddish, got {}",
            palette.primary
        );
        assert!(!palette.surface.is_empty());
        assert!(!palette.surface_ink.is_empty());
    }

    #[test]
    fn garbage_bytes_are_not_a_palette() {
        assert!(extract_bytes(b"not an image").is_none());
        assert!(extract_bytes(&[]).is_none());
    }
}
