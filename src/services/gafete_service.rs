//! Orquestación del catálogo de gafetes: valida contra las reglas de
//! `domain::gafete`, delega en los repositorios y deja cada reporte
//! (perdido, pagado, recuperado) en el historial a nombre de quien lo hizo.

use chrono::{DateTime, Utc};

use crate::application::puertos::gafetes::GafeteRepository;
use crate::application::puertos::registros_ingreso::RegistroIngresoRepository;
use crate::database::queries::gafetes::{FiltroGafetes, GafeteResumen, GafetesQuery};
use crate::database::queries::gafetes_incidentes::GafetesIncidentesWriter;
use crate::domain::gafete::{
    normalizar_detalle, puede_recuperarse, puede_registrarse_pago, puede_reportarse_perdido,
};
use crate::models::gafete::{Gafete, PortadorGafete, TipoGafete};

use super::error::GafeteServiceError;

pub struct GafeteConsultaService<'a, Q: GafetesQuery + ?Sized> {
    query: &'a Q,
}

impl<'a, Q: GafetesQuery + ?Sized> GafeteConsultaService<'a, Q> {
    pub fn new(query: &'a Q) -> Self {
        Self { query }
    }

    pub fn buscar(&self, filtro: &FiltroGafetes) -> Result<Vec<GafeteResumen>, GafeteServiceError> {
        Ok(self.query.buscar(filtro)?)
    }
}

/// Quién hace un reporte y cuándo: todo reporte queda a su nombre.
#[derive(Debug, Clone, Copy)]
pub struct AutorReporte {
    pub usuario_id: i64,
    pub ahora: DateTime<Utc>,
}

/// Compartido entre todos los tipos de gafete -- por diseño no conoce
/// `Contratista`/`CitaVisitante`: validar que el `portador` exista es
/// responsabilidad de quien llama (`AppCore`), que sí puede conocer todos
/// los tipos sin que este servicio tenga que hacerlo (mismo patrón
/// "hub-and-spoke" documentado en
/// `docs/features-futuras/plan-gafetes-compartido-y-realtime-citas.md`).
pub struct GafeteService<'a, R: GafeteRepository + ?Sized> {
    gafetes: &'a R,
}

impl<'a, R: GafeteRepository + ?Sized> GafeteService<'a, R> {
    pub fn new(gafetes: &'a R) -> Self {
        Self { gafetes }
    }

    fn buscar_por_id(&self, id: i64) -> Result<Gafete, GafeteServiceError> {
        self.gafetes
            .buscar_por_id(id)?
            .ok_or(GafeteServiceError::GafeteNoEncontrado)
    }

    pub fn crear_uno(&self, numero: i64, tipo: TipoGafete) -> Result<i64, GafeteServiceError> {
        if numero <= 0 {
            return Err(GafeteServiceError::NumeroInvalido);
        }
        self.gafetes.crear(numero, tipo).map_err(|error| {
            if error.es_constraint_unique() {
                GafeteServiceError::NumeroDuplicado
            } else {
                GafeteServiceError::Database(error)
            }
        })
    }

    /// Si un número del rango falla (típicamente duplicado), el rango
    /// completo aborta sin alta parcial — no hace falta deshacer nada acá:
    /// el llamador (`AppCore::crear_gafetes_rango`) sólo comitea la
    /// transacción cuando esta función devuelve `Ok`.
    pub fn crear_rango(
        &self,
        desde: i64,
        hasta: i64,
        tipo: TipoGafete,
    ) -> Result<Vec<i64>, GafeteServiceError> {
        if desde <= 0 || hasta < desde {
            return Err(GafeteServiceError::RangoInvalido);
        }
        (desde..=hasta)
            .map(|numero| self.crear_uno(numero, tipo))
            .collect()
    }

