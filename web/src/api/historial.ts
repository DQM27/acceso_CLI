import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";
import { expresionesDeFiltros } from "./historialFiltros";
import type { ModeloFiltros } from "./historialFiltros";

export { plegarTexto } from "./historialFiltros";
export type { ModeloFiltros } from "./historialFiltros";

/**
 * Espejo de `ingresos` en Supabase -- ver migración
 * `agrega_columnas_historial_a_ingresos`. Antes esa tabla sólo cacheaba
 * ingresos ABIERTOS para el cierre cruzado entre dispositivos del mismo
 * sitio; ahora que también manda el resto de columnas al cerrar, es un
 * historial real (multi-sitio, sin techo de tiempo -- decisión explícita,
 * ver conversación). RLS: sólo quien esté en `administradores_panel` puede
 * leer (`es_admin_global()`, migración
 * `administradores_panel_gestion_admin_global`) -- sin distinción de rol,
 * se eliminó `admin_regional` (ver migración `elimina_admin_regional`).
 */
export interface MovimientoHistorial {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  contratista_cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  medio_ingreso: string | null;
  gafete_numero: number | null;
  hora_entrada: string;
  hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
  // "pc"/"mobile"/"visor" (`dispositivos.tipo`) -- null si el dispositivo
  // de entrada fue borrado, o para filas viejas sin dispositivo_entrada_id.
  dispositivo_entrada_tipo: string | null;
  /** Tipo de ingreso como se lee en pantalla ("IN HOUSE"). */
  tipo_texto: string;
  /** "CAMINANDO", "VEHÍCULO" o la placa si el ingreso fue en vehículo. */
  medio_texto: string;
}

export interface UnidadOperativa {
  id: string;
  nombre: string;
}

const filaSitioEsquema = z.object({ id: z.string(), nombre: z.string() });

/** Sitios visibles para `admin_global` (misma política que ya deja leer
 * `ingresos` cross-sitio, ver el doc-comment de arriba) -- alimenta el
 * selector "Unidades operativas" de Historial. Independiente de si un sitio
 * ya tiene movimientos o no (a diferencia de sacar los nombres de
 * `ingresos` mismo), para que uno recién creado aparezca en el filtro desde
 * el día uno. */
export async function listarUnidadesOperativas(): Promise<UnidadOperativa[]> {
  const { data, error } = await supabase.from("sitios").select("id, nombre").order("nombre");
  if (error) throw new Error(error.message);
  return z.array(filaSitioEsquema).parse(data);
}

// --- Paginación en el servidor -------------------------------------------
//
// Lee la vista `panel_movimientos` (migración `vista_panel_movimientos`): la
// fila ya viene plana (unidad y dispositivo incluidos) y con columnas sin
// tildes ni mayúsculas (`*_p`), así el servidor ordena, filtra y pagina. El
// navegador sólo recibe la página que se ve, sin importar cuántas unidades
// ni cuántos años de datos haya.

/** Campos por los que se puede ordenar en el servidor (columnas de la vista).
 * Lista cerrada a propósito: el campo llega de la interfaz. */
export const CAMPOS_ORDENABLES = [
  "sitio_nombre",
  "contratista_cedula",
  "contratista_nombre",
  "empresa_nombre",
  "dispositivo_entrada_tipo",
  "tipo_texto",
  "medio_texto",
  "gafete_numero",
  "hora_entrada",
  "hora_salida",
  "usuario_entrada_nombre",
  "usuario_salida_nombre",
] as const;

export type CampoOrdenable = (typeof CAMPOS_ORDENABLES)[number];

export interface ConsultaMovimientos {
  desde?: string;
  hasta?: string;
  /** `undefined` = todas las unidades; lista vacía = ninguna (cero filas). */
  sitioIds?: string[];
  busqueda?: string;
  /** Filtros por columna de la grilla (`api.getFilterModel()`), ver `historialFiltros.ts`. */
  filtros?: ModeloFiltros;
  orden?: { campo: CampoOrdenable; descendente: boolean };
  /** Base 0. */
  pagina: number;
  tamano: number;
}

export interface PaginaMovimientos {
  filas: MovimientoHistorial[];
  /** Total de filas del filtro, sólo cuando esta página llegó al final (vino
   * incompleta); `undefined` si puede haber más. Sin conteo exacto a
   * propósito: con el volumen de varias unidades recorría la tabla entera
   * (1,7-5,3 s con 150.000 filas en staging); la grilla sigue pidiendo
   * páginas hasta recibir una incompleta. */
  total?: number;
}

const COLUMNAS_MOVIMIENTO =
  "id, sitio_id, sitio_nombre, contratista_cedula, contratista_nombre, empresa_nombre, " +
  "tipo_ingreso, medio_ingreso, gafete_numero, hora_entrada, hora_salida, " +
  "usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_tipo, tipo_texto, medio_texto";

