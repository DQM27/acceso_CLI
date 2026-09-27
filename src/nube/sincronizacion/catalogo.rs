use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::{ContextoSincronizacion, SincronizacionError, obtener_json_paginado};
use crate::nube::cliente::cliente_http;

/// Cuántas filas se aplicaron localmente al traer el catálogo del sitio --
/// para que la pantalla pueda avisar "3 contratistas nuevos" sin devolver
/// las filas enteras.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResumenCatalogo {
    pub empresas_recibidas: u32,
    pub contratistas_recibidos: u32,
    pub usuarios_recibidos: u32,
    pub gafetes_recibidos: u32,
    pub rutas_recibidas: u32,
    pub empresas_proveedor_recibidas: u32,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaEmpresaRemota {
    pub(super) id: String,
    pub(super) nombre: String,
    pub(super) activa: bool,
    pub(super) updated_at: String,
}

/// Sin `password_hash` -- nunca viaja, ver el doc-comment de
/// `enviar_usuario`. `rol` puede ser 'ROOT'/'ADMINISTRADOR'/'OPERADOR' (ver
/// migración `permite_root_en_usuarios_globales`).
#[derive(serde::Deserialize)]
pub(super) struct FilaUsuarioRemota {
    pub(super) id: String,
    pub(super) cedula: String,
    pub(super) nombre: String,
    pub(super) rol: String,
    pub(super) activo: bool,
    pub(super) updated_at: String,
}

#[derive(serde::Deserialize)]
pub(super) struct FilaContratistaRemota {
    pub(super) id: String,
    pub(super) nombre: String,
    pub(super) identificacion: Option<String>,
    pub(super) empresa_id: Option<String>,
    pub(super) empresa_nombre: Option<String>,
    pub(super) activo: bool,
    pub(super) tipo_ingreso: Option<String>,
    pub(super) fecha_vencimiento_praind: Option<String>,
    pub(super) es_personal_ruta: Option<bool>,
    pub(super) updated_at: String,
}

/// A diferencia de empresas/contratistas/usuarios (globales), gafetes SÍ
/// se filtra por `sitio_id` -- cada sitio tiene su propio catálogo físico
/// de gafetes, no tiene sentido que uno global exista en todos.
#[derive(serde::Deserialize)]
pub(super) struct FilaGafeteRemota {
    pub(super) id: String,
    pub(super) numero: i64,
    pub(super) tipo: String,
    pub(super) estado: String,
    pub(super) contratista_portador_id: Option<String>,
    pub(super) contratista_portador_nombre: Option<String>,
    // Sin `visita_portador_nombre` a propósito -- a diferencia de
    // contratistas (cuya resolución local puede caer a buscar por nombre,
    // `IndiceLocal::resolver`), `cita_visitantes` sólo se resuelve por
    // `uuid` (ver `indexar_cita_visitantes`), así que el nombre nunca se
    // usa acá -- sí viaja en el `push` (`construir_cuerpo_gafete`) para que
    // el panel/otro consumidor remoto lo pueda mostrar sin un join.
    pub(super) visita_portador_id: Option<String>,
    pub(super) updated_at: String,
}

/// Catálogo de rutas -- por sitio, como gafetes (ver
/// `construir_cuerpo_ruta`), pero sin la complejidad de "portador
/// pendiente" -- comparte la marca general (`filtro_incremental`), no una
/// propia.
#[derive(serde::Deserialize)]
pub(super) struct FilaRutaRemota {
    pub(super) id: String,
    pub(super) numero: i64,
    pub(super) activo: bool,
    pub(super) updated_at: String,
}