    /// `Disponible -> Perdido`. Con `portador`, esa persona queda marcada;
    /// sin él (no se sabe a quién se le dio), el detalle es obligatorio:
    /// quien reporta tiene que contar qué pasó. No valida que `portador`
    /// exista -- a propósito, ver el doc-comment del servicio.
    pub fn reportar_perdido<
        W: GafetesIncidentesWriter + ?Sized,
        I: RegistroIngresoRepository + ?Sized,
    >(
        &self,
        incidentes: &W,
        registros: &I,
        id: i64,
        portador: Option<PortadorGafete>,
        detalle: Option<&str>,
        autor: AutorReporte,
    ) -> Result<(), GafeteServiceError> {
        let detalle =
            normalizar_detalle(detalle).map_err(|_| GafeteServiceError::DetalleMuyLargo)?;
        if portador.is_none() && detalle.is_none() {
            return Err(GafeteServiceError::DetalleRequerido);
        }
        let gafete = self.buscar_por_id(id)?;
        if !puede_reportarse_perdido(gafete.estado) {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        if registros
            .buscar_ingreso_activo_por_gafete(gafete.numero)?
            .is_some()
        {
            return Err(GafeteServiceError::GafeteConIngresoActivo);
        }
        self.gafetes.marcar_perdido(id, portador)?;
        incidentes.registrar_perdido(
            id,
            autor.ahora,
            autor.usuario_id,
            portador,
            detalle.as_deref(),
        )?;
        Ok(())
    }

    /// El responsable pagó: se le quita la marca y el gafete sigue
    /// `Perdido` (no se puede asignar) hasta que se reimprima.
    pub fn registrar_pago<W: GafetesIncidentesWriter + ?Sized>(
        &self,
        incidentes: &W,
        id: i64,
        detalle: Option<&str>,
        autor: AutorReporte,
    ) -> Result<(), GafeteServiceError> {
        let detalle =
            normalizar_detalle(detalle).map_err(|_| GafeteServiceError::DetalleMuyLargo)?;
        let gafete = self.buscar_por_id(id)?;
        if !puede_registrarse_pago(&gafete) {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        let Some(portador) = gafete.portador() else {
            return Err(GafeteServiceError::EstadoInvalido);
        };
        self.gafetes.quitar_responsable(id)?;
        incidentes.registrar_pagado(
            id,
            autor.ahora,
            autor.usuario_id,
            portador,
            detalle.as_deref(),
        )?;
        Ok(())
    }

    /// `Perdido -> Disponible`: apareció o se reimprimió. El detalle es
    /// obligatorio (quién lo encontró, dónde, o que llegó reimpreso). Si
    /// todavía tenía responsable, se le quita la marca: la empresa nunca
    /// reimprime antes de que pague, así que si vuelve, ya no debe nada.
    pub fn recuperar<W: GafetesIncidentesWriter + ?Sized>(
        &self,
        incidentes: &W,
        id: i64,
        detalle: Option<&str>,
        autor: AutorReporte,
    ) -> Result<(), GafeteServiceError> {
        let detalle = normalizar_detalle(detalle)
            .map_err(|_| GafeteServiceError::DetalleMuyLargo)?
            .ok_or(GafeteServiceError::DetalleRequerido)?;
        let gafete = self.buscar_por_id(id)?;
        if !puede_recuperarse(gafete.estado) {
            return Err(GafeteServiceError::EstadoInvalido);
        }
        self.gafetes.recuperar(id)?;
        incidentes.registrar_recuperado(id, autor.ahora, autor.usuario_id, &detalle)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::queries::gafetes_incidentes::{
        GafetesIncidentesQuery, SqliteGafetesIncidentes,
    };
    use crate::database::repositories::gafete_repository::SqliteGafeteRepository;
    use crate::database::repositories::registro_ingreso_repository::SqliteRegistroIngresoRepository;
    use crate::database::schema::initialize_database;
    use crate::models::gafete::{EstadoGafete, TipoIncidenteGafete};
    use crate::models::registro_ingreso::NuevoRegistroIngreso;
    use crate::models::tipo_ingreso::TipoIngreso;
    use rusqlite::Connection;

    fn conexion_con_contratista() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Acme');
                 INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('9', 'Root', 'hash', 'ROOT', 1);",
            )
            .unwrap();
        (connection, 1)
    }

    fn autor() -> AutorReporte {
        AutorReporte {
            usuario_id: 1,
            ahora: Utc::now(),
        }
    }

    fn abrir_ingreso_con_gafete(connection: &Connection, contratista_id: i64, gafete_numero: i64) {
        let registros = SqliteRegistroIngresoRepository::new(connection);
        registros
            .crear(&NuevoRegistroIngreso {
                contratista_id,
                empresa_id: 1,
                fecha_hora_ingreso: Utc::now(),
                medio_ingreso: crate::models::medio_ingreso::MedioIngreso::Caminando,
                tipo_ingreso: TipoIngreso::Praind,
                gafete_numero: Some(gafete_numero),
                placa: None,
                usuario_ingreso_id: 1,
                datos_historicos: crate::models::registro_ingreso::DatosHistoricosEntrada {
                    contratista_cedula: "1".to_owned(),
                    contratista_nombre: "Juan".to_owned(),
                    fecha_vencimiento_praind: None,
                    es_personal_ruta: false,
                    tiene_acceso: true,
                    empresa_activa: true,
                    resultado_acceso:
                        crate::models::registro_ingreso::ResultadoIngresoRegistrado::Permitido,
                    reglas_version: crate::domain::acceso::VERSION_REGLAS_ACCESO,
                },
            })
            .unwrap();
    }

    #[test]
    fn numero_cero_o_negativo_es_invalido() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);

