use crate::models::usuario::RolUsuario;

/// La autorización real ya no vive acá -- vive en el panel administrativo
/// (`administradores_panel`/Supabase Auth): quien tiene una cuenta
/// concedida ahí es quien puede operar la app, punto. `Root`/`Administrador`/
/// `Operador` sigue existiendo como etiqueta informativa (de dónde viene
/// cada `usuarios.rol` sincronizado), pero ninguna acción DENTRO de la app
/// se le niega a nadie por su rol -- ver docs/decisiones-tecnicas.md,
/// entrada "aplanado de roles". `Operacion` queda como catálogo de qué se
/// podía restringir antes, no como gate activo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operacion {
    EditarCedulaContratista,
    ActivarDesactivarContratista,
    ActivarDesactivarEmpresa,
    VerAuditoria,
    /// Persistencia en la nube: sincronizar, leer y cerrar ingresos
    /// abiertos por el otro dispositivo del sitio -- uso diario normal (la
    /// pantalla Activos las usa para cualquiera que esté registrando
    /// ingresos), y el disparador automático de fondo tampoco debe
    /// depender de que justo haya una sesión con algún rol particular
    /// abierta en ese momento.
    UsarNube,
}

impl RolUsuario {
    pub fn puede(self, _operacion: Operacion) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLES: [RolUsuario; 3] = [
        RolUsuario::Root,
        RolUsuario::Administrador,
        RolUsuario::Operador,
    ];
    const OPERACIONES: [Operacion; 5] = [
        Operacion::EditarCedulaContratista,
        Operacion::ActivarDesactivarContratista,
        Operacion::ActivarDesactivarEmpresa,
        Operacion::VerAuditoria,
        Operacion::UsarNube,
    ];

    #[test]
    fn ningun_rol_esta_restringido_por_operacion() {
        for rol in ROLES {
            for operacion in OPERACIONES {
                assert!(
                    rol.puede(operacion),
                    "se esperaba que {rol:?} pudiera {operacion:?}"
                );
            }
        }
    }
}
