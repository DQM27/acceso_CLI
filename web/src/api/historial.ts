import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import {
  inicioDiaCostaRicaUtc,
  inicioDiaSiguienteCostaRicaUtc,
} from "../tiempo";
import { filtrosParaServidor } from "./historialFiltros";
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
  const { data, error } = await supabase
    .from("sitios")
    .select("id, nombre")
    .order("nombre");
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

/** La última fila de un tramo con el orden por hora de entrada: el tramo
 * siguiente se pide "después de esta fila" y el servidor no lee las
 * anteriores (con saltos, la página 1.000 leía y descartaba 99.900 filas). */
interface Cursor {
  hora: string;
  id: string;
}

function ordenaPorHora(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
): boolean {
  return (consulta.orden?.campo ?? "hora_entrada") === "hora_entrada";
}

function cursorDe(filas: MovimientoHistorial[]): Cursor | undefined {
  const ultima = filas.at(-1);
  return ultima ? { hora: ultima.hora_entrada, id: ultima.id } : undefined;
}

/** Un tramo de filas con el filtro y el orden de `consulta`: desde el cursor
 * si lo hay, o saltando `primera` filas. Lo comparten la página en pantalla y
 * la exportación.
 *
 * Todo va a `panel_historial_contratistas` (migración
 * `historial_panel_rendimiento`), que valida que quien consulta sea
 * administrador y arma la consulta completa con los índices: bajo RLS,
 * Postgres no usa índices para los filtros de texto y recorría el período
 * entero. */
async function pedirTramo(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
  primera: number,
  cantidad: number,
  cursor?: Cursor,
): Promise<MovimientoHistorial[]> {
  const { desde, hasta, sitioIds, filtros, orden } = consulta;
  const busqueda = consulta.busqueda?.trim() ?? "";
  const { data, error } = await supabase
    .rpc("panel_historial_contratistas", {
      // Mismo criterio de día calendario de Costa Rica que `listarHistorial`.
      p_desde: desde ? inicioDiaCostaRicaUtc(desde) : null,
      p_hasta: hasta ? inicioDiaSiguienteCostaRicaUtc(hasta) : null,
      p_sitio_ids: sitioIds ?? null,
      p_busqueda: busqueda || null,
      p_filtros: filtrosParaServidor(filtros),
      p_orden: orden?.campo ?? "hora_entrada",
      p_descendente: orden?.descendente ?? true,
      p_limite: cantidad,
      p_desplazamiento: cursor ? 0 : primera,
      p_cursor_hora: cursor?.hora ?? null,
      p_cursor_id: cursor?.id ?? null,
    })
    .select(COLUMNAS_MOVIMIENTO);
  if (error) throw new Error(error.message);
  return z.array(movimientoEsquema).parse(data);
}

/** Cursor de la última fila de cada página ya pedida, por consulta (sin la
 * página). La grilla pide la página siguiente casi siempre, así que esa sale
 * por cursor; saltar a una página nunca vista, o un orden que no sea por
 * hora, sigue saltando filas. Acotado a las últimas consultas. */
const cursoresPorConsulta = new Map<string, Map<number, Cursor>>();
const MAXIMO_CONSULTAS_RECORDADAS = 20;

function cursoresDe(consulta: ConsultaMovimientos): Map<number, Cursor> {
  const clave = JSON.stringify({ ...consulta, pagina: undefined });
  let cursores = cursoresPorConsulta.get(clave);
  if (!cursores) {
    if (cursoresPorConsulta.size >= MAXIMO_CONSULTAS_RECORDADAS) {
      const masVieja = cursoresPorConsulta.keys().next().value;
      if (masVieja !== undefined) cursoresPorConsulta.delete(masVieja);
    }
    cursores = new Map();
    cursoresPorConsulta.set(clave, cursores);
  }
  return cursores;
}

export async function listarMovimientosPagina(
  consulta: ConsultaMovimientos,
): Promise<PaginaMovimientos> {
  const primera = consulta.pagina * consulta.tamano;
  const porHora = ordenaPorHora(consulta);
  const cursores = porHora ? cursoresDe(consulta) : undefined;
  const cursor =
    consulta.pagina > 0 ? cursores?.get(consulta.pagina - 1) : undefined;
  const filas = await pedirTramo(consulta, primera, consulta.tamano, cursor);
  const ultima = cursorDe(filas);
  if (cursores && ultima) cursores.set(consulta.pagina, ultima);
  return {
    filas,
    total: filas.length < consulta.tamano ? primera + filas.length : undefined,
  };
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
  // Con el orden por hora, cada tramo sigue al anterior por cursor: el costo
  // de un tramo no crece con lo ya exportado.
  const porHora = ordenaPorHora(consulta);
  while (filas.length < maximo) {
    const cantidad = Math.min(TRAMO_EXPORTACION, maximo - filas.length);
    const tramo = await pedirTramo(
      consulta,
      filas.length,
      cantidad,
      porHora ? cursorDe(filas) : undefined,
    );
    filas.push(...tramo);
    if (tramo.length < cantidad) return { filas, truncado: false };
  }
  // Se llegó al máximo: una fila más dice si quedó algo afuera, sin contar todo.
  const siguiente = await pedirTramo(
    consulta,
    maximo,
    1,
    porHora ? cursorDe(filas) : undefined,
  );
  return { filas, truncado: siguiente.length > 0 };
}
