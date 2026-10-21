use std::path::PathBuf;

pub const SOCKET_ENV: &str = "ISLED_SOCKET";

pub fn socket_path() -> PathBuf {
    resolve_socket_path(|key| std::env::var_os(key))
}

fn resolve_socket_path(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> PathBuf {
    if let Some(explicit) = env(SOCKET_ENV).filter(|v| !v.is_empty()) {
        return PathBuf::from(explicit);
    }
    let runtime = env("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from);
    runtime.join("isled").join("isled.sock")
}

pub const CLAIM_NOTIFICATIONS_ENV: &str = "ISLED_CLAIM_NOTIFICATIONS";
pub const CLAIM_TRAY_ENV: &str = "ISLED_CLAIM_TRAY";

/// What the daemon should take over on the session bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Claims {
    /// Own `org.freedesktop.Notifications`.
    pub notifications: bool,
    /// Own `org.kde.StatusNotifierWatcher`, which is what a system tray
    /// host is.
    pub tray: bool,
}

pub enum Args {
    Serve(Claims),
    Call { name: String, raw: String },
    SocketPath(Claims),
    Help,
    Unknown(String),
}

pub fn env_wants(key: &str) -> bool {
    matches!(std::env::var(key), Ok(value) if matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    ))
}

pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Args {
    let mut args = args.into_iter().peekable();
    let mut claims = Claims {
        notifications: env_wants(CLAIM_NOTIFICATIONS_ENV),
        tray: env_wants(CLAIM_TRAY_ENV),
    };
    while let Some(flag) = args.peek().map(|s| s.as_str()) {
        match flag {
            "--claim-notifications" => {
                claims.notifications = true;
                args.next();
            }
            "--no-claim-notifications" => {
                claims.notifications = false;
                args.next();
            }
            "--claim-tray" => {
                claims.tray = true;
                args.next();
            }
            "--no-claim-tray" => {
                claims.tray = false;
                args.next();
            }
            _ => break,
        }
    }
    match args.next().as_deref() {
        Some("call") => {
            let name = args.next().unwrap_or_default();
            let raw = args.next().unwrap_or_default();
            Args::Call { name, raw }
        }
        Some("socket-path") => Args::SocketPath(claims),
        Some("--help" | "-h") => Args::Help,
        None => Args::Serve(claims),
        Some(other) => Args::Unknown(other.to_owned()),
    }
}

pub fn parse_args_json(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn env_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
        let owned: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), OsString::from(*v)))
            .collect();
        move |key: &str| owned.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn explicit_socket_env_var_wins() {
        let path = resolve_socket_path(env_from(&[
            (SOCKET_ENV, "/run/isled-test.sock"),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ]));
        assert_eq!(path, PathBuf::from("/run/isled-test.sock"));
    }

    #[test]
    fn an_empty_socket_env_var_is_ignored() {
        let path = resolve_socket_path(env_from(&[
            (SOCKET_ENV, ""),
            ("XDG_RUNTIME_DIR", "/run/user/1000"),
        ]));
        assert_eq!(path, PathBuf::from("/run/user/1000/isled/isled.sock"));
    }

    #[test]
    fn socket_lands_under_the_systemd_runtime_directory() {
        let path = resolve_socket_path(env_from(&[("XDG_RUNTIME_DIR", "/run/user/1000")]));
        assert_eq!(path, PathBuf::from("/run/user/1000/isled/isled.sock"));
    }

    #[test]
    fn missing_runtime_dir_falls_back_to_tmp() {
        assert_eq!(
            resolve_socket_path(env_from(&[])),
            PathBuf::from("/tmp/isled/isled.sock")
        );
    }

    #[test]
    fn call_takes_a_name_and_optional_raw_args() {
        let args = |v: &[&str]| parse_args(v.iter().map(|s| (*s).to_owned()));
        assert!(
            matches!(args(&["call"]), Args::Call { ref name, ref raw } if name.is_empty() && raw.is_empty())
        );
        assert!(matches!(
            args(&["call", "volume.set", r#"{"level":0.4}"#]),
            Args::Call { ref name, ref raw } if name == "volume.set" && raw == r#"{"level":0.4}"#
        ));
    }

    #[test]
    fn bare_invocation_serves_and_help_recognises_flags() {
        let args = |v: &[&str]| parse_args(v.iter().map(|s| (*s).to_owned()));
        assert!(matches!(
            args(&[]),
            Args::Serve(Claims {
                notifications: false,
                tray: false
            })
        ));
        assert!(matches!(args(&["--help"]), Args::Help));
        assert!(matches!(args(&["-h"]), Args::Help));
        assert!(matches!(args(&["nonsense"]), Args::Unknown(ref s) if s == "nonsense"));
    }

    #[test]
    fn claiming_the_bus_names_is_opt_in_by_flag() {
        let args = |v: &[&str]| parse_args(v.iter().map(|s| (*s).to_owned()));
        assert!(matches!(
            args(&["--claim-notifications"]),
            Args::Serve(Claims {
                notifications: true,
                tray: false
            })
        ));
        assert!(matches!(
            args(&["--no-claim-notifications"]),
            Args::Serve(Claims {
                notifications: false,
                tray: false
            })
        ));
        assert!(matches!(
            args(&["--claim-tray"]),
            Args::Serve(Claims {
                notifications: false,
                tray: true
            })
        ));
        assert!(matches!(
            args(&["--claim-notifications", "--claim-tray"]),
            Args::Serve(Claims {
                notifications: true,
                tray: true
            })
        ));
    }

    #[test]
    fn the_flag_is_stripped_before_finding_the_subcommand() {
        let args = |v: &[&str]| parse_args(v.iter().map(|s| (*s).to_owned()));
        assert!(matches!(
            args(&["--claim-notifications", "socket-path"]),
            Args::SocketPath(Claims {
                notifications: true,
                tray: false
            })
        ));
        assert!(matches!(
            args(&["call", "ping"]),
            Args::Call { ref name, .. } if name == "ping"
        ));
    }

    #[test]
    fn call_args_default_to_null() {
        assert_eq!(parse_args_json(""), serde_json::Value::Null);
    }

    #[test]
    fn call_args_prefer_json_but_accept_bare_strings() {
        assert_eq!(
            parse_args_json(r#"{"level":0.4}"#),
            serde_json::json!({"level": 0.4})
        );
        assert_eq!(
            parse_args_json("context"),
            serde_json::Value::String("context".into())
        );
    }
}
