import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";
import { expresionesDeFiltros, plegarTexto } from "./historialFiltros";
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
// fila ya viene plana (unidad y dispositivo incluidos) y con `texto_busqueda`
// sin tildes ni mayúsculas, así el servidor ordena, filtra y pagina. El
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
  /** Total de filas que cumplen el filtro (todas las páginas). */
  total: number;
}

/** Palabras de la búsqueda, ya plegadas y con `\`, `%` y `_` escapados para
 * `like` (una cédula o un nombre nunca se interpretan como comodines). Cada
 * palabra debe aparecer (en cualquier orden) en `texto_busqueda`. */
export function palabrasDeBusqueda(busqueda: string): string[] {
  return plegarTexto(busqueda)
    .split(/\s+/)
    .filter((palabra) => palabra.length > 0)
    .map((palabra) => palabra.replace(/[\\%_]/g, "\\$&"));
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
): Promise<PaginaMovimientos> {
  const { desde, hasta, sitioIds, busqueda, filtros, orden } = consulta;

  let peticion = supabase
    .from("panel_movimientos")
    .select(COLUMNAS_MOVIMIENTO, { count: "exact" })
    .order(orden?.campo ?? "hora_entrada", {
      ascending: !(orden?.descendente ?? true),
      nullsFirst: false,
    })
    // Desempate estable: sin él, dos filas con el mismo valor pueden repetirse
    // o saltarse entre una página y la siguiente.
    .order("id")
    .range(primera, primera + cantidad - 1);

  // Mismo criterio de día calendario de Costa Rica que `listarHistorial`.
  if (desde) peticion = peticion.gte("hora_entrada", inicioDiaCostaRicaUtc(desde));
  if (hasta) peticion = peticion.lt("hora_entrada", inicioDiaSiguienteCostaRicaUtc(hasta));
  if (sitioIds) peticion = peticion.in("sitio_id", sitioIds);
  for (const palabra of palabrasDeBusqueda(busqueda ?? "")) {
    peticion = peticion.like("texto_busqueda", `%${palabra}%`);
  }
  // Un `.or()` por columna filtrada: el AND entre columnas sale de aplicarlos todos.
  for (const expresion of expresionesDeFiltros(filtros)) {
    peticion = peticion.or(expresion);
  }

  const { data, error, count } = await peticion;
  if (error) throw new Error(error.message);
  const filas = z.array(movimientoEsquema).parse(data);
  return { filas, total: count ?? filas.length };
}

export async function listarMovimientosPagina(consulta: ConsultaMovimientos): Promise<PaginaMovimientos> {
  return pedirTramo(consulta, consulta.pagina * consulta.tamano, consulta.tamano);
}

// Supabase entrega como máximo 1.000 filas por petición.
const TRAMO_EXPORTACION = 1_000;
/** Tope de filas de una exportación -- válvula de seguridad para que un
 * rango enorme no deje la pestaña sin memoria. */
export const MAXIMO_EXPORTACION = 50_000;

export interface ResultadoExportacion {
  filas: MovimientoHistorial[];
  /** Cuántas filas cumplen el filtro en total. */
  total: number;
  /** `true` si `total` supera lo exportado (se llegó a `MAXIMO_EXPORTACION`). */
  truncado: boolean;
}

/** Todas las filas del filtro y orden actuales (no sólo la página en
 * pantalla), pedidas en tramos de 1.000, hasta `MAXIMO_EXPORTACION`. */
export async function listarMovimientosParaExportar(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
  maximo: number = MAXIMO_EXPORTACION,
): Promise<ResultadoExportacion> {
  const filas: MovimientoHistorial[] = [];
  let total = 0;
  while (filas.length < maximo) {
    const cantidad = Math.min(TRAMO_EXPORTACION, maximo - filas.length);
    const tramo = await pedirTramo(consulta, filas.length, cantidad);
    total = tramo.total;
    filas.push(...tramo.filas);
    if (tramo.filas.length < cantidad) break;
  }
  return { filas, total, truncado: total > filas.length };
}
