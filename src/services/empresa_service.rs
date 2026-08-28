use chrono::{DateTime, Utc};

use crate::database::error::DatabaseError;
use crate::database::queries::auditoria::{AuditoriaWriter, EntidadAuditada};
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

    #[allow(clippy::too_many_arguments)]
    pub fn actualizar_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        nombre: &str,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), EmpresaServiceError> {
        let nombre = normalizar_nombre(nombre)?;
        let actual = self.buscar_por_id(id)?;
        let nombre_anterior = actual.nombre.clone();
        let empresa = Empresa {
            id,
            nombre: nombre.to_string(),
            activo: actual.activo,
        };
        self.empresas
            .actualizar(&empresa)
            .map_err(mapear_nombre_duplicado)?;
        if nombre_anterior != empresa.nombre {
            auditoria.registrar_cambio(
                fecha_hora,
                actor_id,
                actor_nombre,
                EntidadAuditada::Empresa,
                id,
                &empresa.nombre,
                "nombre",
                Some(&nombre_anterior),
                Some(&empresa.nombre),
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn establecer_activo_auditado<A: AuditoriaWriter + ?Sized>(
        &self,
        id: i64,
        activo: bool,
        actor_id: i64,
        actor_nombre: &str,
        fecha_hora: DateTime<Utc>,
        auditoria: &A,
    ) -> Result<(), EmpresaServiceError> {
        let actual = self.buscar_por_id(id)?;
        self.empresas.establecer_activo(id, activo)?;
        if actual.activo != activo {
            auditoria.registrar_cambio(
                fecha_hora,
                actor_id,
                actor_nombre,
                EntidadAuditada::Empresa,
                id,
                &actual.nombre,
                "activo",
                Some(texto_si_no(actual.activo)),
                Some(texto_si_no(activo)),
            )?;
        }
        Ok(())
    }
}

fn texto_si_no(valor: bool) -> &'static str {
    if valor { "SI" } else { "NO" }
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
