#[path = "brisas_cli/app.rs"]
mod app;
#[path = "brisas_cli/ingreso.rs"]
mod ingreso;
#[path = "brisas_cli/login.rs"]
mod login;
#[path = "brisas_cli/menu.rs"]
mod menu;
#[path = "brisas_cli/terminal.rs"]
mod terminal;

use crossterm::event::Event;
use ratatui::Frame;
use std::time::Duration;

trait PilotScreen {
    fn is_running(&self) -> bool;
    fn handle_event(&mut self, event: &Event) -> bool;
    fn render(&mut self, frame: &mut Frame);

    fn redraw_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

impl PilotScreen for app::PilotApp {
    fn is_running(&self) -> bool {
        app::PilotApp::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        app::PilotApp::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        app::PilotApp::render(self, frame);
    }
}

impl PilotScreen for menu::MenuPilot {
    fn is_running(&self) -> bool {
        menu::MenuPilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        menu::MenuPilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        menu::MenuPilot::render(self, frame);
    }
}

impl PilotScreen for login::LoginPilot {
    fn is_running(&self) -> bool {
        login::LoginPilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        login::LoginPilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        login::LoginPilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

impl PilotScreen for ingreso::EntryPilot {
    fn is_running(&self) -> bool {
        ingreso::EntryPilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        ingreso::EntryPilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        ingreso::EntryPilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("login") => terminal::run(login::LoginPilot::default()),
        Some("ingreso") => terminal::run(ingreso::EntryPilot::default()),
        Some("menu") => terminal::run(menu::MenuPilot::default()),
        None | Some("contratistas") => terminal::run(app::PilotApp::default()),
        Some("--help" | "-h") => {
            println!("Uso: cargo run --example brisas_cli -- [contratistas|menu|login|ingreso]");
            Ok(())
        }
        Some(view) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("vista desconocida: {view}"),
        )),
    }
}
