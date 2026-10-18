//! "¿Quién tiene el gafete N?" -- la respuesta, de todas las clases a la vez.
//!
//! Los gafetes son de conjuntos distintos (`TipoGafete`: contratista,
//! proveedor, visita y provisional KOF) y cada conjunto numera por su cuenta:
//! el 1 de un proveedor y el 1 de un contratista son dos gafetes físicos
//! distintos y pueden estar adentro a la vez. Por eso un número puede tener
//! MÁS DE UN tenedor, y quien da la salida tiene que saber a cuál se refiere.
//! Esa respuesta es del núcleo, no de cada pantalla: así escritorio y móvil
//! cuentan lo mismo y ninguna tiene que juntar cuatro listas para averiguarlo.
//!
//! Mira lo abierto en este equipo y, si la sesión puede usar la nube, lo abierto
//! por el otro dispositivo del sitio (las cachés `*_remotos`). Consulta directo
//! por número, sin pasar por las listas de activos, que pueden venir recortadas.

use rusqlite::params;

use crate::database::error::DatabaseError;
use crate::services::autenticacion_service::UsuarioSesion;

use super::AppCore;

/// A qué conjunto de gafetes pertenece el ingreso que lo tiene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ClaseTenedorGafete {
    Contratista,
    Proveedor,
    /// Visita autorizada por correo (gafete de visita).
    Correo,
    /// Préstamo de un gafete provisional KOF.
    Kof,
}

