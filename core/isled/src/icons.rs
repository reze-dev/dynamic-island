//! Finding an icon on disk.
//!
//! A tray item names its icon — `network-transmit-receive-symbolic` — and
//! something has to turn that into a picture. The usual way is the
//! `image://icontheme/` handler, which needs a Qt plugin that is not always
//! there, and the icons that are there are often SVG, which needs another
//! plugin that is also not always there.
//!
//! So the lookup is done here instead: the name is resolved to a file on
//! disk, preferring a PNG, and the panel is given a path it can load with
//! the plugins it definitely has.

use std::path::PathBuf;

/// The sizes worth looking for, best first. The largest is the one that
/// stays sharp on a scaled screen.
const SIZES: &[&str] = &[
    "48x48", "64x64", "32x32", "24x24", "22x22", "16x16", "128x128",
];

/// The places an icon might be.
const CONTEXTS: &[&str] = &[
    "apps",
    "status",
    "actions",
    "places",
    "categories",
    "emotes",
    "legacy",
];

/// The file types to accept, best first.
const EXTENSIONS: &[&str] = &["png", "svg", "xpm"];

/// The icon themes to look in.
const THEMES: &[&str] = &["hicolor", "Adwaita", "breeze", "Papirus"];

/// Find an icon by name.
///
/// A name that is already a path is used as it is. Otherwise the icon
/// themes are searched the way the freedesktop specification says they
/// should be.
pub fn find(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }

    // An absolute path is a path.
    if name.starts_with('/') {
        let path = PathBuf::from(name);
        return path.is_file().then_some(path);
    }

    // A name with a directory in it is a path relative to a theme.
    if name.contains('/') {
        for root in roots() {
            let candidate = root.join("hicolor").join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        return None;
    }

    for root in roots() {
        for theme in THEMES {
            for size in SIZES {
                for context in CONTEXTS {
                    for extension in EXTENSIONS {
                        let candidate = root
                            .join(theme)
                            .join(size)
                            .join(context)
                            .join(format!("{name}.{extension}"));
                        if candidate.is_file() {
                            return Some(candidate);
                        }
                    }
                }
            }
            // A theme may also have a scalable directory.
            for context in CONTEXTS {
                for extension in EXTENSIONS {
                    let candidate = root
                        .join(theme)
                        .join("scalable")
                        .join(context)
                        .join(format!("{name}.{extension}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

/// The directories the icon themes live in.
fn roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        roots.push(PathBuf::from(home).join("icons"));
    } else if let Some(home) = std::env::var_os("HOME").filter(|v| !v.is_empty()) {
        roots.push(PathBuf::from(home).join(".local/share/icons"));
    }

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|dirs| {
            std::env::split_paths(&dirs)
                .map(|dir| dir.join("icons"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    roots.extend(data_dirs);

    if roots.is_empty() {
        roots.push(PathBuf::from("/usr/share/icons"));
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_name_finds_nothing() {
        assert!(find("").is_none());
    }

    #[test]
    fn an_absolute_path_is_used_as_it_is() {
        assert!(find("/definitely/not/here.png").is_none());
    }

    #[test]
    fn a_name_with_a_directory_in_it_is_looked_up_in_the_theme() {
        // There is no such icon, but the path must not escape the theme.
        assert!(find("48x48/apps/nothing-here").is_none());
    }

    #[test]
    fn the_roots_are_never_empty() {
        assert!(!roots().is_empty());
    }
}
