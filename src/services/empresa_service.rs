use crate::database::error::DatabaseError;
use crate::database::queries::empresas::{EmpresaResumen, EmpresasQuery, FiltroEmpresas};
use crate::database::repositories::empresa_repository::EmpresaRepository;
use crate::models::empresa::Empresa;

use super::error::EmpresaServiceError;

pub struct EmpresaService<'a, R>
where
    R: EmpresaRepository + ?Sized,
{
    empresas: &'a R,
}

pub struct EmpresaConsultaService<'a, Q: EmpresasQuery + ?Sized> {
    query: &'a Q,
}

impl<'a, Q: EmpresasQuery + ?Sized> EmpresaConsultaService<'a, Q> {
    pub fn new(query: &'a Q) -> Self {
        Self { query }
    }

    pub fn buscar_para_tabla(
        &self,
        filtro: &FiltroEmpresas,
    ) -> Result<Vec<EmpresaResumen>, EmpresaServiceError> {
        Ok(self.query.buscar(filtro)?)
    }
}

impl<'a, R> EmpresaService<'a, R>
where
    R: EmpresaRepository + ?Sized,
{
    pub fn new(empresas: &'a R) -> Self {
        Self { empresas }
    }

    pub fn crear(&self, nombre: &str) -> Result<i64, EmpresaServiceError> {
        let nombre = normalizar_nombre(nombre)?;
        let empresa = Empresa {
            id: 0,
            nombre: nombre.to_string(),
            activo: true,
        };

        self.empresas
            .crear(&empresa)
            .map_err(mapear_nombre_duplicado)
    }

    pub fn buscar_por_id(&self, id: i64) -> Result<Empresa, EmpresaServiceError> {
        self.empresas
            .buscar_por_id(id)?
            .ok_or(EmpresaServiceError::EmpresaNoEncontrada)
    }

    pub fn buscar_por_nombre(&self, nombre: &str) -> Result<Empresa, EmpresaServiceError> {
        let nombre = nombre.trim();

        self.empresas
            .buscar_por_nombre(nombre)?
            .ok_or(EmpresaServiceError::EmpresaNoEncontrada)
    }

    pub fn actualizar(&self, id: i64, nombre: &str) -> Result<(), EmpresaServiceError> {
        let nombre = normalizar_nombre(nombre)?;
        let actual = self.buscar_por_id(id)?;

        let empresa = Empresa {
            id,
            nombre: nombre.to_string(),
            activo: actual.activo,
        };

        self.empresas
            .actualizar(&empresa)
            .map_err(mapear_nombre_duplicado)
    }

    pub fn activar(&self, id: i64) -> Result<(), EmpresaServiceError> {
        self.buscar_por_id(id)?;
        Ok(self.empresas.establecer_activo(id, true)?)
    }

    pub fn desactivar(&self, id: i64) -> Result<(), EmpresaServiceError> {
        self.buscar_por_id(id)?;
        Ok(self.empresas.establecer_activo(id, false)?)
    }

    pub fn listar(&self) -> Result<Vec<Empresa>, EmpresaServiceError> {
        Ok(self.empresas.listar()?)
    }
}

fn mapear_nombre_duplicado(error: DatabaseError) -> EmpresaServiceError {
    if error.es_constraint_unique() {
        EmpresaServiceError::NombreDuplicado
    } else {
        EmpresaServiceError::Database(error)
    }
}

fn normalizar_nombre(nombre: &str) -> Result<&str, EmpresaServiceError> {
    let nombre = nombre.trim();

    if nombre.is_empty() {
        return Err(EmpresaServiceError::NombreEmpresaVacio);
    }

    Ok(nombre)
}
