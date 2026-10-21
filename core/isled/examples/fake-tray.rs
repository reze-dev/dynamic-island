//! A fake tray item, for testing the tray without an application.
//!
//! It exports the `org.kde.StatusNotifierItem` interface at the path the
//! specification says, registers itself with the watcher, and then waits.
//! The point is that the whole path can be exercised without anything real
//! being installed: the daemon has to find the item, read its properties,
//! and show it.
//!
//!   cargo run --example fake-tray
//!
//! The name it registers under is on the command line, so several can run at
//! once and the panel can be seen with more than one item in it.

/// A pixmap, which the fake item does not have.
type Pixmap = Vec<(i32, i32, Vec<u8>)>;

struct FakeItem {
    title: String,
    icon: String,
    status: String,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl FakeItem {
    #[zbus(property)]
    fn category(&self) -> &str {
        "ApplicationStatus"
    }

    #[zbus(property)]
    fn id(&self) -> &str {
        "fake-tray-item"
    }

    #[zbus(property)]
    fn title(&self) -> &str {
        &self.title
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        &self.status
    }

    #[zbus(property)]
    fn icon_name(&self) -> &str {
        &self.icon
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }

    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        "dialog-warning-symbolic"
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn menu(&self) -> String {
        "/MenuBar".to_string()
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Pixmap {
        Vec::new()
    }

    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Pixmap {
        Vec::new()
    }

    #[zbus(property)]
    fn attention_icon_pixmap(&self) -> Pixmap {
        Vec::new()
    }

    #[zbus(property)]
    fn tool_tip(&self) -> (String, Pixmap, String, String) {
        (
            self.icon.clone(),
            Vec::new(),
            self.title.clone(),
            "a fake item, for testing".to_string(),
        )
    }

    fn activate(&self, x: i32, y: i32) {
        println!("activated at {x},{y}");
    }

    fn secondary_activate(&self, x: i32, y: i32) {
        println!("secondary activated at {x},{y}");
    }

    fn scroll(&self, delta: i32, orientation: &str) {
        println!("scrolled {delta} {orientation}");
    }

    fn context_menu(&self, x: i32, y: i32) {
        println!("menu requested at {x},{y}");
    }
}

#[tokio::main]
async fn main() -> zbus::Result<()> {
    let mut title = "Fake Item".to_string();
    let mut icon = "network-transmit-receive-symbolic".to_string();
    let mut status = "Active".to_string();
    let mut name = "org.kde.StatusNotifierItem-fake".to_string();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--title" => title = args.next().unwrap_or_default(),
            "--icon" => icon = args.next().unwrap_or_default(),
            "--status" => status = args.next().unwrap_or_default(),
            "--name" => name = args.next().unwrap_or_default(),
            other => {
                eprintln!("unknown argument: {other}");
                eprintln!("usage: fake-tray [--title T] [--icon I] [--status S] [--name N]");
                std::process::exit(2);
            }
        }
    }

    let connection = zbus::Connection::session().await?;
    connection
        .request_name_with_flags(
            name.as_str(),
            zbus::fdo::RequestNameFlags::ReplaceExisting.into(),
        )
        .await?;

    let item = FakeItem {
        title,
        icon,
        status,
    };
    connection
        .object_server()
        .at("/StatusNotifierItem", item)
        .await?;

    // Tell the watcher about it, the way a real item does.
    let proxy = zbus::Proxy::new(
        &connection,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    )
    .await?;
    match proxy
        .call_method("RegisterStatusNotifierItem", &(name.clone(),))
        .await
    {
        Ok(_) => println!("registered {name}"),
        Err(err) => {
            eprintln!("could not register: {err}");
            eprintln!("is the watcher running?");
        }
    }

    println!("waiting. press ctrl-c to stop.");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