/// Trae de la nube las empresas y contratistas de *este mismo sitio* que
/// este dispositivo todavía no tiene localmente -- el "pull" que le
/// faltaba al espejo (hasta ahora sólo empujaba: local → nube, nunca al
/// revés). Usa la misma política RLS que ya existe, sin tocarla, así que
/// sólo trae lo del propio sitio -- esto no es el seed global entre
/// sitios (`docs/planes-implementados/plan-persistencia-nube.md`, diferido), sólo lo que el
/// otro dispositivo de este sitio ya empujó.
///
/// Mismo patrón `ON CONFLICT` que usa el archivo de seed
/// (`contratistas_base_final_limpia_v15.sql`): si ya existe localmente una
/// fila con el mismo nombre/cédula (la creó este dispositivo, o un import
/// anterior), la actualiza y le completa el `uuid` en vez de duplicarla --
/// `COALESCE(tabla.uuid, excluded.uuid)` nunca pisa un `uuid` que ya tenía.
///
/// Una fila remota sin `identificacion`/`tipo_ingreso` (contratista creado
/// antes de que el espejo mandara estos campos) se salta -- ambos son
/// `NOT NULL` en la tabla local, y en una app de control de acceso no se
/// inventan datos de clasificación para completar el hueco.
/// Resultado de la descarga (sin tocar la base todavía) -- separado de
/// `recibir_catalogo_del_sitio` sólo para que esa función no crezca más allá
/// del límite de líneas del lint `too_many_lines`; el corte natural ya
/// existía en el código (todo el HTTP primero, todo el `INSERT` después).
pub(super) struct CatalogoRemotoDescargado {
    pub(super) empresas: Vec<FilaEmpresaRemota>,
    pub(super) contratistas: Vec<FilaContratistaRemota>,
    pub(super) usuarios: Vec<FilaUsuarioRemota>,
    pub(super) gafetes: Vec<FilaGafeteRemota>,
    pub(super) rutas: Vec<FilaRutaRemota>,
    pub(super) empresas_proveedor: Vec<FilaEmpresaProveedorRemota>,
    pub(super) marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>>,
}

/// Catálogo por sitio, como rutas (ver `FilaRutaRemota`) -- a diferencia de
/// `empresas` (contratistas), que es global entre sitios, cada sitio
/// maneja su propio directorio de empresas proveedoras.
#[derive(serde::Deserialize)]
pub(super) struct FilaEmpresaProveedorRemota {
    pub(super) id: String,
    pub(super) nombre: String,
    pub(super) activa: bool,
    pub(super) updated_at: String,
}

