use chrono::{DateTime, NaiveDate, Utc};

use crate::database::error::DatabaseError;
use crate::database::queries::ingresos::{
    FiltroHistorial, FiltroIngresosActivos, IngresoActivoLectura, IngresosQuery, PaginaHistorial,
};
use crate::database::repositories::contratista_repository::ContratistaRepository;
use crate::database::repositories::empresa_repository::EmpresaRepository;
use crate::database::repositories::gafete_repository::GafeteRepository;
use crate::database::repositories::registro_ingreso_repository::RegistroIngresoRepository;
use crate::domain::acceso::verificar_acceso;
use crate::domain::registro_ingreso::salida_es_cronologicamente_valida;
use crate::domain::resultado_acceso::ResultadoAcceso;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{
    DatosHistoricosEntrada, MotivoResultadoIngreso, NuevoRegistroIngreso, RegistroIngreso,
    ResultadoIngresoRegistrado, VERSION_REGLAS_ACCESO,
};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::fecha_costa_rica;

use super::error::RegistroIngresoServiceError;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ResultadoRegistroEntrada {
    pub registro_id: i64,
    pub resultado_acceso: ResultadoAcceso,
}

/// Vista previa del estado actual de un contratista antes de registrar su entrada.
///
/// No constituye una autorización cacheada: `registrar_entrada()` vuelve a consultar
/// y validar todas las reglas inmediatamente antes de persistir.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PreparacionIngreso {
    pub contratista_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub resultado_acceso: ResultadoAcceso,
    pub requiere_gafete: bool,
    pub tiene_ingreso_activo: bool,
    /// Números de gafete que este contratista debe actualmente
    /// (`docs/plan-gafetes.md`) — puramente informativo, no bloquea el
    /// ingreso. `Vec` y no `Option<i64>`: nada impide más de una deuda
    /// simultánea.
    pub gafetes_deuda: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IngresoActivoResumen {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    /// Decisión persistida en el momento del ingreso; no se recalcula.
    pub resultado_registrado: ResultadoIngresoRegistrado,
    /// Evaluación con los datos y la fecha actuales.
    pub resultado_acceso: ResultadoAcceso,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ListaIngresosActivosResumen {
    pub items: Vec<IngresoActivoResumen>,
    /// Total real de personas dentro, independiente del filtro aplicado.
    pub total: usize,
}

pub struct RegistroIngresoConsultaService<'a, Q>
where
    Q: IngresosQuery + ?Sized,
{
    consultas: &'a Q,
}

impl<'a, Q> RegistroIngresoConsultaService<'a, Q>
where
    Q: IngresosQuery + ?Sized,
{
    pub fn new(consultas: &'a Q) -> Self {
        Self { consultas }
    }

    pub fn listar_activos(
        &self,
        filtro: &FiltroIngresosActivos,
        hoy: NaiveDate,
    ) -> Result<ListaIngresosActivosResumen, RegistroIngresoServiceError> {
        let resultado = self.consultas.listar_activos(filtro)?;
        Ok(ListaIngresosActivosResumen {
            items: resultado
                .items
                .into_iter()
                .map(|lectura| convertir_activo(lectura, hoy))
                .collect(),
            total: resultado.total,
        })
    }

    pub fn buscar_historial(
        &self,
        filtro: &FiltroHistorial,
    ) -> Result<PaginaHistorial, RegistroIngresoServiceError> {
        if filtro.desde >= filtro.hasta {
            return Err(RegistroIngresoServiceError::RangoFechasInvalido);
        }
        Ok(self.consultas.buscar_historial(filtro)?)
    }
}

fn convertir_activo(lectura: IngresoActivoLectura, hoy: NaiveDate) -> IngresoActivoResumen {
    let contratista = crate::models::contratista::Contratista::reconstruir(
        lectura.contratista_id,
        lectura.cedula.clone(),
        lectura.contratista_nombre.clone(),
        lectura.empresa_id,
        lectura.tipo_ingreso,
        lectura.fecha_vencimiento_praind,
        lectura.es_personal_ruta,
        lectura.tiene_acceso,
        lectura.empresa_activa,
    );
    let resultado_acceso = verificar_acceso(&contratista, hoy);

    IngresoActivoResumen {
        registro_id: lectura.registro_id,
        contratista_id: lectura.contratista_id,
        cedula: lectura.cedula,
        contratista_nombre: lectura.contratista_nombre,
        empresa_nombre: lectura.empresa_nombre,
        tipo_ingreso: lectura.tipo_ingreso,
        medio_ingreso: lectura.medio_ingreso,
        fecha_hora_ingreso: lectura.fecha_hora_ingreso,
        gafete_numero: lectura.gafete_numero,
        usuario_ingreso_nombre: lectura.usuario_ingreso_nombre,
        resultado_registrado: lectura.resultado_registrado,
        resultado_acceso,
    }
}

pub struct RegistroIngresoService<'a, C, R, G>
where
    C: ContratistaRepository + ?Sized,
    R: RegistroIngresoRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    contratistas: &'a C,
    registros: &'a R,
    gafetes: &'a G,
}

