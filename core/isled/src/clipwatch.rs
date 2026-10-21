//! Watching the Wayland clipboard.
//!
//! The clipboard history is only useful if it fills itself up. `wl-paste
//! --watch` runs a command each time the selection changes, and hands it the
//! selection on stdin — but it gives no sign of where one change ends and
//! the next begins, so a multi-line selection arrives as several lines and
//! the boundaries are lost.
//!
//! The command this runs puts each selection in a file of its own and
//! announces the file's name afterwards. The name is only printed once the
//! content is on disk, so the reader knows the file is complete before it
//! reads it, and multi-line selections survive intact.
//!
//! `wl-paste` is not always installed, and there is no way to watch the
//! clipboard without it. The watcher is therefore optional: without it the
//! history still works, it just has to be filled by hand.

use crate::wallpaper::which;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// How long to wait before restarting a watcher that stopped.
const RESTART_DELAY: Duration = Duration::from_secs(2);

/// The script `wl-paste --watch` runs for each change.
///
/// The directory is baked in: `wl-paste` takes a single command and no
/// arguments, so there is nowhere to pass one.
fn hook_script(dir: &Path) -> String {
    format!(
        r#"#!/bin/sh
# The clipboard arrives on stdin. It is written to a file whose name is
# announced afterwards, so the reader knows the file is complete before it
# reads it.
f=$(mktemp "{dir}/clip.XXXXXXXXXX") || exit 1
cat > "$f" || {{ rm -f "$f"; exit 1; }}
echo "$f"
"#,
        dir = dir.display()
    )
}

/// The directory the watcher works in.
pub fn watch_dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("isled").join("clipwatch")
}

/// Start watching the clipboard.
///
/// Returns the channel the selections arrive on, or `None` if there is no
/// way to watch: `wl-paste` is not installed.
pub fn spawn(dir: PathBuf) -> Option<mpsc::UnboundedReceiver<String>> {
    if which("wl-paste").is_none() {
        tracing::info!("wl-paste is not installed; the clipboard will not be watched");
        return None;
    }

    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            if let Err(err) = run_once(&dir, &tx).await {
                tracing::warn!(%err, "the clipboard watcher stopped");
            }
            tokio::time::sleep(RESTART_DELAY).await;
        }
    });
    Some(rx)
}

/// Watch until the process exits. Called again if it does.
async fn run_once(dir: &Path, tx: &mpsc::UnboundedSender<String>) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;

    let script = dir.join("hook.sh");
    tokio::fs::write(&script, hook_script(dir)).await?;
    make_executable(&script)?;

    let mut child = Command::new("wl-paste")
        // The order matters: `--watch` has to come last, or the argument
        // parser takes the command as a stray argument.
        .args(["--type", "text", "--watch"])
        .arg(&script)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("the clipboard watcher has no stdout"))?;

    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        let path = PathBuf::from(line.trim());
        // Only ever read a file this watcher made.
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("clip.") {
            continue;
        }

        let Ok(bytes) = tokio::fs::read(&path).await else {
            continue;
        };
        let _ = tokio::fs::remove_file(&path).await;

        if let Ok(text) = String::from_utf8(bytes) {
            if tx.send(text).is_err() {
                break;
            }
        }
    }

    let _ = child.wait().await;
    Ok(())
}

fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_announces_the_file_after_writing_it() {
        let script = hook_script(Path::new("/tmp/watch"));
        let write = script.find("cat >").expect("the script writes the file");
        let announce = script.find("echo \"$f\"").expect("the script announces it");
        assert!(
            write < announce,
            "the file has to be complete before it is named"
        );
    }

    #[test]
    fn the_script_writes_into_the_watch_directory() {
        let script = hook_script(Path::new("/tmp/watch"));
        assert!(script.contains("/tmp/watch/clip."));
    }

    #[test]
    fn the_script_cleans_up_after_itself() {
        let script = hook_script(Path::new("/tmp/watch"));
        assert!(script.contains("rm -f"));
    }
}