        assert!(matches!(
            servicio.crear_uno(0, TipoGafete::Contratista),
            Err(GafeteServiceError::NumeroInvalido)
        ));
        assert!(matches!(
            servicio.crear_uno(-1, TipoGafete::Contratista),
            Err(GafeteServiceError::NumeroInvalido)
        ));
    }

    #[test]
    fn rango_invalido_no_crea_nada() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);

        assert!(matches!(
            servicio.crear_rango(5, 3, TipoGafete::Contratista),
            Err(GafeteServiceError::RangoInvalido)
        ));
    }

    #[test]
    fn rango_con_un_numero_ya_existente_aborta_completo() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        servicio.crear_uno(3, TipoGafete::Contratista).unwrap();

        let resultado = servicio.crear_rango(1, 5, TipoGafete::Contratista);

        assert!(matches!(
            resultado,
            Err(GafeteServiceError::NumeroDuplicado)
        ));
    }

    #[test]
    fn mismo_numero_en_tipo_distinto_no_colisiona() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        servicio.crear_uno(7, TipoGafete::Contratista).unwrap();

        assert!(servicio.crear_uno(7, TipoGafete::Visita).is_ok());
    }

    /// El caso del pago: la persona queda limpia pero el gafete sigue sin
    /// poderse asignar hasta que llegue reimpreso.
    #[test]
    fn pagar_quita_la_marca_pero_el_gafete_sigue_perdido_hasta_recuperarlo() {
        let (connection, contratista_id) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let registros = SqliteRegistroIngresoRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();

        servicio
            .reportar_perdido(
                &incidentes,
                &registros,
                id,
                Some(PortadorGafete::Contratista(contratista_id)),
                None,
                autor(),
            )
            .unwrap();
        assert_eq!(
            gafetes.deuda_de_contratista(contratista_id).unwrap(),
            vec![1]
        );

        servicio
            .registrar_pago(&incidentes, id, Some("Recibo 123"), autor())
            .unwrap();
        let pagado = gafetes.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(pagado.estado, EstadoGafete::Perdido);
        assert_eq!(pagado.portador(), None);
        assert!(
            gafetes
                .deuda_de_contratista(contratista_id)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            servicio.registrar_pago(&incidentes, id, None, autor()),
            Err(GafeteServiceError::EstadoInvalido)
        ));

        servicio
            .recuperar(&incidentes, id, Some("Llegó reimpreso"), autor())
            .unwrap();
        let recuperado = gafetes.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(recuperado.estado, EstadoGafete::Disponible);

        let tipos: Vec<_> = incidentes
            .historial(id)
            .unwrap()
            .into_iter()
            .map(|incidente| incidente.tipo)
            .collect();
        assert_eq!(
            tipos,
            vec![
                TipoIncidenteGafete::Recuperado,
                TipoIncidenteGafete::Pagado,
                TipoIncidenteGafete::Perdido
            ]
        );
    }

    #[test]
    fn perdido_sin_responsable_exige_detalle() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let registros = SqliteRegistroIngresoRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();

        assert!(matches!(
            servicio.reportar_perdido(&incidentes, &registros, id, None, Some("   "), autor()),
            Err(GafeteServiceError::DetalleRequerido)
        ));

        servicio
            .reportar_perdido(
                &incidentes,
                &registros,
                id,
                None,
                Some("Recibí el turno sin el 01, me entregó Pedro"),
                autor(),
            )
            .unwrap();
        let perdido = gafetes.buscar_por_id(id).unwrap().unwrap();
        assert_eq!(perdido.estado, EstadoGafete::Perdido);
        assert_eq!(perdido.portador(), None);
        // Sin responsable no hay pago que registrar.
        assert!(matches!(
            servicio.registrar_pago(&incidentes, id, None, autor()),
            Err(GafeteServiceError::EstadoInvalido)
        ));
        let historial = incidentes.historial(id).unwrap();
        assert_eq!(
            historial[0].detalle.as_deref(),
            Some("Recibí el turno sin el 01, me entregó Pedro")
        );
    }

    #[test]
    fn recuperar_exige_detalle_y_estado_perdido() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();

        assert!(matches!(
            servicio.recuperar(&incidentes, id, Some("apareció"), autor()),
            Err(GafeteServiceError::EstadoInvalido)
        ));
        assert!(matches!(
            servicio.recuperar(&incidentes, id, None, autor()),
            Err(GafeteServiceError::DetalleRequerido)
        ));
    }

    /// La empresa nunca reimprime antes del pago: si el gafete vuelve con
    /// responsable todavía marcado, apareció, y la marca se quita igual.
    #[test]
    fn recuperar_con_responsable_le_quita_la_marca() {
        let (connection, contratista_id) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let registros = SqliteRegistroIngresoRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();
        servicio
            .reportar_perdido(
                &incidentes,
                &registros,
                id,
                Some(PortadorGafete::Contratista(contratista_id)),
                None,
                autor(),
            )
            .unwrap();

        servicio
            .recuperar(&incidentes, id, Some("Lo trajo él mismo"), autor())
            .unwrap();

        assert!(
            gafetes
                .deuda_de_contratista(contratista_id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reportar_perdido_con_ingreso_activo_se_bloquea() {
        let (connection, contratista_id) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let registros = SqliteRegistroIngresoRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();
        abrir_ingreso_con_gafete(&connection, contratista_id, 1);

        assert!(matches!(
            servicio.reportar_perdido(
                &incidentes,
                &registros,
                id,
                Some(PortadorGafete::Contratista(contratista_id)),
                None,
                autor()
            ),
            Err(GafeteServiceError::GafeteConIngresoActivo)
        ));
    }

    #[test]
    fn reportar_perdido_dos_veces_falla() {
        let (connection, _) = conexion_con_contratista();
        let gafetes = SqliteGafeteRepository::new(&connection);
        let incidentes = SqliteGafetesIncidentes::new(&connection);
        let registros = SqliteRegistroIngresoRepository::new(&connection);
        let servicio = GafeteService::new(&gafetes);
        let id = servicio.crear_uno(1, TipoGafete::Contratista).unwrap();

        servicio
            .reportar_perdido(&incidentes, &registros, id, None, Some("no está"), autor())
            .unwrap();
        assert!(matches!(
            servicio.reportar_perdido(&incidentes, &registros, id, None, Some("no está"), autor()),
            Err(GafeteServiceError::EstadoInvalido)
        ));
    }
}