impl<'a, C, R, G> RegistroIngresoService<'a, C, R, G>
where
    C: ContratistaRepository + ?Sized,
    R: RegistroIngresoRepository + ?Sized,
    G: GafeteRepository + ?Sized,
{
    pub fn new(contratistas: &'a C, registros: &'a R, gafetes: &'a G) -> Self {
        Self {
            contratistas,
            registros,
            gafetes,
        }
    }

    pub fn preparar_ingreso<E>(
        &self,
        empresas: &E,
        contratista_id: i64,
        hoy: NaiveDate,
    ) -> Result<PreparacionIngreso, RegistroIngresoServiceError>
    where
        E: EmpresaRepository + ?Sized,
    {
        let contratista = self
            .contratistas
            .buscar_por_id(contratista_id)?
            .ok_or(RegistroIngresoServiceError::ContratistaNoEncontrado)?;
        let empresa = empresas
            .buscar_por_id(contratista.empresa_id)?
            .ok_or_else(|| {
                RegistroIngresoServiceError::Database(DatabaseError::Sqlite(
                    rusqlite::Error::InvalidQuery,
                ))
            })?;
        let resultado_acceso = verificar_acceso(&contratista, hoy);
        let requiere_gafete = contratista.requiere_gafete();
        let tiene_ingreso_activo = self
            .registros
            .buscar_ingreso_activo(contratista.id)?
            .is_some();
        let gafetes_deuda = self.gafetes.deuda_de_contratista(contratista.id)?;

        Ok(PreparacionIngreso {
            contratista_id: contratista.id,
            cedula: contratista.cedula,
            nombre: contratista.nombre,
            empresa_nombre: empresa.nombre,
            tipo_ingreso: contratista.tipo_ingreso,
            resultado_acceso,
            requiere_gafete,
            tiene_ingreso_activo,
            gafetes_deuda,
        })
    }

    /// Ejecuta la decisión definitiva usando los repositorios recibidos.
    ///
    /// Este servicio es independiente de SQLite y no abre transacciones. El adaptador
    /// productivo (`AppCore`) debe construir ambos repositorios sobre la misma unidad
    /// de trabajo `IMMEDIATE` para que todas las lecturas y el `INSERT` sean atómicos.
    pub fn registrar_entrada(
        &self,
        contratista_id: i64,
        medio_ingreso: MedioIngreso,
        gafete_numero: Option<i64>,
        usuario_ingreso_id: i64,
        fecha_hora_ingreso: DateTime<Utc>,
    ) -> Result<ResultadoRegistroEntrada, RegistroIngresoServiceError> {
        let contratista = self
            .contratistas
            .buscar_por_id(contratista_id)?
            .ok_or(RegistroIngresoServiceError::ContratistaNoEncontrado)?;

        let resultado_acceso = verificar_acceso(&contratista, fecha_costa_rica(fecha_hora_ingreso));

        if let ResultadoAcceso::Denegado(motivo) = resultado_acceso {
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

            let gafete_encontrado = self.gafetes.buscar_por_numero(numero)?;
            match crate::domain::gafete::validar_para_asignar(gafete_encontrado.as_ref()) {
                crate::domain::gafete::ValidacionAsignacion::NoRegistrado => {
                    return Err(RegistroIngresoServiceError::GafeteNoRegistrado);
                }
                crate::domain::gafete::ValidacionAsignacion::NoDisponible(estado) => {
                    return Err(RegistroIngresoServiceError::GafeteNoDisponible(estado));
                }
                crate::domain::gafete::ValidacionAsignacion::Asignable => {}
            }

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

        let resultado_registrado = match &resultado_acceso {
            ResultadoAcceso::Permitido => ResultadoIngresoRegistrado::Permitido,
            ResultadoAcceso::PermitidoConAdvertencia => {
                ResultadoIngresoRegistrado::PermitidoConAdvertencia(
                    MotivoResultadoIngreso::PraindProximoVencer,
                )
            }
            ResultadoAcceso::Denegado(_) => unreachable!("el acceso denegado ya fue rechazado"),
        };
        // Se lee antes de mover `cedula`/`nombre` hacia `datos_historicos`
        // — `empresa_activa()` pide `&contratista`, que ya no se puede
        // tomar una vez que el struct queda parcialmente movido.
        let empresa_activa = contratista.empresa_activa();
        let registro = NuevoRegistroIngreso {
            contratista_id: contratista.id,
            empresa_id: contratista.empresa_id,
            fecha_hora_ingreso,
            medio_ingreso,
            tipo_ingreso: contratista.tipo_ingreso,
            gafete_numero,
            usuario_ingreso_id,
            datos_historicos: DatosHistoricosEntrada {
                contratista_cedula: contratista.cedula,
                contratista_nombre: contratista.nombre,
                fecha_vencimiento_praind: contratista.fecha_vencimiento_praind,
                es_personal_ruta: contratista.es_personal_ruta,
                tiene_acceso: contratista.tiene_acceso,
                empresa_activa,
                resultado_acceso: resultado_registrado,
                reglas_version: VERSION_REGLAS_ACCESO,
            },
        };

        let registro_id = self.registros.crear(&registro)?;

        Ok(ResultadoRegistroEntrada {
            registro_id,
            resultado_acceso,
        })
    }

    pub fn registrar_salida(
        &self,
        registro_id: i64,
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        let registro = self
            .registros
            .buscar_por_id(registro_id)?
            .ok_or(RegistroIngresoServiceError::RegistroNoActivo)?;

        if registro.salida.is_some() {
            return Err(RegistroIngresoServiceError::RegistroNoActivo);
        }

        if !salida_es_cronologicamente_valida(registro.fecha_hora_ingreso, fecha_hora_salida) {
            return Err(RegistroIngresoServiceError::SalidaAnteriorAIngreso);
        }

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
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), RegistroIngresoServiceError> {
        let registro = self.buscar_ingreso_activo_por_gafete(gafete_numero)?;
        self.registrar_salida(registro.id, fecha_hora_salida, usuario_salida_id)
    }
}
