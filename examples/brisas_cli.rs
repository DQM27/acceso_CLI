#[path = "brisas_cli/activos_v2.rs"]
mod activos_v2;
#[path = "brisas_cli/app.rs"]
mod app;
#[path = "brisas_cli/configuracion_inicial_v2.rs"]
mod configuracion_inicial_v2;
#[path = "brisas_cli/contratistas_v2.rs"]
mod contratistas_v2;
#[path = "brisas_cli/empresas_v2.rs"]
mod empresas_v2;
#[path = "brisas_cli/ingreso.rs"]
mod ingreso;
#[path = "brisas_cli/historial_v2.rs"]
mod historial_v2;
#[path = "brisas_cli/ingreso_v2.rs"]
mod ingreso_v2;
#[path = "brisas_cli/login.rs"]
mod login;
#[path = "brisas_cli/login_v2.rs"]
mod login_v2;
#[path = "brisas_cli/menu.rs"]
mod menu;
#[path = "brisas_cli/menu_v2.rs"]
mod menu_v2;
#[path = "brisas_cli/quick_exit.rs"]
mod quick_exit;
#[path = "brisas_cli/terminal.rs"]
mod terminal;
#[path = "brisas_cli/usuarios_v2.rs"]
mod usuarios_v2;

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

impl PilotScreen for activos_v2::ActivosV2Pilot {
    fn is_running(&self) -> bool {
        activos_v2::ActivosV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        activos_v2::ActivosV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        activos_v2::ActivosV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
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

impl PilotScreen for contratistas_v2::ContratistasV2Pilot {
    fn is_running(&self) -> bool {
        contratistas_v2::ContratistasV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        contratistas_v2::ContratistasV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        contratistas_v2::ContratistasV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
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

impl PilotScreen for menu_v2::MenuV2Pilot {
    fn is_running(&self) -> bool {
        menu_v2::MenuV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        menu_v2::MenuV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        menu_v2::MenuV2Pilot::render(self, frame);
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

impl PilotScreen for login_v2::LoginV2Pilot {
    fn is_running(&self) -> bool {
        login_v2::LoginV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        login_v2::LoginV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        login_v2::LoginV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        login_v2::LoginV2Pilot::redraw_interval(self)
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

impl PilotScreen for ingreso_v2::IngresoV2Pilot {
    fn is_running(&self) -> bool {
        ingreso_v2::IngresoV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        ingreso_v2::IngresoV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        ingreso_v2::IngresoV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

impl PilotScreen for historial_v2::HistorialV2Pilot {
    fn is_running(&self) -> bool {
        historial_v2::HistorialV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        historial_v2::HistorialV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        historial_v2::HistorialV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

impl PilotScreen for empresas_v2::EmpresasV2Pilot {
    fn is_running(&self) -> bool {
        empresas_v2::EmpresasV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        empresas_v2::EmpresasV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        empresas_v2::EmpresasV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

impl PilotScreen for usuarios_v2::UsuariosV2Pilot {
    fn is_running(&self) -> bool {
        usuarios_v2::UsuariosV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        usuarios_v2::UsuariosV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        usuarios_v2::UsuariosV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        Duration::from_millis(250)
    }
}

impl PilotScreen for configuracion_inicial_v2::ConfiguracionInicialV2Pilot {
    fn is_running(&self) -> bool {
        configuracion_inicial_v2::ConfiguracionInicialV2Pilot::is_running(self)
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        configuracion_inicial_v2::ConfiguracionInicialV2Pilot::handle_event(self, event)
    }

    fn render(&mut self, frame: &mut Frame) {
        configuracion_inicial_v2::ConfiguracionInicialV2Pilot::render(self, frame);
    }

    fn redraw_interval(&self) -> Duration {
        configuracion_inicial_v2::ConfiguracionInicialV2Pilot::redraw_interval(self)
    }
}

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("login") => terminal::run(login::LoginPilot::default()),
        Some("login-v2") => terminal::run(login_v2::LoginV2Pilot::default()),
        Some("ingreso") => terminal::run(ingreso::EntryPilot::default()),
        Some("ingreso-v2") => terminal::run(ingreso_v2::IngresoV2Pilot::default()),
        Some("menu") => terminal::run(menu::MenuPilot::default()),
        Some("menu-v2") => terminal::run(menu_v2::MenuV2Pilot::default()),
        None | Some("contratistas") => terminal::run(app::PilotApp::default()),
        Some("contratistas-v2") => {
            terminal::run(contratistas_v2::ContratistasV2Pilot::default())
        }
        Some("activos-v2") => terminal::run(activos_v2::ActivosV2Pilot::default()),
        Some("historial-v2") => terminal::run(historial_v2::HistorialV2Pilot::default()),
        Some("empresas-v2") => terminal::run(empresas_v2::EmpresasV2Pilot::default()),
        Some("usuarios-v2") => terminal::run(usuarios_v2::UsuariosV2Pilot::default()),
        Some("configuracion-inicial-v2") => terminal::run(
            configuracion_inicial_v2::ConfiguracionInicialV2Pilot::default(),
        ),
        Some("--help" | "-h") => {
            println!(
                "Uso: cargo run --example brisas_cli -- [contratistas|contratistas-v2|menu|menu-v2|login|login-v2|ingreso|ingreso-v2|activos-v2|historial-v2|empresas-v2|usuarios-v2|configuracion-inicial-v2]"
            );
            Ok(())
        }
        Some(view) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("vista desconocida: {view}"),
        )),
    }
}
