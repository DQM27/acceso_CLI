use control_acceso::database::queries::usuarios::FiltroUsuarios;
use control_acceso::models::usuario::RolUsuario;
use control_acceso::services::usuario_service::{ActualizarUsuarioInput, CrearUsuarioInput};

#[derive(serde::Deserialize, Default)]
pub struct FiltroUsuariosEntrada {
    pub texto: Option<String>,
}

impl FiltroUsuariosEntrada {
    pub fn construir(self) -> FiltroUsuarios {
        FiltroUsuarios {
            texto: self
                .texto
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty()),
            ..Default::default()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct CrearUsuarioEntrada {
    pub cedula: String,
    pub nombre: String,
    pub password: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

impl From<CrearUsuarioEntrada> for CrearUsuarioInput {
    fn from(entrada: CrearUsuarioEntrada) -> Self {
        CrearUsuarioInput {
            cedula: entrada.cedula,
            nombre: entrada.nombre,
            password: entrada.password,
            rol: entrada.rol,
            activo: entrada.activo,
        }
    }
}

/// Sin contraseña — `AppCore::actualizar_usuario` no la toca; cambiarla es un
/// comando aparte (`cambiar_password_usuario`), igual que en la TUI.
#[derive(serde::Deserialize)]
pub struct ActualizarUsuarioEntrada {
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

impl ActualizarUsuarioEntrada {
    pub fn input(&self) -> ActualizarUsuarioInput {
        ActualizarUsuarioInput {
            cedula: self.cedula.clone(),
            nombre: self.nombre.clone(),
            rol: self.rol,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_texto_en_blanco_se_convierte_en_ninguno() {
        let filtro = FiltroUsuariosEntrada {
            texto: Some("   ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, None);
    }

    #[test]
    fn el_texto_se_recorta_de_espacios() {
        let filtro = FiltroUsuariosEntrada {
            texto: Some("  admin  ".to_owned()),
        }
        .construir();

        assert_eq!(filtro.texto, Some("admin".to_owned()));
    }
}