/// Trae empresas/contratistas/usuarios/gafetes, cada uno incremental según
/// su propia marca -- ver los comentarios que tenían estas mismas consultas
/// en `recibir_catalogo_del_sitio` antes del corte.
pub(super) fn descargar_catalogo_remoto(
    contexto: &ContextoSincronizacion<'_>,
    marca_anterior: Option<&str>,
    marca_gafetes_anterior: Option<&str>,
) -> Result<CatalogoRemotoDescargado, SincronizacionError> {
    let cliente = cliente_http();
    let filtro_incremental = marca_anterior
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();

    // Sin `sitio_id=eq...` a propósito -- contratistas y empresas son
    // globales (ver docs/planes-implementados/plan-panel-administrativo-web.md, "Modelo de
    // datos"): si a un contratista se le niega el acceso en un sitio, tiene
    // que quedar negado en TODOS. El nombre de la función quedó del modelo
    // viejo (un solo sitio por dispositivo); lo que trae ahora es el
    // catálogo global completo, no "del sitio" de `contexto`.
    let empresas: Vec<FilaEmpresaRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/empresas?select=id,nombre,activa,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    let contratistas: Vec<FilaContratistaRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/contratistas?select=id,nombre,identificacion,empresa_id,empresa_nombre,\
             activo,tipo_ingreso,fecha_vencimiento_praind,es_personal_ruta,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    let usuarios: Vec<FilaUsuarioRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/usuarios?select=id,cedula,nombre,rol,activo,updated_at{filtro_incremental}",
            contexto.base_url
        ),
    )?;
    // Incremental con marca PROPIA (`gafetes_actualizado_hasta`, no la misma
    // `marca_anterior` de arriba) -- antes se descargaba completo cada vez,
    // justamente para no depender de un cursor compartido que podía ser
    // anterior a que gafetes se sumara al pull. Una columna propia (nace en
    // `NULL`) resuelve eso sin ayuda: el primer sync de cualquier
    // dispositivo siempre baja todo. La otra razón de bajar todo siempre
    // -- reintentar gafetes PERDIDOS cuyo deudor todavía no resolvía
    // localmente -- la resuelve `guardar_gafetes`, capando cuánto puede
    // avanzar esta marca. Con `sitio_id=eq...` a diferencia de las tres de
    // arriba -- ver comentario de `FilaGafeteRemota`.
    let filtro_incremental_gafetes = marca_gafetes_anterior
        .map(|marca| format!("&updated_at=gt.{marca}"))
        .unwrap_or_default();
    let gafetes: Vec<FilaGafeteRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/gafetes?sitio_id=eq.{}&select=id,numero,tipo,estado,\
             contratista_portador_id,contratista_portador_nombre,\
             visita_portador_id,updated_at{filtro_incremental_gafetes}",
            contexto.base_url, contexto.sitio_id
        ),
    )?;
    // Comparte `filtro_incremental` (marca general), no una propia -- ver
    // el doc-comment de `FilaRutaRemota`. Con `sitio_id=eq...`, mismo
    // motivo que gafetes: catálogo por sitio.
    let rutas: Vec<FilaRutaRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/rutas?sitio_id=eq.{}&select=id,numero,activo,updated_at{filtro_incremental}",
            contexto.base_url, contexto.sitio_id
        ),
    )?;
    // Comparte `filtro_incremental` (marca general), no una propia -- mismo
    // criterio que rutas: catálogo por sitio, sin la complejidad de
    // "portador pendiente" que sí justifica la marca separada de gafetes.
    let empresas_proveedor: Vec<FilaEmpresaProveedorRemota> = obtener_json_paginado(
        &cliente,
        contexto,
        &format!(
            "{}/rest/v1/empresas_proveedor?sitio_id=eq.{}&select=id,nombre,activa,updated_at{filtro_incremental}",
            contexto.base_url, contexto.sitio_id
        ),
    )?;

    // Máximo `updated_at` real entre empresas/contratistas/usuarios/rutas/
    // empresas_proveedor -- gafetes lleva su propia marca por separado
    // (`guardar_gafetes` la calcula después, capada por lo que haya
    // quedado pendiente de resolver, ver su doc-comment). Sin filas
    // nuevas, la marca no avanza -- preferible repetir la misma consulta
    // (ya sabemos que no trae nada) a arriesgar perder una fila por un
    // reloj local desviado.
    let mut marca_mas_nueva: Option<chrono::DateTime<chrono::Utc>> =
        marca_anterior.and_then(|marca| crate::tiempo::parsear_utc(marca).ok());
    for actualizado_en in empresas
        .iter()
        .map(|f| &f.updated_at)
        .chain(contratistas.iter().map(|f| &f.updated_at))
        .chain(usuarios.iter().map(|f| &f.updated_at))
        .chain(rutas.iter().map(|f| &f.updated_at))
        .chain(empresas_proveedor.iter().map(|f| &f.updated_at))
    {
        let actualizado_en = crate::tiempo::parsear_utc(actualizado_en)
            .map_err(|_| SincronizacionError::FechaInvalida(actualizado_en.clone()))?;
        if marca_mas_nueva.is_none_or(|marca| actualizado_en > marca) {
            marca_mas_nueva = Some(actualizado_en);
        }
    }

    Ok(CatalogoRemotoDescargado {
        empresas,
        contratistas,
        usuarios,
        gafetes,
        rutas,
        empresas_proveedor,
        marca_mas_nueva,
    })
}

