use chrono::NaiveDate;

use crate::database::error::DatabaseError;
use crate::database::queries::contratistas::{
    ContratistaResumen, ContratistasQuery, FiltroContratistas,
};
use crate::database::repositories::contratista_repository::ContratistaRepository;
use crate::database::repositories::empresa_repository::EmpresaRepository;
use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;

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
    ) -> Result<Vec<ContratistaResumen>, ContratistaServiceError> {
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
}

impl<'a, C, E> ContratistaService<'a, C, E>
where
    C: ContratistaRepository + ?Sized,
    E: EmpresaRepository + ?Sized,
{
    pub fn new(contratistas: &'a C, empresas: &'a E) -> Self {
        Self {
            contratistas,
            empresas,
        }
    }

    pub fn crear(&self, datos: DatosContratista) -> Result<i64, ContratistaServiceError> {
        let contratista = self.construir_contratista(0, datos)?;
        self.contratistas
            .crear(&contratista)
            .map_err(mapear_cedula_duplicada)
    }

    pub fn buscar_por_id(&self, id: i64) -> Result<Contratista, ContratistaServiceError> {
        self.contratistas
            .buscar_por_id(id)?
            .ok_or(ContratistaServiceError::ContratistaNoEncontrado)
    }

    pub fn buscar_por_cedula(&self, cedula: &str) -> Result<Contratista, ContratistaServiceError> {
        self.contratistas
            .buscar_por_cedula(cedula.trim())?
            .ok_or(ContratistaServiceError::ContratistaNoEncontrado)
    }

    pub fn actualizar(
        &self,
        id: i64,
        datos: DatosActualizacionContratista,
    ) -> Result<(), ContratistaServiceError> {
        let actual = self.buscar_por_id(id)?;
        let contratista = self.construir_actualizacion(actual, datos)?;
        Ok(self.contratistas.actualizar(&contratista)?)
    }

    pub fn listar(&self) -> Result<Vec<Contratista>, ContratistaServiceError> {
        Ok(self.contratistas.listar()?)
    }

    fn construir_contratista(
        &self,
        id: i64,
        datos: DatosContratista,
    ) -> Result<Contratista, ContratistaServiceError> {
        let cedula = datos.cedula.trim();
        if cedula.is_empty() {
            return Err(ContratistaServiceError::CedulaVacia);
        }

        let nombre = datos.nombre.trim();
        if nombre.is_empty() {
            return Err(ContratistaServiceError::NombreVacio);
        }

        if self.empresas.buscar_por_id(datos.empresa_id)?.is_none() {
            return Err(ContratistaServiceError::EmpresaNoEncontrada);
        }

        let contratista = Contratista {
            id,
            cedula: cedula.to_string(),
            nombre: nombre.to_string(),
            empresa_id: datos.empresa_id,
            tipo_ingreso: datos.tipo_ingreso,
            fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
            es_personal_ruta: datos.es_personal_ruta,
            tiene_acceso: datos.tiene_acceso,
        };

        if contratista.requiere_praind() && contratista.fecha_vencimiento_praind.is_none() {
            return Err(ContratistaServiceError::PraindRequerido);
        }

        Ok(contratista)
    }

    fn construir_actualizacion(
        &self,
        actual: Contratista,
        datos: DatosActualizacionContratista,
    ) -> Result<Contratista, ContratistaServiceError> {
        let nombre = datos.nombre.trim();
        if nombre.is_empty() {
            return Err(ContratistaServiceError::NombreVacio);
        }

        if self.empresas.buscar_por_id(datos.empresa_id)?.is_none() {
            return Err(ContratistaServiceError::EmpresaNoEncontrada);
        }

        let contratista = Contratista {
            id: actual.id,
            cedula: actual.cedula,
            nombre: nombre.to_string(),
            empresa_id: datos.empresa_id,
            tipo_ingreso: datos.tipo_ingreso,
            fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
            es_personal_ruta: datos.es_personal_ruta,
            tiene_acceso: datos.tiene_acceso,
        };

        if contratista.requiere_praind() && contratista.fecha_vencimiento_praind.is_none() {
            return Err(ContratistaServiceError::PraindRequerido);
        }

        Ok(contratista)
    }
}

fn mapear_cedula_duplicada(error: DatabaseError) -> ContratistaServiceError {
    if matches!(
        &error,
        DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(codigo, _))
            if codigo.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    ) {
        ContratistaServiceError::CedulaDuplicada
    } else {
        ContratistaServiceError::Database(error)
    }
}
