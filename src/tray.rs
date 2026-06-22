//! System-tray status/quit UI, and the helper's main-thread entry point.
//!
//! `tray-icon`'s menu and `tao`'s event loop must own the main thread, so [`run`]
//! is what `main` hands control to. The tokio runtime (daemon + WS server) is
//! driven on worker threads; the two sides talk over the loop's [`EventLoopProxy`]:
//! the async side pushes [`Status`] updates, and menu clicks are bridged in as
//! [`UserEvent::MenuClick`] so the loop can idle on `ControlFlow::Wait`.
//!
//! [`EventLoopProxy`]: tao::event_loop::EventLoopProxy

use std::net::Ipv4Addr;
use std::sync::Arc;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tracing::{error, info};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::daemon::Daemon;
use crate::ws;

/// What the tray reports about the helper's lifecycle.
enum Status {
    Starting,
    Running(u16),
    Failed(String),
}

impl Status {
    /// Text for the disabled status line in the tray menu.
    fn label(&self) -> String {
        match self {
            Status::Starting => "Starting…".into(),
            Status::Running(port) => format!("Running on :{port}"),
            Status::Failed(msg) => format!("Failed: {msg}"),
        }
    }

    /// Hover tooltip on the tray icon itself.
    fn tooltip(&self) -> String {
        format!("thingblock-link — {}", self.label())
    }
}

/// Things that wake the event loop. Both the async side and menu clicks funnel
/// here so the loop stays on `ControlFlow::Wait` rather than busy-polling.
enum UserEvent {
    Status(Status),
    MenuClick(MenuId),
}

/// Tray handles kept alive for the lifetime of the loop. The status item is
/// retained so its text can be updated; the quit id is matched against clicks.
struct Tray {
    _icon: TrayIcon,
    status_item: MenuItem,
    quit_id: MenuId,
}

/// Run the helper: spawn the daemon + WS server on `runtime`, then drive the
/// tray's event loop on this (main) thread. Diverges — the process exits from
/// within the loop on Quit.
pub fn run(runtime: Runtime, port: u16) -> ! {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Bridge muda's global menu-event channel into our loop as a user event.
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = menu_proxy.send_event(UserEvent::MenuClick(event.id));
    }));

    // Start the daemon + WS server; status flows back through the proxy.
    runtime.spawn(run_services(port, proxy));

    // The tray is built on `Init` (macOS requires icon creation after the loop
    // starts); `runtime` lives in an Option so Quit can take it exactly once.
    let mut tray: Option<Tray> = None;
    let mut runtime = Some(runtime);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => tray = Some(build_tray()),
            Event::UserEvent(UserEvent::Status(status)) => {
                if let Some(tray) = &tray {
                    tray.status_item.set_text(status.label());
                    let _ = tray._icon.set_tooltip(Some(status.tooltip()));
                }
            }
            Event::UserEvent(UserEvent::MenuClick(id))
                if tray.as_ref().is_some_and(|t| t.quit_id == id) =>
            {
                info!("quit requested; shutting down");
                // Aborting the runtime drops the last `Arc<Daemon>`, whose
                // `kill_on_drop` reaps the arduino-cli daemon.
                if let Some(runtime) = runtime.take() {
                    runtime.shutdown_background();
                }
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    })
}

/// Spawn + own the daemon and serve the editor over WS, reporting lifecycle to
/// the tray. Terminal states (`Failed`) are reported and the task returns; the
/// process keeps running so the tray stays usable for Quit.
async fn run_services(port: u16, proxy: tao::event_loop::EventLoopProxy<UserEvent>) {
    let _ = proxy.send_event(UserEvent::Status(Status::Starting));

    let daemon = match Daemon::start().await {
        Ok(daemon) => Arc::new(daemon),
        Err(e) => {
            error!(error = %e, "daemon failed to start");
            let _ = proxy.send_event(UserEvent::Status(Status::Failed(e.to_string())));
            return;
        }
    };

    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
        Ok(listener) => listener,
        Err(e) => {
            error!(error = %e, port, "failed to bind WS port");
            let _ = proxy.send_event(UserEvent::Status(Status::Failed(e.to_string())));
            return;
        }
    };

    let _ = proxy.send_event(UserEvent::Status(Status::Running(port)));
    if let Err(e) = ws::server::serve(listener, daemon).await {
        error!(error = %e, "ws server stopped");
        let _ = proxy.send_event(UserEvent::Status(Status::Failed(e.to_string())));
    }
}

/// Build the tray icon and its menu: a disabled status line, a separator, and
/// Quit. A failure here means the OS rejected the tray, which is unrecoverable
/// for this UI, so we surface it loudly.
fn build_tray() -> Tray {
    let status_item = MenuItem::new(Status::Starting.label(), false, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    menu.append_items(&[&status_item, &PredefinedMenuItem::separator(), &quit_item])
        .expect("build tray menu");

    let icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(Status::Starting.tooltip())
        .with_icon(make_icon())
        .build()
        .expect("build tray icon");

    Tray {
        _icon: icon,
        status_item,
        quit_id: quit_item.id().clone(),
    }
}

/// A plain 32×32 solid-color icon, generated in code so the preview needs no
/// bundled asset. Swap in the real logo (decoded via `image`) before release.
fn make_icon() -> Icon {
    const SIZE: u32 = 32;
    const RGBA: [u8; 4] = [0x2d, 0xd4, 0xbf, 0xff]; // teal
    let rgba = RGBA.repeat((SIZE * SIZE) as usize);
    Icon::from_rgba(rgba, SIZE, SIZE).expect("build tray icon image")
}