/// Dónde vive el registro: en este equipo (con su `id`) o en el otro
/// dispositivo del sitio (con su `uuid`). Es lo que hace falta para cerrarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "tipo", rename_all = "snake_case"))]
pub enum OrigenTenedorGafete {
    Local { id: i64 },
    Remoto { uuid: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TenedorGafete {
    pub clase: ClaseTenedorGafete,
    pub origen: OrigenTenedorGafete,
    pub nombre: String,
    /// Lo que acompaña al nombre para reconocer a la persona: la empresa
    /// (contratista, proveedor), el motivo (correo) o el código de empleado
    /// (KOF, que no tiene cédula).
    pub detalle: Option<String>,
}

impl AppCore {
    /// Todos los que tienen abierto el gafete `numero`, de cualquier clase:
    /// primero lo de este equipo y después lo del otro dispositivo. Vacío si
    /// nadie lo tiene. Más de uno es normal (conjuntos de gafetes distintos).
    pub fn tenedores_del_gafete(
        &self,
        actor: &UsuarioSesion,
        numero: i64,
    ) -> Result<Vec<TenedorGafete>, DatabaseError> {
        #[cfg_attr(not(feature = "nube"), allow(unused_mut))]
        let mut tenedores = self.tenedores_locales(numero)?;
        #[cfg(feature = "nube")]
        if self.autorizar_uso_nube(actor).is_ok() {
            tenedores.extend(self.tenedores_remotos(numero)?);
        }
        #[cfg(not(feature = "nube"))]
        let _ = actor;
        Ok(tenedores)
    }

    fn tenedores_locales(&self, numero: i64) -> Result<Vec<TenedorGafete>, DatabaseError> {
        let mut tenedores = self.leer_tenedores(
            "SELECT id, contratista_nombre, empresa_nombre FROM registro_ingresos
             WHERE fecha_hora_salida IS NULL AND gafete_numero = ?1
             ORDER BY fecha_hora_ingreso",
            numero,
            ClaseTenedorGafete::Contratista,
            true,
        )?;
        tenedores.extend(self.leer_tenedores(
            "SELECT id, nombre, empresa_nombre FROM registro_ingresos_proveedor
             WHERE fecha_hora_salida IS NULL AND gafete_numero = ?1
             ORDER BY fecha_hora_ingreso",
            numero,
            ClaseTenedorGafete::Proveedor,
            true,
        )?);
        tenedores.extend(self.leer_tenedores(
            "SELECT id, nombre, motivo FROM registro_ingresos_correo
             WHERE fecha_hora_salida IS NULL AND gafete_numero = ?1
             ORDER BY fecha_hora_ingreso",
            numero,
            ClaseTenedorGafete::Correo,
            true,
        )?);
        tenedores.extend(self.leer_tenedores(
            "SELECT id, encargado_nombre, encargado_codigo_empleado
             FROM prestamos_gafete_provisional
             WHERE fecha_hora_devolucion IS NULL AND gafete_numero = ?1
             ORDER BY fecha_hora_entrega",
            numero,
            ClaseTenedorGafete::Kof,
            true,
        )?);
        Ok(tenedores)
    }

    #[cfg(feature = "nube")]
    fn tenedores_remotos(&self, numero: i64) -> Result<Vec<TenedorGafete>, DatabaseError> {
        let mut tenedores = self.leer_tenedores(
            "SELECT uuid, contratista_nombre, empresa_nombre FROM ingresos_remotos
             WHERE gafete_numero = ?1 ORDER BY hora_entrada",
            numero,
            ClaseTenedorGafete::Contratista,
            false,
        )?;
        tenedores.extend(self.leer_tenedores(
            "SELECT uuid, nombre, empresa_nombre FROM ingresos_proveedor_remotos
             WHERE gafete_numero = ?1 ORDER BY hora_entrada",
            numero,
            ClaseTenedorGafete::Proveedor,
            false,
        )?);
        tenedores.extend(self.leer_tenedores(
            "SELECT uuid, nombre, motivo FROM ingresos_correo_remotos
             WHERE gafete_numero = ?1 ORDER BY hora_entrada",
            numero,
            ClaseTenedorGafete::Correo,
            false,
        )?);
        tenedores.extend(self.leer_tenedores(
            "SELECT uuid, encargado_nombre, encargado_codigo_empleado
             FROM prestamos_gafete_provisional_remotos
             WHERE gafete_numero = ?1 ORDER BY hora_entrega",
            numero,
            ClaseTenedorGafete::Kof,
            false,
        )?);
        Ok(tenedores)
    }

    /// Las tres columnas de cada consulta van siempre en el mismo orden: el
    /// identificador del registro (`id` local o `uuid` remoto), el nombre y el
    /// detalle.
    fn leer_tenedores(
        &self,
        sql: &str,
        numero: i64,
        clase: ClaseTenedorGafete,
        local: bool,
    ) -> Result<Vec<TenedorGafete>, DatabaseError> {
        let mut statement = self.connection.prepare(sql)?;
        let filas = statement
            .query_map(params![numero], |row| {
                let origen = if local {
                    OrigenTenedorGafete::Local { id: row.get(0)? }
                } else {
                    OrigenTenedorGafete::Remoto { uuid: row.get(0)? }
                };
                Ok(TenedorGafete {
                    clase,
                    origen,
                    nombre: row.get(1)?,
                    detalle: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(filas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::empresa_proveedor_repository::{
        EmpresaProveedorRepository, SqliteEmpresaProveedorRepository,
    };
    use crate::database::repositories::encargado_ruta_repository::{
        EncargadoRutaRepository, SqliteEncargadoRutaRepository,
    };
    use crate::database::repositories::gafete_repository::{
        GafeteRepository, SqliteGafeteRepository,
    };
    use crate::database::schema::initialize_database;
    use crate::models::empresa_proveedor::EmpresaProveedor;
    use crate::models::gafete::TipoGafete;
    use crate::models::medio_ingreso::MedioIngreso;
    use crate::services::ingreso_correo_service::DatosIngresoCorreo;
    use crate::tiempo::RelojFijo;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    /// Un equipo (el contratista es PRAIND vigente: los de otros tipos no
    /// llevan gafete) con un gafete número 1 de CADA conjunto (contratista,
    /// proveedor, visita y KOF), una persona de cada clase para ocuparlos y el
    /// operador que los registra.
    fn nucleo_con_un_gafete_de_cada_conjunto() -> (AppCore, UsuarioSesion, i64, i64) {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas(id,nombre) VALUES (1,'Aldama');
                 INSERT INTO usuarios(id,cedula,nombre,password_hash,rol,activo)
                 VALUES (1,'1001','Operador','hash','OPERADOR',1);
                 INSERT INTO contratistas(
                     id,cedula,nombre,empresa_id,tipo_ingreso,es_personal_ruta,tiene_acceso,
                     fecha_vencimiento_praind
                 ) VALUES (1,'101110111','CLAUDIO QUIROS',1,'PRAIND',0,1,'2027-12-31');",
            )
            .unwrap();
        let empresa_proveedor = SqliteEmpresaProveedorRepository::new(&connection)
            .crear(&EmpresaProveedor {
                id: 0,
                nombre: "K-9 Inter".to_string(),
                activo: true,
            })
            .unwrap();
        let encargado = SqliteEncargadoRutaRepository::new(&connection)
            .crear(&crate::models::encargado_ruta::EncargadoRuta {
                id: 0,
                codigo_empleado: "3600482".to_string(),
                nombre: "Abel Turcios".to_string(),
                cedula: None,
                activo: true,
            })
            .unwrap();
        let gafetes = SqliteGafeteRepository::new(&connection);
        for tipo in [
            TipoGafete::Contratista,
            TipoGafete::Proveedor,
            TipoGafete::Visita,
            TipoGafete::ProvisionalKof,
        ] {
            gafetes.crear(1, tipo).unwrap();
        }
        let reloj = Arc::new(RelojFijo::new(
            Utc.with_ymd_and_hms(2026, 10, 17, 12, 0, 0).unwrap(),
        ));
        let sesion = UsuarioSesion {
            id: 1,
            cedula: "1001".to_string(),
            nombre: "Operador".to_string(),
            rol: crate::models::usuario::RolUsuario::Operador,
        };
        (
            AppCore::con_reloj(connection, reloj),
            sesion,
            empresa_proveedor,
            encargado,
        )
    }

    /// Ocupa el gafete 1 de los cuatro conjuntos; devuelve los ids locales en
    /// el orden contratista, proveedor, correo, KOF.
    fn ocupar_el_gafete_1_en_las_cuatro_clases(
        core: &AppCore,
        actor: &UsuarioSesion,
        empresa_proveedor: i64,
        encargado: i64,
    ) -> [i64; 4] {
        let contratista = core
            .registrar_ingreso(actor, 1, MedioIngreso::Caminando, Some(1), None)
            .unwrap()
            .registro_id;
        let proveedor = core
            .registrar_ingreso_proveedor(
                actor,
                "1-1111-1111",
                "Daniel Quintana",
                empresa_proveedor,
                None,
                1,
            )
            .unwrap();
        let correo = core
            .registrar_ingreso_correo(
                actor,
                &DatosIngresoCorreo {
                    cedula: "7-8965-4123",
                    nombre: "Jenna Ortega",
                    motivo: "Entrevista RH",
                    placa: None,
                    gafete_numero: 1,
                },
            )
            .unwrap();
        let kof = core
            .entregar_gafete_provisional(actor, encargado, 1)
            .unwrap();
        [contratista, proveedor, correo, kof]
    }

    #[test]
    fn un_gafete_que_nadie_tiene_no_devuelve_tenedores() {
        let (core, actor, _, _) = nucleo_con_un_gafete_de_cada_conjunto();
        assert!(core.tenedores_del_gafete(&actor, 1).unwrap().is_empty());
    }

    #[test]
    fn el_mismo_numero_en_las_cuatro_clases_devuelve_los_cuatro_tenedores() {
        let (core, actor, empresa_proveedor, encargado) = nucleo_con_un_gafete_de_cada_conjunto();
        let [contratista, proveedor, correo, kof] =
            ocupar_el_gafete_1_en_las_cuatro_clases(&core, &actor, empresa_proveedor, encargado);

        let tenedores = core.tenedores_del_gafete(&actor, 1).unwrap();

        assert_eq!(
            tenedores,
            vec![
                TenedorGafete {
                    clase: ClaseTenedorGafete::Contratista,
                    origen: OrigenTenedorGafete::Local { id: contratista },
                    nombre: "CLAUDIO QUIROS".to_string(),
                    detalle: Some("Aldama".to_string()),
                },
                TenedorGafete {
                    clase: ClaseTenedorGafete::Proveedor,
                    origen: OrigenTenedorGafete::Local { id: proveedor },
                    nombre: "DANIEL QUINTANA".to_string(),
                    detalle: Some("K-9 Inter".to_string()),
                },
                TenedorGafete {
                    clase: ClaseTenedorGafete::Correo,
                    origen: OrigenTenedorGafete::Local { id: correo },
                    nombre: "JENNA ORTEGA".to_string(),
                    detalle: Some("Entrevista RH".to_string()),
                },
                TenedorGafete {
                    clase: ClaseTenedorGafete::Kof,
                    origen: OrigenTenedorGafete::Local { id: kof },
                    nombre: "Abel Turcios".to_string(),
                    detalle: Some("3600482".to_string()),
                },
            ]
        );
    }

    #[test]
    fn otro_numero_no_se_mezcla() {
        let (core, actor, empresa_proveedor, encargado) = nucleo_con_un_gafete_de_cada_conjunto();
        ocupar_el_gafete_1_en_las_cuatro_clases(&core, &actor, empresa_proveedor, encargado);

        assert!(core.tenedores_del_gafete(&actor, 2).unwrap().is_empty());
    }

    #[test]
    fn quien_ya_salio_deja_de_tener_el_gafete() {
        let (core, actor, empresa_proveedor, encargado) = nucleo_con_un_gafete_de_cada_conjunto();
        let [contratista, proveedor, ..] =
            ocupar_el_gafete_1_en_las_cuatro_clases(&core, &actor, empresa_proveedor, encargado);

        core.registrar_salida(&actor, contratista).unwrap();
        core.registrar_salida_proveedor(&actor, proveedor).unwrap();

        let clases: Vec<_> = core
            .tenedores_del_gafete(&actor, 1)
            .unwrap()
            .into_iter()
            .map(|tenedor| tenedor.clase)
            .collect();
        assert_eq!(
            clases,
            vec![ClaseTenedorGafete::Correo, ClaseTenedorGafete::Kof]
        );
    }

    #[cfg(feature = "nube")]
    #[test]
    fn incluye_lo_abierto_por_el_otro_dispositivo_despues_de_lo_local() {
        let (core, actor, empresa_proveedor, encargado) = nucleo_con_un_gafete_de_cada_conjunto();
        let [contratista, ..] =
            ocupar_el_gafete_1_en_las_cuatro_clases(&core, &actor, empresa_proveedor, encargado);
        core.connection
            .execute_batch(
                "INSERT INTO ingresos_remotos (uuid, sitio_id, contratista_nombre, hora_entrada,
                     dispositivo_entrada_id, actualizado_en, empresa_nombre, gafete_numero)
                 VALUES ('r-c', 's1', 'ZAIDA MADRIGAL', '2026-10-17T11:00:00Z', 'otro',
                     '2026-10-17T11:00:00Z', 'BAC', 1);
                 INSERT INTO ingresos_proveedor_remotos (uuid, sitio_id, cedula, nombre,
                     empresa_nombre, placa, gafete_numero, hora_entrada, usuario_entrada_nombre,
                     dispositivo_entrada_id, actualizado_en)
                 VALUES ('r-p', 's1', '2-2222-2222', 'Ana', 'Maika', NULL, 1,
                     '2026-10-17T11:00:00Z', 'Otro', 'otro', '2026-10-17T11:00:00Z');
                 INSERT INTO ingresos_correo_remotos (uuid, sitio_id, cedula, nombre, motivo,
                     placa, gafete_numero, hora_entrada, usuario_entrada_nombre,
                     dispositivo_entrada_id, actualizado_en)
                 VALUES ('r-m', 's1', '3-3333-3333', 'Luis', 'Reunión', NULL, 1,
                     '2026-10-17T11:00:00Z', 'Otro', 'otro', '2026-10-17T11:00:00Z');
                 INSERT INTO prestamos_gafete_provisional_remotos (uuid, sitio_id,
                     encargado_nombre, encargado_codigo_empleado, gafete_numero, hora_entrega,
                     usuario_entrega_nombre, actualizado_en)
                 VALUES ('r-k', 's1', 'Mario', '111', 1, '2026-10-17T11:00:00Z', 'Otro',
                     '2026-10-17T11:00:00Z');
                 INSERT INTO ingresos_remotos (uuid, sitio_id, contratista_nombre, hora_entrada,
                     dispositivo_entrada_id, actualizado_en, gafete_numero)
                 VALUES ('r-otro-numero', 's1', 'NO', '2026-10-17T11:00:00Z', 'otro',
                     '2026-10-17T11:00:00Z', 2);",
            )
            .unwrap();

        let tenedores = core.tenedores_del_gafete(&actor, 1).unwrap();

        // Lo local primero (4) y después lo remoto (4), sin el de otro número.
        assert_eq!(tenedores.len(), 8);
        assert_eq!(
            tenedores[0].origen,
            OrigenTenedorGafete::Local { id: contratista }
        );
        let remotos: Vec<_> = tenedores[4..]
            .iter()
            .map(|tenedor| (tenedor.clase, tenedor.origen.clone()))
            .collect();
        assert_eq!(
            remotos,
            vec![
                (
                    ClaseTenedorGafete::Contratista,
                    OrigenTenedorGafete::Remoto {
                        uuid: "r-c".to_string()
                    }
                ),
                (
                    ClaseTenedorGafete::Proveedor,
                    OrigenTenedorGafete::Remoto {
                        uuid: "r-p".to_string()
                    }
                ),
                (
                    ClaseTenedorGafete::Correo,
                    OrigenTenedorGafete::Remoto {
                        uuid: "r-m".to_string()
                    }
                ),
                (
                    ClaseTenedorGafete::Kof,
                    OrigenTenedorGafete::Remoto {
                        uuid: "r-k".to_string()
                    }
                ),
            ]
        );
    }
}
