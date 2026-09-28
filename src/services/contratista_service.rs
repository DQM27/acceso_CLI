use chrono::{DateTime, NaiveDate, Utc};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::{AuditoriaWriter, EntidadAuditada};
use crate::database::queries::contratistas::{
    ContratistasQuery, FiltroContratistas, PaginaContratistas,
};
use crate::database::repositories::contratista_repository::ContratistaRepository;
use crate::database::repositories::empresa_repository::EmpresaRepository;
use crate::domain::cedula::{Cedula, CedulaInvalida};
use crate::domain::contratista::{admite_personal_ruta, normalizar_nombre, praind_vencido};
use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::fecha_costa_rica;

use super::error::ContratistaServiceError;

pub struct ContratistaConsultaService<'a, Q>
where
    Q: ContratistasQuery + ?Sized,
{
    consultas: &'a Q,
}

impl<'a, Q> ContratistaConsultaService<'a, Q>
where
    Q: ContratistasQuery + ?Sized,
{
    pub fn new(consultas: &'a Q) -> Self {
        Self { consultas }
    }

    pub fn buscar_para_tabla(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<PaginaContratistas, ContratistaServiceError> {
        Ok(self.consultas.buscar(filtro)?)
    }
}

pub struct DatosContratista {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

pub struct DatosActualizacionContratista {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

pub struct ContratistaService<'a, C, E>
where
    C: ContratistaRepository + ?Sized,
    E: EmpresaRepository + ?Sized,
{
    contratistas: &'a C,
    empresas: &'a E,
    /// Fecha de Costa Rica contra la que se decide si un PRAIND está
    /// vencido. `AppCore` pasa la de su reloj (`con_hoy`); `new` usa la del
    /// sistema.
    hoy: NaiveDate,
}

impl<'a, C, E> ContratistaService<'a, C, E>
where
    C: ContratistaRepository + ?Sized,
    E: EmpresaRepository + ?Sized,
{
    pub fn new(contratistas: &'a C, empresas: &'a E) -> Self {
        Self::con_hoy(contratistas, empresas, fecha_costa_rica(Utc::now()))
    }

    pub fn con_hoy(contratistas: &'a C, empresas: &'a E, hoy: NaiveDate) -> Self {
        Self {
            contratistas,
            empresas,
            hoy,
        }
    }

    /// Alta de contratista. Siempre queda con acceso: se registra a quien
    /// está presente y, si corresponde, se le niega después editando
    /// (decisión del dueño 2026-09-27). `datos.tiene_acceso` se ignora.
    /// El resto de las reglas, ver `validar_datos`.
    pub fn crear(&self, datos: DatosContratista) -> Result<i64, ContratistaServiceError> {
        let contratista = self.construir_contratista(
            0,
            DatosContratista {
                tiene_acceso: true,
                ..datos
            },
        )?;
        self.contratistas
            .crear(&contratista)
            .map_err(mapear_cedula_duplicada)
    }

    pub fn buscar_por_id(&self, id: i64) -> Result<Contratista, ContratistaServiceError> {
        self.contratistas
            .buscar_por_id(id)?
            .ok_or(ContratistaServiceError::ContratistaNoEncontrado)
    }

    /// Busca por la forma única de la cédula (ver `domain::cedula`): la
    /// misma persona se encuentra aunque se escriba con guiones o con el
    /// cero del TSE.
    pub fn buscar_por_cedula(&self, cedula: &str) -> Result<Contratista, ContratistaServiceError> {
        let cedula = Cedula::normalizar(cedula)
            .map_err(|_| ContratistaServiceError::ContratistaNoEncontrado)?;
        self.contratistas
            .buscar_por_cedula(cedula.as_str())?
            .ok_or(ContratistaServiceError::ContratistaNoEncontrado)
    }

    pub fn actualizar(
        &self,
        id: i64,
        datos: DatosActualizacionContratista,
    ) -> Result<(), ContratistaServiceError> {
        let actual = self.buscar_por_id(id)?;
        let contratista = self.construir_actualizacion(actual, datos)?;
        self.contratistas
            .actualizar(&contratista)
            .map_err(mapear_cedula_duplicada)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn actualizar_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        datos: DatosActualizacionContratista,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), ContratistaServiceError> {
        let actual = self.buscar_por_id(id)?;
        let cedula_anterior = actual.cedula.clone();
        let nombre_anterior = actual.nombre.clone();
        let empresa_anterior = actual.empresa_id;
        let tipo_anterior = actual.tipo_ingreso.as_str_sql().to_owned();
        let fecha_anterior = actual
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());
        let ruta_anterior = actual.es_personal_ruta;
        let acceso_anterior = actual.tiene_acceso;
        let contratista = self.construir_actualizacion(actual, datos)?;
        let cedula_nueva = contratista.cedula.clone();
        let nombre_nuevo = contratista.nombre.clone();
        let empresa_nueva = contratista.empresa_id;
        let tipo_nuevo = contratista.tipo_ingreso.as_str_sql().to_owned();
        let fecha_nueva = contratista
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());
        let ruta_nueva = contratista.es_personal_ruta;
        let acceso_nuevo = contratista.tiene_acceso;

        self.contratistas
            .actualizar(&contratista)
            .map_err(mapear_cedula_duplicada)?;

        // `entidad_nombre` es el nombre YA actualizado (post-guardado) en
        // cada fila de este lote de cambios — así todas las filas de una
        // misma edición (incluida la que audita el propio cambio de
        // nombre) identifican a la misma persona de la misma forma.
        let registrar = |campo: &str, anterior: Option<&str>, nuevo: Option<&str>| {
            auditoria.registrar_cambio(
                fecha_hora,
                actor_id,
                actor_nombre,
                EntidadAuditada::Contratista,
                id,
                &nombre_nuevo,
                campo,
                anterior,
                nuevo,
            )
        };
        if cedula_anterior != cedula_nueva {
            registrar("cedula", Some(&cedula_anterior), Some(&cedula_nueva))?;
        }
        if nombre_anterior != nombre_nuevo {
            registrar("nombre", Some(&nombre_anterior), Some(&nombre_nuevo))?;
        }
        if empresa_anterior != empresa_nueva {
            // Nombre de la empresa en vez del id crudo — mucho más legible
            // en la auditoría; si por lo que sea ya no se puede resolver
            // (no debería pasar, `empresa_id` es `NOT NULL REFERENCES`),
            // cae al id como texto en vez de fallar el guardado entero.
            let nombre_empresa = |empresa_id: i64| -> String {
                self.empresas
                    .buscar_por_id(empresa_id)
                    .ok()
                    .flatten()
                    .map_or_else(|| empresa_id.to_string(), |empresa| empresa.nombre)
            };
            registrar(
                "empresa_id",
                Some(&nombre_empresa(empresa_anterior)),
                Some(&nombre_empresa(empresa_nueva)),
            )?;
        }
        if tipo_anterior != tipo_nuevo {
            registrar("tipo_ingreso", Some(&tipo_anterior), Some(&tipo_nuevo))?;
        }
        if fecha_anterior != fecha_nueva {
            registrar(
                "fecha_vencimiento_praind",
                fecha_anterior.as_deref(),
                fecha_nueva.as_deref(),
            )?;
        }
        if ruta_anterior != ruta_nueva {
            registrar(
                "es_personal_ruta",
                Some(texto_si_no(ruta_anterior)),
                Some(texto_si_no(ruta_nueva)),
            )?;
        }
        if acceso_anterior != acceso_nuevo {
            registrar(
                "tiene_acceso",
                Some(texto_estado_acceso(acceso_anterior)),
                Some(texto_estado_acceso(acceso_nuevo)),
            )?;
        }
        Ok(())
    }

    pub fn listar(&self) -> Result<Vec<Contratista>, ContratistaServiceError> {
        Ok(self.contratistas.listar()?)
    }

    fn construir_contratista(
        &self,
        id: i64,
        datos: DatosContratista,
    ) -> Result<Contratista, ContratistaServiceError> {
        self.armar(
            id,
            None,
            &datos.cedula,
            &datos.nombre,
            datos.empresa_id,
            datos.tipo_ingreso,
            datos.fecha_vencimiento_praind,
            datos.es_personal_ruta,
            datos.tiene_acceso,
        )
    }

    fn construir_actualizacion(
        &self,
        actual: Contratista,
        datos: DatosActualizacionContratista,
    ) -> Result<Contratista, ContratistaServiceError> {
        self.armar(
            actual.id,
            Some(&actual),
            &datos.cedula,
            &datos.nombre,
            datos.empresa_id,
            datos.tipo_ingreso,
            datos.fecha_vencimiento_praind,
            datos.es_personal_ruta,
            datos.tiene_acceso,
        )
    }

    /// Todas las reglas del contratista, iguales al crear y al editar y
    /// para cualquier interfaz (escritorio, móvil):
    /// - cédula obligatoria, sólo dígitos;
    /// - nombre obligatorio, sólo letras, guardado en mayúsculas;
    /// - la empresa existe;
    /// - personal de ruta sólo para PRAIND e IN HOUSE;
    /// - PRAIND obligatorio si el tipo lo requiere, y vigente.
    ///
    /// Al editar (`anterior` presente), "vigente" y "personal de ruta" sólo
    /// se revisan si cambió lo que deciden (fecha, tipo o la casilla): si
    /// no, a alguien con el PRAIND ya vencido no se le podría ni quitar el
    /// acceso ni corregir el nombre.
    #[allow(clippy::too_many_arguments)]
    fn armar(
        &self,
        id: i64,
        anterior: Option<&Contratista>,
        cedula: &str,
        nombre: &str,
        empresa_id: i64,
        tipo_ingreso: TipoIngreso,
        fecha_vencimiento_praind: Option<NaiveDate>,
        es_personal_ruta: bool,
        tiene_acceso: bool,
    ) -> Result<Contratista, ContratistaServiceError> {
        let cedula = match Cedula::normalizar(cedula) {
            Ok(cedula) if cedula.es_nacional_o_de_extranjero() => cedula,
            Err(CedulaInvalida::Vacia) => return Err(ContratistaServiceError::CedulaVacia),
            Ok(_) | Err(_) => return Err(ContratistaServiceError::CedulaInvalida),
        };
        let cedula = cedula.as_str();

        if nombre.trim().is_empty() {
            return Err(ContratistaServiceError::NombreVacio);
        }
        let nombre = normalizar_nombre(nombre).ok_or(ContratistaServiceError::NombreInvalido)?;

        let empresa = self
            .empresas
            .buscar_por_id(empresa_id)?
            .ok_or(ContratistaServiceError::EmpresaNoEncontrada)?;

        let cambia_tipo_o_ruta = anterior.is_none_or(|previo| {
            previo.tipo_ingreso != tipo_ingreso || previo.es_personal_ruta != es_personal_ruta
        });
        let cambia_praind = cambia_tipo_o_ruta
            || anterior
                .is_none_or(|previo| previo.fecha_vencimiento_praind != fecha_vencimiento_praind);

        if cambia_tipo_o_ruta && es_personal_ruta && !admite_personal_ruta(tipo_ingreso) {
            return Err(ContratistaServiceError::PersonalRutaNoAdmitido);
        }

        let contratista = Contratista::nuevo(
            id,
            cedula.to_string(),
            nombre,
            &empresa,
            tipo_ingreso,
            fecha_vencimiento_praind,
            es_personal_ruta,
            tiene_acceso,
        );

        if contratista.requiere_praind() {
            if contratista.fecha_vencimiento_praind.is_none() {
                return Err(ContratistaServiceError::PraindRequerido);
            }
            if cambia_praind && praind_vencido(contratista.fecha_vencimiento_praind, self.hoy) {
                return Err(ContratistaServiceError::PraindVencido);
            }
        }

        Ok(contratista)
    }
}

fn texto_estado_acceso(tiene_acceso: bool) -> &'static str {
    if tiene_acceso {
        "HABILITADO"
    } else {
        "DESHABILITADO"
    }
}

fn texto_si_no(valor: bool) -> &'static str {
    if valor { "SI" } else { "NO" }
}

fn mapear_cedula_duplicada(error: DatabaseError) -> ContratistaServiceError {
    if error.es_constraint_unique() {
        ContratistaServiceError::CedulaDuplicada
    } else {
        ContratistaServiceError::Database(error)
    }
}
