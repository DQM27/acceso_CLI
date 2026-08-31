use chrono::NaiveDate;

use super::empresa::Empresa;
use super::tipo_ingreso::TipoIngreso;

#[derive(Debug, Clone)]
pub struct Contratista {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    /// Estado de `Empresa::activo` de `empresa_id` — no es un campo propio
    /// de `contratistas`, viaja aquí para que `domain::acceso::verificar_acceso`
    /// no necesite consultar otra tabla. Privado a propósito: antes era
    /// `pub`, así que cualquier código podía construir o mutar un
    /// `Contratista` con este campo mintiendo sobre el estado real de la
    /// empresa (`domain::acceso::verificar_acceso` confía en él sin
    /// volver a verificar). Sólo se llega a un valor a través de
    /// [`Contratista::nuevo`] (deriva de una `Empresa` real) o
    /// [`Contratista::reconstruir`] (reconstrucción desde una fila de base
    /// de datos ya unida con `empresas`) — ambos nombrados y documentados
    /// en vez de estructuras literales dispersas por el código.
    empresa_activa: bool,
}

impl Contratista {
    /// Construye un `Contratista` nuevo a partir de una `Empresa` real —
    /// `empresa_activa` se deriva de `empresa.activo`, nunca se acepta como
    /// parámetro suelto. Usado por `ContratistaService` al crear/actualizar
    /// (ya tiene la `Empresa` resuelta de una consulta previa).
    #[allow(clippy::too_many_arguments)]
    pub fn nuevo(
        id: i64,
        cedula: String,
        nombre: String,
        empresa: &Empresa,
        tipo_ingreso: TipoIngreso,
        fecha_vencimiento_praind: Option<NaiveDate>,
        es_personal_ruta: bool,
        tiene_acceso: bool,
    ) -> Self {
        Self {
            id,
            cedula,
            nombre,
            empresa_id: empresa.id,
            tipo_ingreso,
            fecha_vencimiento_praind,
            es_personal_ruta,
            tiene_acceso,
            empresa_activa: empresa.activo,
        }
    }

    /// Reconstruye un `Contratista` desde datos ya persistidos (una fila de
    /// `contratistas` unida con `empresas`, o un resumen de consulta con el
    /// mismo origen) — a diferencia de [`Contratista::nuevo`], acá no hay
    /// una `Empresa` viva a mano, sólo el booleano que la consulta SQL ya
    /// resolvió. **Quien llama es responsable de que `empresa_activa` venga
    /// de una unión real con `empresas.activo`, nunca de un valor
    /// inventado** — a diferencia de antes (campo `pub`, mutable desde
    /// cualquier lado sin este nombre ni este comentario), ahora es un solo
    /// punto nombrado y documentado para revisar en vez de estructuras
    /// literales dispersas por el código.
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruir(
        id: i64,
        cedula: String,
        nombre: String,
        empresa_id: i64,
        tipo_ingreso: TipoIngreso,
        fecha_vencimiento_praind: Option<NaiveDate>,
        es_personal_ruta: bool,
        tiene_acceso: bool,
        empresa_activa: bool,
    ) -> Self {
        Self {
            id,
            cedula,
            nombre,
            empresa_id,
            tipo_ingreso,
            fecha_vencimiento_praind,
            es_personal_ruta,
            tiene_acceso,
            empresa_activa,
        }
    }

    pub fn empresa_activa(&self) -> bool {
        self.empresa_activa
    }

    /// Regla de negocio: ver `domain::contratista::requiere_praind`, donde
    /// vive la definición real. Delegada acá para no tocar los ~20 call
    /// sites que ya usan `contratista.requiere_praind()` como método.
    pub fn requiere_praind(&self) -> bool {
        crate::domain::contratista::requiere_praind(self)
    }

    /// Regla de negocio: ver `domain::contratista::requiere_gafete`, donde
    /// vive la definición real. Delegada acá por el mismo motivo.
    pub fn requiere_gafete(&self) -> bool {
        crate::domain::contratista::requiere_gafete(self)
    }
}