const movimientoEsquema = z.object({
  id: z.string(),
  sitio_id: z.string(),
  sitio_nombre: z.string().nullable(),
  contratista_cedula: z.string().nullable(),
  contratista_nombre: z.string(),
  empresa_nombre: z.string().nullable(),
  tipo_ingreso: z.string().nullable(),
  medio_ingreso: z.string().nullable(),
  gafete_numero: z.number().nullable(),
  hora_entrada: z.string(),
  hora_salida: z.string().nullable(),
  usuario_entrada_nombre: z.string().nullable(),
  usuario_salida_nombre: z.string().nullable(),
  dispositivo_entrada_tipo: z.string().nullable(),
  tipo_texto: z.string(),
  medio_texto: z.string(),
});

/** Un tramo de filas (`primera`..`primera + cantidad - 1`) con el filtro y el
 * orden de `consulta`. Lo comparten la página en pantalla y la exportación. */
async function pedirTramo(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
  primera: number,
  cantidad: number,
): Promise<MovimientoHistorial[]> {
  const { desde, hasta, sitioIds, filtros, orden } = consulta;
  const busqueda = consulta.busqueda?.trim() ?? "";
  const desdeUtc = desde ? inicioDiaCostaRicaUtc(desde) : undefined;
  const hastaUtc = hasta ? inicioDiaSiguienteCostaRicaUtc(hasta) : undefined;

  // Con búsqueda (cédula o nombre) se usa `panel_buscar_movimientos`: bajo
  // RLS, Postgres no puede usar índices para `like` sobre la vista y
  // recorrería todo el período; la función sí los usa y valida ella misma
  // que quien consulta sea administrador. Devuelve filas con la forma de la
  // vista, así que el orden, los filtros de columna y el tramo se aplican
  // igual en los dos casos. Fechas y unidades van adentro de la función.
  const base = busqueda
    ? supabase.rpc("panel_buscar_movimientos", {
        p_busqueda: busqueda,
        p_desde: desdeUtc ?? null,
        p_hasta: hastaUtc ?? null,
        p_sitio_ids: sitioIds ?? null,
      })
    : supabase.from("panel_movimientos");

  let peticion = base
    .select(COLUMNAS_MOVIMIENTO)
    .order(orden?.campo ?? "hora_entrada", {
      ascending: !(orden?.descendente ?? true),
      nullsFirst: false,
    })
    // Desempate estable: sin él, dos filas con el mismo valor pueden repetirse
    // o saltarse entre una página y la siguiente.
    .order("id")
    .range(primera, primera + cantidad - 1);

  // Mismo criterio de día calendario de Costa Rica que `listarHistorial`.
  if (!busqueda) {
    if (desdeUtc) peticion = peticion.gte("hora_entrada", desdeUtc);
    if (hastaUtc) peticion = peticion.lt("hora_entrada", hastaUtc);
    if (sitioIds) peticion = peticion.in("sitio_id", sitioIds);
  }
  // Un `.or()` por columna filtrada: el AND entre columnas sale de aplicarlos todos.
  for (const expresion of expresionesDeFiltros(filtros)) {
    peticion = peticion.or(expresion);
  }

  const { data, error } = await peticion;
  if (error) throw new Error(error.message);
  return z.array(movimientoEsquema).parse(data);
}

export async function listarMovimientosPagina(consulta: ConsultaMovimientos): Promise<PaginaMovimientos> {
  const primera = consulta.pagina * consulta.tamano;
  const filas = await pedirTramo(consulta, primera, consulta.tamano);
  return { filas, total: filas.length < consulta.tamano ? primera + filas.length : undefined };
}

// Supabase entrega como máximo 1.000 filas por petición.
const TRAMO_EXPORTACION = 1_000;
/** Tope de filas de una exportación -- válvula de seguridad para que un
 * rango enorme no deje la pestaña sin memoria. */
export const MAXIMO_EXPORTACION = 50_000;

export interface ResultadoExportacion {
  filas: MovimientoHistorial[];
  /** `true` si el filtro tiene más filas que `MAXIMO_EXPORTACION`. */
  truncado: boolean;
}

/** Todas las filas del filtro y orden actuales (no sólo la página en
 * pantalla), pedidas en tramos de 1.000, hasta `MAXIMO_EXPORTACION`. */
export async function listarMovimientosParaExportar(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
  maximo: number = MAXIMO_EXPORTACION,
): Promise<ResultadoExportacion> {
  const filas: MovimientoHistorial[] = [];
  while (filas.length < maximo) {
    const cantidad = Math.min(TRAMO_EXPORTACION, maximo - filas.length);
    const tramo = await pedirTramo(consulta, filas.length, cantidad);
    filas.push(...tramo);
    if (tramo.length < cantidad) return { filas, truncado: false };
  }
  // Se llegó al máximo: una fila más dice si quedó algo afuera, sin contar todo.
  const siguiente = await pedirTramo(consulta, maximo, 1);
  return { filas, truncado: siguiente.length > 0 };
}
