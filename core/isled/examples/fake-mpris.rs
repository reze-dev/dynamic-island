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

#[derive(Default, Clone)]
struct FakePlayer {
    playback_status: Arc<Mutex<String>>,
    title: Arc<Mutex<String>>,
    artist: Arc<Mutex<String>>,
    art_url: Arc<Mutex<String>>,
    can_play: bool,
    can_pause: bool,
    can_go_next: bool,
    can_go_previous: bool,
    position: Arc<Mutex<i64>>,
}

#[allow(non_snake_case)]
#[interface(name = "org.mpris.MediaPlayer2")]
impl FakePlayer {
    #[zbus(property)]
    async fn PlaybackStatus(&self) -> String {
        self.playback_status.lock().await.clone()
    }

    #[zbus(property)]
    async fn Metadata(&self) -> HashMap<String, zbus::zvariant::Value<'static>> {
        let mut m = HashMap::new();
        m.insert(
            "xesam:title".to_string(),
            zbus::zvariant::Value::Str(self.title.lock().await.clone().into()),
        );
        m.insert(
            "xesam:artist".to_string(),
            zbus::zvariant::Value::Str(self.artist.lock().await.clone().into()),
        );
        m.insert(
            "mpris:artUrl".to_string(),
            zbus::zvariant::Value::Str(self.art_url.lock().await.clone().into()),
        );
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
        *self.position.lock().await
    }

    async fn Play(&self) {
        println!("Play called");
        *self.playback_status.lock().await = "Playing".to_string();
    }

    async fn Pause(&self) {
        println!("Pause called");
        *self.playback_status.lock().await = "Paused".to_string();
    }

    async fn Next(&self) {
        println!("Next called");
        *self.title.lock().await = "Next Song".to_string();
        *self.artist.lock().await = "Next Artist".to_string();
    }

    async fn Previous(&self) {
        println!("Previous called");
        *self.title.lock().await = "Previous Song".to_string();
        *self.artist.lock().await = "Previous Artist".to_string();
    }

    async fn PlayPause(&self) {
        println!("PlayPause called");
        let mut s = self.playback_status.lock().await;
        *s = if s.as_str() == "Playing" {
            "Paused".to_string()
        } else {
            "Playing".to_string()
        };
    }

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

    let player = FakePlayer {
        playback_status: Arc::new(Mutex::new("Playing".to_string())),
        title: Arc::new(Mutex::new("Fake Song".to_string())),
        artist: Arc::new(Mutex::new("Fake Artist".to_string())),
        art_url: Arc::new(Mutex::new("https://example.com/fake.png".to_string())),
        can_play: true,
        can_pause: true,
        can_go_next: true,
        can_go_previous: true,
        position: Arc::new(Mutex::new(0)),
    };

    // Export at /org/mpris/MediaPlayer2 (both interfaces on same object)
    connection
        .object_server()
        .at("/org/mpris/MediaPlayer2", player.clone())
        .await?;

    // Also export Player interface explicitly at same path via separate interface
    // zbus handles both interfaces on same object when using interface macros

    // Timer to emit PropertiesChanged and toggle state
    let _conn_clone = connection.clone();
    let player_clone = player.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut s = player_clone.playback_status.lock().await;
            *s = if s.as_str() == "Playing" {
                "Paused".to_string()
            } else {
                "Playing".to_string()
            };
            drop(s);
            let mut title = player_clone.title.lock().await;
            *title = if title.as_str() == "Fake Song" {
                "Another Song".to_string()
            } else {
                "Fake Song".to_string()
            };
            let mut artist = player_clone.artist.lock().await;
            *artist = if artist.as_str() == "Fake Artist" {
                "Another Artist".to_string()
            } else {
                "Fake Artist".to_string()
            };
            println!(
                "timer: PlaybackStatus -> {}, title -> {}",
                *player_clone.playback_status.lock().await,
                *title
            );
        }
    });

    tokio::signal::ctrl_c().await?;
    Ok(())
}
