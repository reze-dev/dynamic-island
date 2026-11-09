//! A fake MPRIS media player, for testing MPRIS without a real application.
//!
//! It exports `org.mpris.MediaPlayer2` at `/org/mpris/MediaPlayer2`, registers
//! on the session bus, and toggles PlaybackStatus and Metadata on a timer.
//!
//!   cargo run --example fake-mpris
//!
//! The bus name is on the command line, so several can run at once.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use zbus::{Connection, interface};

/// The mutable state both interfaces read: the player owns it, the root
/// view of the same fake just needs the player's identity.
#[derive(Default, Clone)]
struct PlayerState {
    playback_status: Arc<Mutex<String>>,
    title: Arc<Mutex<String>>,
    artist: Arc<Mutex<String>>,
    art_url: Arc<Mutex<String>>,
    position: Arc<Mutex<i64>>,
}

/// The fake as a `.Player`: holds the state plus the static capability
/// flags. zbus permits one `#[interface]` per type, so the root view of
/// the fake is a separate, stateless struct.
#[derive(Clone)]
#[allow(non_snake_case)]
struct FakePlayer {
    state: PlayerState,
    can_play: bool,
    can_pause: bool,
    can_go_next: bool,
    can_go_previous: bool,
}

/// The fake as the root `org.mpris.MediaPlayer2` interface.
#[allow(non_snake_case)]
struct FakeRoot;

/// The `.Player` interface: playback status, metadata, and the transport
/// methods. Consumers (Quickshell's Mpris service) look here.
#[allow(non_snake_case)]
#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl FakePlayer {
    #[zbus(property)]
    async fn PlaybackStatus(&self) -> String {
        self.state.playback_status.lock().await.clone()
    }

    #[zbus(property)]
    async fn Metadata(&self) -> HashMap<String, zbus::zvariant::Value<'static>> {
        let mut m = HashMap::new();
        m.insert(
            "xesam:title".to_string(),
            zbus::zvariant::Value::Str(self.state.title.lock().await.clone().into()),
        );
        m.insert(
            "xesam:artist".to_string(),
            zbus::zvariant::Value::Str(self.state.artist.lock().await.clone().into()),
        );
        let art = self.state.art_url.lock().await.clone();
        if !art.is_empty() {
            m.insert(
                "mpris:artUrl".to_string(),
                zbus::zvariant::Value::Str(art.into()),
            );
        }
        m
    }

    #[zbus(property)]
    fn CanPlay(&self) -> bool {
        self.can_play
    }

    #[zbus(property)]
    fn CanPause(&self) -> bool {
        self.can_pause
    }

    #[zbus(property)]
    fn CanGoNext(&self) -> bool {
        self.can_go_next
    }

    #[zbus(property)]
    fn CanGoPrevious(&self) -> bool {
        self.can_go_previous
    }

    #[zbus(property)]
    async fn Position(&self) -> i64 {
        *self.state.position.lock().await
    }

    async fn Play(&self) {
        println!("Play called");
        *self.state.playback_status.lock().await = "Playing".to_string();
    }

    async fn Pause(&self) {
        println!("Pause called");
        *self.state.playback_status.lock().await = "Paused".to_string();
    }

    async fn Next(&self) {
        println!("Next called");
        *self.state.title.lock().await = "Next Song".to_string();
        *self.state.artist.lock().await = "Next Artist".to_string();
    }

    async fn Previous(&self) {
        println!("Previous called");
        *self.state.title.lock().await = "Previous Song".to_string();
        *self.state.artist.lock().await = "Previous Artist".to_string();
    }

    async fn PlayPause(&self) {
        println!("PlayPause called");
        let mut s = self.state.playback_status.lock().await;
        *s = if s.as_str() == "Playing" {
            "Paused".to_string()
        } else {
            "Playing".to_string()
        };
    }

    async fn Seek(&self, _offset: i64) {}

    async fn OpenUri(&self, _uri: &str) {}
}

/// The root interface: identity plus the window verbs an MPRIS player has.
#[allow(non_snake_case)]
#[interface(name = "org.mpris.MediaPlayer2")]
impl FakeRoot {
    #[zbus(property)]
    fn Identity(&self) -> &str {
        "Fake Media Player"
    }

    #[zbus(property)]
    fn DesktopEntry(&self) -> &str {
        "fake-mpris"
    }

    #[zbus(property)]
    fn CanQuit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn CanSetFullscreen(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn HasTrackList(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn CanRaise(&self) -> bool {
        true
    }

    async fn Quit(&self) {
        println!("Quit called");
    }

    async fn Raise(&self) {
        println!("Raise called");
    }
}

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let mut name = "org.mpris.MediaPlayer2.fake".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => name = args.next().unwrap_or_default(),
            other => {
                eprintln!("unknown argument: {other}");
                eprintln!("usage: fake-mpris [--name N]");
                std::process::exit(2);
            }
        }
    }

    println!("fake-mpris: exporting MPRIS at /org/mpris/MediaPlayer2");
    println!("fake-mpris: bus name = {name}");
    println!("fake-mpris: waiting. press ctrl-c to stop.");

    let connection = Connection::session().await?;
    connection
        .request_name_with_flags(
            name.as_str(),
            zbus::fdo::RequestNameFlags::ReplaceExisting.into(),
        )
        .await?;

    let state = PlayerState {
        playback_status: Arc::new(Mutex::new("Playing".to_string())),
        title: Arc::new(Mutex::new("Fake Song".to_string())),
        artist: Arc::new(Mutex::new("Fake Artist".to_string())),
        // No real art: a fake URL would 404 in consumers that try to fetch
        // it. The empty string omits `mpris:artUrl` from Metadata.
        art_url: Arc::new(Mutex::new(String::new())),
        position: Arc::new(Mutex::new(0)),
    };

    let player = FakePlayer {
        state: state.clone(),
        can_play: true,
        can_pause: true,
        can_go_next: true,
        can_go_previous: true,
    };

    // Export at /org/mpris/MediaPlayer2. zbus serves both interfaces
    // (`org.mpris.MediaPlayer2` and `org.mpris.MediaPlayer2.Player`) on
    // the same object, as the MPRIS spec requires.
    connection
        .object_server()
        .at("/org/mpris/MediaPlayer2", player.clone())
        .await?;
    connection
        .object_server()
        .at("/org/mpris/MediaPlayer2", FakeRoot)
        .await?;

    // Timer to toggle state every few seconds.
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut s = state_clone.playback_status.lock().await;
            *s = if s.as_str() == "Playing" {
                "Paused".to_string()
            } else {
                "Playing".to_string()
            };
            drop(s);
            let mut title = state_clone.title.lock().await;
            *title = if title.as_str() == "Fake Song" {
                "Another Song".to_string()
            } else {
                "Fake Song".to_string()
            };
            let mut artist = state_clone.artist.lock().await;
            *artist = if artist.as_str() == "Fake Artist" {
                "Another Artist".to_string()
            } else {
                "Fake Artist".to_string()
            };
            println!(
                "timer: PlaybackStatus -> {}, title -> {}",
                *state_clone.playback_status.lock().await,
                *title
            );
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}
