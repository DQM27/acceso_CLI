use crate::models::usuario::RolUsuario;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operacion {
    GestionarRespaldos,
    GestionarUsuarios,
    EditarCedulaContratista,
    ActivarDesactivarContratista,
    ActivarDesactivarEmpresa,
    VerAuditoria,
    /// Persistencia en la nube (`docs/plan-persistencia-nube.md`): guardar
    /// el secreto de este dispositivo. Exclusivo de ROOT, ni siquiera
    /// Administrador -- ese secreto es la identidad del dispositivo entero
    /// ante el receptor, no una preferencia operativa.
    GestionarNube,
    /// Persistencia en la nube: sincronizar, leer y cerrar ingresos
    /// abiertos por el otro dispositivo del sitio. A diferencia de
    /// `GestionarNube`, cualquier rol puede -- estas acciones ya no son
    /// "administración", son parte del uso diario normal (la pantalla
    /// Activos las usa para cualquiera que esté registrando ingresos), y el
    /// disparador automático de fondo tampoco debe depender de que
    /// justo haya una sesión ROOT abierta en ese momento.
    UsarNube,
}

impl RolUsuario {
    pub fn puede(self, operacion: Operacion) -> bool {
        match self {
            Self::Root => true,
            Self::Administrador => !matches!(
                operacion,
                Operacion::GestionarRespaldos | Operacion::GestionarNube
            ),
            Self::Operador => matches!(operacion, Operacion::UsarNube),
        }
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
    const OPERACIONES: [Operacion; 8] = [
        Operacion::GestionarRespaldos,
        Operacion::GestionarUsuarios,
        Operacion::EditarCedulaContratista,
        Operacion::ActivarDesactivarContratista,
        Operacion::ActivarDesactivarEmpresa,
        Operacion::VerAuditoria,
        Operacion::GestionarNube,
        Operacion::UsarNube,
    ];

    #[test]
    fn matriz_completa_de_permisos_por_operacion() {
        let esperados = [
            [true, true, true, true, true, true, true, true],
            [false, true, true, true, true, true, false, true],
            [false, false, false, false, false, false, false, true],
        ];

        for (indice_rol, rol) in ROLES.into_iter().enumerate() {
            for (indice_operacion, operacion) in OPERACIONES.into_iter().enumerate() {
                assert_eq!(
                    rol.puede(operacion),
                    esperados[indice_rol][indice_operacion],
                    "permiso inesperado para {rol:?} y {operacion:?}"
                );
            }
        }
    }

    #[test]
    fn matriz_completa_para_gestionar_usuarios() {
        let esperados = [
            [true, true, true],
            [false, true, true],
            [false, false, false],
        ];

        for (indice_actor, actor) in ROLES.into_iter().enumerate() {
            for (indice_objetivo, objetivo) in ROLES.into_iter().enumerate() {
                assert_eq!(
                    puede_gestionar_usuario(actor, objetivo),
                    esperados[indice_actor][indice_objetivo],
                    "gestión inesperada para actor {actor:?} y objetivo {objetivo:?}"
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
