use crate::database::repositories::empresa_repository::EmpresaRepository;
use crate::models::empresa::Empresa;

use super::error::EmpresaServiceError;

pub struct EmpresaService<'a, R>
where
    R: EmpresaRepository + ?Sized,
{
    empresas: &'a R,
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
        };

        Ok(self.empresas.crear(&empresa)?)
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
        self.buscar_por_id(id)?;

        let empresa = Empresa {
            id,
            nombre: nombre.to_string(),
        };

        Ok(self.empresas.actualizar(&empresa)?)
    }

    pub fn listar(&self) -> Result<Vec<Empresa>, EmpresaServiceError> {
        Ok(self.empresas.listar()?)
    }
}

fn normalizar_nombre(nombre: &str) -> Result<&str, EmpresaServiceError> {
    let nombre = nombre.trim();

    if nombre.is_empty() {
        return Err(EmpresaServiceError::NombreEmpresaVacio);
    }

    Ok(nombre)
}
