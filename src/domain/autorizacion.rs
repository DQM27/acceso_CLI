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
    GestionarUsuarios,
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

pub fn puede_gestionar_usuario(actor: RolUsuario, objetivo: RolUsuario) -> bool {
    actor.puede(Operacion::GestionarUsuarios)
        && (objetivo != RolUsuario::Root || actor == RolUsuario::Root)
}

pub fn puede_cambiar_password(
    actor_id: i64,
    actor_rol: RolUsuario,
    objetivo_id: i64,
    objetivo_rol: RolUsuario,
) -> bool {
    actor_id == objetivo_id || puede_gestionar_usuario(actor_rol, objetivo_rol)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROLES: [RolUsuario; 3] = [
        RolUsuario::Root,
        RolUsuario::Administrador,
        RolUsuario::Operador,
    ];
    const OPERACIONES: [Operacion; 6] = [
        Operacion::GestionarUsuarios,
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

    /// `puede_gestionar_usuario` ya no depende del rol del actor (aplanado),
    /// pero conserva la única protección real que quedaba acá: nadie
    /// promueve/gestiona a un ROOT salvo otro ROOT -- ver
    /// docs/decisiones-tecnicas.md.
    #[test]
    fn nadie_gestiona_a_un_root_salvo_otro_root() {
        for actor in ROLES {
            assert_eq!(
                puede_gestionar_usuario(actor, RolUsuario::Root),
                actor == RolUsuario::Root,
                "gestión de un ROOT inesperada para actor {actor:?}"
            );
            for objetivo in [RolUsuario::Administrador, RolUsuario::Operador] {
                assert!(
                    puede_gestionar_usuario(actor, objetivo),
                    "se esperaba que {actor:?} pudiera gestionar a {objetivo:?}"
                );
            }
        }
    }

    #[test]
    fn cambio_de_password_propio_siempre_es_posible_y_el_ajeno_sigue_la_matriz() {
        for actor in ROLES {
            for objetivo in ROLES {
                assert!(puede_cambiar_password(7, actor, 7, objetivo));
                assert_eq!(
                    puede_cambiar_password(7, actor, 8, objetivo),
                    puede_gestionar_usuario(actor, objetivo),
                    "cambio ajeno inesperado para actor {actor:?} y objetivo {objetivo:?}"
                );
            }
        }
    }
}
