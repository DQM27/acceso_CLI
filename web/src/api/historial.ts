import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";

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
}

// Valida en runtime la forma real de lo que devuelve Supabase -- ver el
// mismo criterio en contratistas.ts/usuarios.ts. `z.infer` reemplaza a la
// interfaz `FilaCruda` que había antes, para no mantener dos fuentes de
// verdad del mismo shape.
const filaCrudaEsquema = z.object({
  id: z.string(),
  sitio_id: z.string(),
  sitios: z.object({ nombre: z.string() }).nullable(),
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
  dispositivo_entrada: z.object({ tipo: z.string() }).nullable(),
});

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
  "tipo_ingreso",
  "medio_ingreso",
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

/** Igual que `public.plegar_texto` en la base: sin tildes y en minúsculas
 * (la ñ queda como n, igual que `unaccent`). */
export function plegarTexto(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
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
  "usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_tipo";

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
});

/** Un tramo de filas (`primera`..`primera + cantidad - 1`) con el filtro y el
 * orden de `consulta`. Lo comparten la página en pantalla y la exportación. */
async function pedirTramo(
  consulta: Omit<ConsultaMovimientos, "pagina" | "tamano">,
  primera: number,
  cantidad: number,
): Promise<PaginaMovimientos> {
  const { desde, hasta, sitioIds, busqueda, orden } = consulta;

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

export interface ResultadoHistorial {
  filas: MovimientoHistorial[];
  /** `true` si el rango pedido tiene más filas que `LIMITE_HISTORIAL` -- ver
   * esa constante. AG Grid corre en modo client-side (trae todo, filtra en
   * el navegador, ver `componentes/Tabla.tsx`); sin este tope, un rango
   * amplio (o el preset "Todo el historial", sin fecha) podía crecer sin
   * cota junto con el uso real del sistema. Mismo criterio que
   * `CargaCompleta.truncado` del núcleo Rust en la versión de escritorio
   * (`desktop/src/pantallas/Historial.tsx`) -- filas visibles acotadas,
   * exportar (Excel/PDF) sigue trayendo el rango completo sin este límite
   * (ver `exportarAExcel`/`exportarAPdf` en `pantallas/Historial.tsx`).
   */
  truncado: boolean;
}

// Bien por encima de cualquier volumen real de un rango de fechas típico
// (6 meses por defecto, ver `Historial.tsx`) -- es una válvula de
// seguridad, no una paginación real: mientras el volumen se mantenga
// razonable, nadie la nota.
const LIMITE_HISTORIAL = 20_000;

export async function listarHistorial(
  desde?: string,
  hasta?: string,
  sitioIds?: string[],
): Promise<ResultadoHistorial> {
  let consulta = supabase
    .from("ingresos")
    .select(
      "id, sitio_id, contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso, " +
        "medio_ingreso, gafete_numero, hora_entrada, hora_salida, usuario_entrada_nombre, " +
        "usuario_salida_nombre, sitios(nombre), " +
        "dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)",
      { count: "exact" },
    )
    .order("hora_entrada", { ascending: false })
    .range(0, LIMITE_HISTORIAL - 1);

  // `desde`/`hasta` llegan como YMD del selector (día calendario en Costa
  // Rica, ver `SelectorRangoFecha`), pero `hora_entrada` es un `timestamptz`
  // en UTC -- compararlo contra el string crudo lo interpreta a medianoche
  // UTC (no Costa Rica) y, para `hasta`, deja afuera casi todo ese día (sólo
  // calificaría el instante exacto de esa medianoche). Con un rango amplio
  // el corte pasaba desapercibido; con "Hoy"/"Ayer" (mismo día en desde y
  // hasta) el rango resultante quedaba prácticamente vacío siempre. Mismo
  // criterio que `rango_utc` en
  // `desktop/src-tauri/src/comandos/historial.rs`: `hasta` es el inicio del
  // día SIGUIENTE, límite exclusivo.
  if (desde) consulta = consulta.gte("hora_entrada", inicioDiaCostaRicaUtc(desde));
  if (hasta) consulta = consulta.lt("hora_entrada", inicioDiaSiguienteCostaRicaUtc(hasta));
  // `undefined`/vacío es "sin filtro" (todas) -- ver `sitioIdsFiltro` en
  // `Historial.tsx` sobre por qué eso está separado de "excluir todas", que
  // sí manda una lista (vacía) acá y trae cero filas a propósito.
  if (sitioIds) consulta = consulta.in("sitio_id", sitioIds);

  const { data: crudo, error, count } = await consulta;
  if (error) throw new Error(error.message);
  const data = z.array(filaCrudaEsquema).parse(crudo);

  return {
    filas: data.map(({ sitios, dispositivo_entrada, ...resto }) => ({
      ...resto,
      sitio_nombre: sitios?.nombre ?? null,
      dispositivo_entrada_tipo: dispositivo_entrada?.tipo ?? null,
    })),
    truncado: count !== null && count > data.length,
  };
}