pub(super) fn guardar_rutas(
    transaction: &rusqlite::Transaction<'_>,
    rutas: &[FilaRutaRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidas = 0;
    for ruta in rutas {
        transaction.execute(
            "
            INSERT INTO rutas (numero, activo, uuid) VALUES (?1, ?2, ?3)
            ON CONFLICT(numero) DO UPDATE SET
                activo = excluded.activo,
                uuid = COALESCE(rutas.uuid, excluded.uuid)
            ",
            params![ruta.numero, ruta.activo, ruta.id],
        )?;
        recibidas += 1;
    }
    Ok(recibidas)
}

/// Tres `ON CONFLICT` encadenados (soportado desde `SQLite` 3.35, ver
/// `guardar_empresas` con el mismo patrón), probados en este orden hasta que
/// uno matchea:
/// 1. `uuid` -- la fila YA estaba sincronizada y sólo le cambió el nombre en
///    la nube (ej. un `UPDATE` a mayúsculas hecho directo en Supabase):
///    actualiza el nombre de esa misma fila.
/// 2. `nombre` (exacto) -- fila nueva sin conflicto de uuid, pero coincide
///    letra por letra con una empresa creada en ESTE dispositivo: la fusiona
///    sin duplicar, completándole el `uuid`.
/// 3. `PLEGAR(nombre)` (sin mayúsculas/acentos) -- ni uuid ni el nombre
///    exacto coinciden, pero SÍ el nombre plegado: es la MISMA empresa con
///    otra grafía (ej. local "Mayca" vs. remoto ya normalizado "MAYCA", cada
///    una con su propio `uuid` por haberse creado por separado). Sin esta
///    tercera rama, ese INSERT no encontraba conflicto en `uuid` ni en
///    `nombre` y terminaba chocando en cambio con
///    `idx_empresas_proveedor_nombre_plegado`, un error sin capturar que
///    abortaba el pull ENTERO (bug real en producción, 2026-09-21: tras
///    normalizar nombres de empresas a mayúsculas en Supabase, ni desktop ni
///    mobile volvían a sincronizar nada, ni siquiera ingresos, porque toda
///    la transacción del pull fallaba en este paso).
pub(super) fn guardar_empresas_proveedor(
    transaction: &rusqlite::Transaction<'_>,
    empresas: &[FilaEmpresaProveedorRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidas = 0;
    for empresa in empresas {
        transaction.execute(
            "
            INSERT INTO empresas_proveedor (nombre, activo, uuid) VALUES (?1, ?2, ?3)
            ON CONFLICT(uuid) DO UPDATE SET
                nombre = excluded.nombre,
                activo = excluded.activo
            ON CONFLICT(nombre) DO UPDATE SET
                activo = excluded.activo,
                uuid = COALESCE(empresas_proveedor.uuid, excluded.uuid)
            ON CONFLICT(PLEGAR(nombre)) DO UPDATE SET
                nombre = excluded.nombre,
                activo = excluded.activo,
                uuid = COALESCE(empresas_proveedor.uuid, excluded.uuid)
            ",
            params![empresa.nombre, empresa.activa, empresa.id],
        )?;
        recibidas += 1;
    }
    Ok(recibidas)
}

/// Mismo motivo que `guardar_empresas_proveedor`: `ON CONFLICT(uuid)`
/// primero para que un nombre cambiado en la nube (ej. mayúsculas) sí baje a
/// una fila ya sincronizada, con `ON CONFLICT(nombre)` como respaldo para
/// fusionar una empresa creada en este dispositivo sin duplicarla.
pub(super) fn guardar_empresas(
    transaction: &rusqlite::Transaction<'_>,
    empresas: &[FilaEmpresaRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidas = 0;
    for empresa in empresas {
        transaction.execute(
            "
            INSERT INTO empresas (nombre, activo, uuid) VALUES (?1, ?2, ?3)
            ON CONFLICT(uuid) DO UPDATE SET
                nombre = excluded.nombre,
                activo = excluded.activo
            ON CONFLICT(nombre) DO UPDATE SET
                activo = excluded.activo,
                uuid = COALESCE(empresas.uuid, excluded.uuid)
            ON CONFLICT(PLEGAR(nombre)) DO UPDATE SET
                nombre = excluded.nombre,
                activo = excluded.activo,
                uuid = COALESCE(empresas.uuid, excluded.uuid)
            ",
            params![empresa.nombre, empresa.activa, empresa.id],
        )?;
        recibidas += 1;
    }
    Ok(recibidas)
}

pub(super) fn guardar_contratistas(
    transaction: &rusqlite::Transaction<'_>,
    contratistas: &[FilaContratistaRemota],
) -> Result<u32, SincronizacionError> {
    // Un solo `SELECT` de todas las empresas locales antes del lote, en vez
    // de hasta dos por cada contratista remoto (`resolver_empresa_local`
    // anterior) -- con un catálogo grande eran miles de consultas
    // individuales secuenciales sólo para resolver el vínculo. `empresas`
    // ya está completa en este punto (`guardar_empresas` corrió antes, ver
    // `recibir_catalogo_del_sitio`) y esta función nunca la modifica, así
    // que el índice no se desactualiza durante el resto del lote.
    let indice_empresas = indexar_empresas(transaction)?;

    let mut recibidos = 0;
    for contratista in contratistas {
        let (Some(cedula), Some(tipo_ingreso), Some(es_personal_ruta)) = (
            contratista.identificacion.as_deref(),
            contratista.tipo_ingreso.as_deref(),
            contratista.es_personal_ruta,
        ) else {
            continue;
        };
        let empresa_id_local = indice_empresas.resolver(
            contratista.empresa_id.as_deref(),
            contratista.empresa_nombre.as_deref(),
        );
        let Some(empresa_id_local) = empresa_id_local else {
            continue;
        };

        transaction.execute(
            "
            INSERT INTO contratistas (
                cedula, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind,
                es_personal_ruta, tiene_acceso, uuid
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(cedula) DO UPDATE SET
                nombre = excluded.nombre,
                empresa_id = excluded.empresa_id,
                tipo_ingreso = excluded.tipo_ingreso,
                fecha_vencimiento_praind = excluded.fecha_vencimiento_praind,
                es_personal_ruta = excluded.es_personal_ruta,
                tiene_acceso = excluded.tiene_acceso,
                uuid = COALESCE(contratistas.uuid, excluded.uuid)
            ",
            params![
                cedula,
                contratista.nombre,
                empresa_id_local,
                tipo_ingreso,
                contratista.fecha_vencimiento_praind,
                es_personal_ruta,
                contratista.activo,
                contratista.id,
            ],
        )?;
        recibidos += 1;
    }
    Ok(recibidos)
}

pub(super) fn guardar_usuarios(
    transaction: &rusqlite::Transaction<'_>,
    usuarios: &[FilaUsuarioRemota],
) -> Result<u32, SincronizacionError> {
    let mut recibidos = 0;
    for usuario in usuarios {
        // `password_hash` queda AFUERA del `DO UPDATE SET` a propósito --
        // si esta cédula ya existe local (con una contraseña real, fijada
        // en este mismo dispositivo alguna vez), el `ON CONFLICT` no debe
        // tocarla nunca. Sólo un usuario recién llegado (el `INSERT` de la
        // primera vez) arranca con el centinela `SIN_PASSWORD_LOCAL`, que
        // `AutenticacionService::buscar_candidato` reconoce para mandar al
        // alta de contraseña en vez de "credenciales inválidas".
        transaction.execute(
            "
            INSERT INTO usuarios (cedula, nombre, rol, activo, password_hash, uuid)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(cedula) DO UPDATE SET
                nombre = excluded.nombre,
                rol = excluded.rol,
                activo = excluded.activo,
                uuid = COALESCE(usuarios.uuid, excluded.uuid)
            ",
            params![
                usuario.cedula,
                usuario.nombre,
                usuario.rol,
                usuario.activo,
                crate::services::password::SIN_PASSWORD_LOCAL,
                usuario.id,
            ],
        )?;
        recibidos += 1;
    }
    Ok(recibidos)
}

/// Devuelve cuántos gafetes se guardaron y hasta qué `updated_at` es seguro
/// avanzar `gafetes_actualizado_hasta` -- las dos cosas separadas porque no
/// necesariamente coinciden: un gafete PERDIDO cuyo deudor todavía no
/// resuelve localmente NO se guarda (violaría el `CHECK` de la tabla,
/// `estado = 'PERDIDO' AND contratista_deudor_id IS NOT NULL`), pero tiene
/// que seguir pidiéndose en el próximo sync hasta que resuelva -- si la
/// marca avanzara igual hasta su `updated_at`, ese gafete quedaría afuera
/// del filtro incremental (`updated_at=gt.marca`) para siempre y nunca más
/// se sabría de su deuda. Mientras quede alguno pendiente, la marca
/// simplemente no avanza nada este ciclo (`None`, el llamador deja
/// `gafetes_actualizado_hasta` como estaba) -- todo lo demás sí se guarda
/// igual (no tiene sentido demorar gafetes que sí resuelven sólo porque
/// otro, sin relación, sigue pendiente), sólo la marca de agua espera. Es
/// una regla deliberadamente conservadora (podría, en teoría, avanzar
/// parcialmente hasta el más viejo de los pendientes) a cambio de quedar
/// simple y obviamente correcta -- con un catálogo de gafetes chico
/// (acotado al físico de un sitio, no crece con la cantidad de
/// contratistas) el costo de repetir la consulta un ciclo más mientras algo
/// sigue pendiente es insignificante.
/// Índice de `cita_visitantes` sólo por `uuid` -- a diferencia de
/// `IndiceLocal` (empresas/contratistas), esta tabla es puramente de
/// sincronización (`CitaRepository`, doc-comment: "la llena la
/// sincronización"), sin creación local previa que compita, así que no
/// hace falta el respaldo por nombre.
pub(super) fn indexar_cita_visitantes(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, i64>, SincronizacionError> {
    let mut statement = transaction.prepare("SELECT id, uuid FROM cita_visitantes")?;
    let filas = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut indice = HashMap::new();
    for fila in filas {
        let (id, uuid) = fila?;
        indice.insert(uuid, id);
    }
    Ok(indice)
}

pub(super) fn guardar_gafetes(
    transaction: &rusqlite::Transaction<'_>,
    gafetes: &[FilaGafeteRemota],
) -> Result<(u32, Option<chrono::DateTime<chrono::Utc>>), SincronizacionError> {
    // Mismo motivo que `indice_empresas` en `guardar_contratistas`: un solo
    // `SELECT` de todos los contratistas/`cita_visitantes` locales antes
    // del lote, en vez de hasta dos por cada gafete PERDIDO
    // (`resolver_contratista_local` anterior). `guardar_contratistas` ya
    // corrió antes en el mismo `recibir_catalogo_del_sitio` y esta función
    // no modifica ninguna de las dos tablas, así que ambos índices se
    // mantienen válidos todo el lote.
    let indice_contratistas = indexar_contratistas(transaction)?;
    let indice_cita_visitantes = indexar_cita_visitantes(transaction)?;

    let mut recibidos = 0_u32;
    let mut marca_maxima_aplicada: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut quedo_pendiente = false;
    for gafete in gafetes {
        let actualizado_en = crate::tiempo::parsear_utc(&gafete.updated_at)
            .map_err(|_| SincronizacionError::FechaInvalida(gafete.updated_at.clone()))?;

        // Un gafete PERDIDO sin portador resoluble localmente violaría el
        // `CHECK` de la tabla -- se salta por ahora, mismo criterio que un
        // contratista remoto incompleto: se autorresuelve solo en un sync
        // posterior, en cuanto la fila que le falta también llegue acá (ver
        // el doc-comment de la función sobre `quedo_pendiente`). Un gafete
        // `PROVEEDOR` todavía no tiene columna de portador local -- no se
        // guarda hasta que exista esa categoría de verdad.
        let (contratista_portador_id_local, visita_portador_id_local) =
            if gafete.estado == "PERDIDO" {
                match gafete.tipo.as_str() {
                    "CONTRATISTA" => {
                        let Some(id) = indice_contratistas.resolver(
                            gafete.contratista_portador_id.as_deref(),
                            gafete.contratista_portador_nombre.as_deref(),
                        ) else {
                            quedo_pendiente = true;
                            continue;
                        };
                        (Some(id), None)
                    }
                    "VISITA" => {
                        let Some(uuid) = gafete.visita_portador_id.as_deref() else {
                            quedo_pendiente = true;
                            continue;
                        };
                        let Some(id) = indice_cita_visitantes.get(uuid).copied() else {
                            quedo_pendiente = true;
                            continue;
                        };
                        (None, Some(id))
                    }
                    _ => {
                        quedo_pendiente = true;
                        continue;
                    }
                }
            } else {
                (None, None)
            };

        transaction.execute(
            "
            INSERT INTO gafetes (numero, tipo, estado, contratista_portador_id, visita_portador_id, uuid)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(numero, tipo) DO UPDATE SET
                estado = excluded.estado,
                contratista_portador_id = excluded.contratista_portador_id,
                visita_portador_id = excluded.visita_portador_id,
                uuid = COALESCE(gafetes.uuid, excluded.uuid)
            ",
            params![
                gafete.numero,
                gafete.tipo,
                gafete.estado,
                contratista_portador_id_local,
                visita_portador_id_local,
                gafete.id
            ],
        )?;
        recibidos += 1;
        if marca_maxima_aplicada.is_none_or(|marca| actualizado_en > marca) {
            marca_maxima_aplicada = Some(actualizado_en);
        }
    }

    let marca_segura = if quedo_pendiente {
        None
    } else {
        marca_maxima_aplicada
    };
    Ok((recibidos, marca_segura))
}

pub fn recibir_catalogo_del_sitio(
    connection: &Connection,
    contexto: &ContextoSincronizacion<'_>,
) -> Result<ResumenCatalogo, SincronizacionError> {
    // Sync incremental (MIGRACION_23): sin esto, cada ciclo (cada 2 minutos,
    // para siempre) traía las tres tablas COMPLETAS aunque nada hubiera
    // cambiado. `marca_anterior` es NULL la primera vez (sembrado inicial,
    // sin filtro -- hay que traer todo lo que ya existe). La marca nueva se
    // calcula a partir del `updated_at` real que devolvió el servidor -- no
    // del reloj de este dispositivo: un reloj local apenas adelantado
    // respecto al del servidor dejaba la marca "en el futuro", y cualquier
    // fila con `updated_at` real por debajo quedaba fuera de
    // `WHERE updated_at > marca` para siempre, sin ningún error visible
    // (bug real, reproducido en producción).
    let marca_anterior: Option<String> = connection.query_row(
        "SELECT catalogo_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    // Marca propia de gafetes (MIGRACION_27) -- ver el doc-comment de
    // `guardar_gafetes` sobre por qué no comparte `marca_anterior`.
    let marca_gafetes_anterior: Option<String> = connection.query_row(
        "SELECT gafetes_actualizado_hasta FROM sincronizacion_estado WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    let descarga = descargar_catalogo_remoto(
        contexto,
        marca_anterior.as_deref(),
        marca_gafetes_anterior.as_deref(),
    )?;

    let transaction = connection.unchecked_transaction()?;
    // Orden explícito (no evaluación implícita de un literal de struct):
    // `guardar_contratistas` necesita que `guardar_empresas` ya haya
    // corrido (resuelve `empresa_id` local), y `guardar_gafetes` necesita
    // que `guardar_contratistas` ya haya corrido (resuelve
    // `contratista_deudor_id` local) -- ver sus propios comentarios.
    let empresas_recibidas = guardar_empresas(&transaction, &descarga.empresas)?;
    let contratistas_recibidos = guardar_contratistas(&transaction, &descarga.contratistas)?;
    let usuarios_recibidos = guardar_usuarios(&transaction, &descarga.usuarios)?;
    let (gafetes_recibidos, marca_gafetes_nueva) =
        guardar_gafetes(&transaction, &descarga.gafetes)?;
    let rutas_recibidas = guardar_rutas(&transaction, &descarga.rutas)?;
    let empresas_proveedor_recibidas =
        guardar_empresas_proveedor(&transaction, &descarga.empresas_proveedor)?;
    let resumen = ResumenCatalogo {
        empresas_recibidas,
        contratistas_recibidos,
        usuarios_recibidos,
        gafetes_recibidos,
        rutas_recibidas,
        empresas_proveedor_recibidas,
    };

    if let Some(marca) = descarga.marca_mas_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET catalogo_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }
    if let Some(marca) = marca_gafetes_nueva {
        transaction.execute(
            "UPDATE sincronizacion_estado SET gafetes_actualizado_hasta = ?1 WHERE id = 1",
            params![crate::tiempo::serializar_utc(marca)],
        )?;
    }

    transaction.commit()?;
    Ok(resumen)
}

/// Índice en memoria de una tabla local por `uuid` y por `nombre`,
/// construido con un solo `SELECT` antes de recorrer un lote remoto -- ver
/// `indexar_empresas`/`indexar_contratistas`. Reemplaza a
/// `resolver_empresa_local`/`resolver_contratista_local`, que antes hacían
/// hasta dos `SELECT` individuales POR FILA del lote (miles de round-trips
/// secuenciales para un catálogo grande).
pub(super) struct IndiceLocal {
    pub(super) por_uuid: HashMap<String, i64>,
    pub(super) por_nombre: HashMap<String, i64>,
}

impl IndiceLocal {
    /// Primero por `uuid` (el vínculo real), y si todavía no llegó acá por
    /// ese camino, por nombre (mismo respaldo que ya usaban las funciones
    /// que esto reemplaza). `nombre` no es único en ninguna de las dos
    /// tablas que usan esto -- misma ambigüedad que ya tenía el `SELECT
    /// ... WHERE nombre = ?1` original (sin `ORDER BY`, ya devolvía una
    /// fila arbitraria entre varias); acá el índice simplemente se queda
    /// con la última que recorrió `indexar_empresas`/`indexar_contratistas`.
    fn resolver(&self, uuid: Option<&str>, nombre: Option<&str>) -> Option<i64> {
        uuid.and_then(|uuid| self.por_uuid.get(uuid).copied())
            .or_else(|| nombre.and_then(|nombre| self.por_nombre.get(nombre).copied()))
    }
}

pub(super) fn indexar_empresas(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<IndiceLocal, SincronizacionError> {
    let mut statement = transaction.prepare("SELECT id, uuid, nombre FROM empresas")?;
    let filas = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut por_uuid = HashMap::new();
    let mut por_nombre = HashMap::new();
    for fila in filas {
        let (id, uuid, nombre) = fila?;
        if let Some(uuid) = uuid {
            por_uuid.insert(uuid, id);
        }
        por_nombre.insert(nombre, id);
    }
    Ok(IndiceLocal {
        por_uuid,
        por_nombre,
    })
}

pub(super) fn indexar_contratistas(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<IndiceLocal, SincronizacionError> {
    let mut statement = transaction.prepare("SELECT id, uuid, nombre FROM contratistas")?;
    let filas = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut por_uuid = HashMap::new();
    let mut por_nombre = HashMap::new();
    for fila in filas {
        let (id, uuid, nombre) = fila?;
        if let Some(uuid) = uuid {
            por_uuid.insert(uuid, id);
        }
        por_nombre.insert(nombre, id);
    }
    Ok(IndiceLocal {
        por_uuid,
        por_nombre,
    })
}
