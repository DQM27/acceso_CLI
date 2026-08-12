use chrono::NaiveDateTime;

use crate::database::repositories::contratista_repository::ContratistaRepository;
use crate::database::repositories::registro_ingreso_repository::RegistroIngresoRepository;
use crate::domain::acceso::verificar_acceso;
use crate::domain::resultado_acceso::ResultadoAcceso;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::RegistroIngreso;

use super::error::RegistroIngresoServiceError;

pub struct RegistroIngresoService<'a, C, R>
where
    C: ContratistaRepository + ?Sized,
    R: RegistroIngresoRepository + ?Sized,
{
    contratistas: &'a C,
    registros: &'a R,
}

impl<'a, C, R> RegistroIngresoService<'a, C, R>
where
    C: ContratistaRepository + ?Sized,
    R: RegistroIngresoRepository + ?Sized,
{
    pub fn new(contratistas: &'a C, registros: &'a R) -> Self {
        Self {
            contratistas,
            registros,
        }
    }

    pub fn registrar_entrada(
        &self,
        contratista_id: i64,
        medio_ingreso: MedioIngreso,
        gafete_numero: Option<i64>,
        usuario_ingreso_id: i64,
        fecha_hora_ingreso: NaiveDateTime,
    ) -> Result<i64, RegistroIngresoServiceError> {
        let contratista = self
            .contratistas
            .buscar_por_id(contratista_id)?
            .ok_or(RegistroIngresoServiceError::ContratistaNoEncontrado)?;

        if let ResultadoAcceso::Denegado(motivo) =
            verificar_acceso(&contratista, fecha_hora_ingreso.date())
        {
            return Err(RegistroIngresoServiceError::AccesoDenegado(motivo));
        }

        if self
            .registros
            .buscar_ingreso_activo(contratista.id)?
            .is_some()
        {
            return Err(RegistroIngresoServiceError::IngresoActivo);
        }

        let gafete_numero = if contratista.requiere_gafete() {
            let numero = gafete_numero.ok_or(RegistroIngresoServiceError::GafeteRequerido)?;

            if self
                .registros
                .buscar_ingreso_activo_por_gafete(numero)?
                .is_some()
            {
                return Err(RegistroIngresoServiceError::GafeteOcupado);
            }

            Some(numero)
        } else {
            None
        };

        let registro = RegistroIngreso {
            id: 0,
            contratista_id: contratista.id,
            empresa_id: contratista.empresa_id,
            fecha_hora_ingreso,
            medio_ingreso,
            tipo_ingreso: contratista.tipo_ingreso,
            gafete_numero,
            usuario_ingreso_id,
            fecha_hora_salida: None,
            usuario_salida_id: None,
        };

        Ok(self.registros.crear(&registro)?)
    }

    pub fn registrar_salida(
        &self,
        registro_id: i64,
        fecha_hora_salida: NaiveDateTime,
        usuario_salida_id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        Ok(self
            .registros
            .registrar_salida(registro_id, fecha_hora_salida, usuario_salida_id)?)
    }

    pub fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<RegistroIngreso, RegistroIngresoServiceError> {
        self.registros
            .buscar_ingreso_activo_por_gafete(gafete_numero)?
            .ok_or(RegistroIngresoServiceError::GafeteNoAsignado)
    }

    pub fn registrar_salida_por_gafete(
        &self,
        gafete_numero: i64,
        fecha_hora_salida: NaiveDateTime,
        usuario_salida_id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        let registro = self.buscar_ingreso_activo_por_gafete(gafete_numero)?;
        self.registrar_salida(registro.id, fecha_hora_salida, usuario_salida_id)
    }
}
