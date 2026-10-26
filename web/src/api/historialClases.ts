import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";
import { expresionBusqueda, expresionesDeFiltros } from "./historialFiltros";
import type { ColumnaFiltrable, ModeloFiltros } from "./historialFiltros";

/**
 * Historial de proveedores, visitas por correo y gafetes KOF -- las vistas
 * `panel_movimientos_proveedor`, `_correo` y `_kof` (migración
 * `historial_panel_proveedores_correo_kof`). Mismo camino que el de
 * contratistas (`api/historial.ts`): el servidor filtra, ordena y pagina, y
 * el navegador sólo recibe la página que se ve. Las tres vistas se piden con
 * la misma forma de fila (`MovimientoClase`): lo que cambia de una a otra es
 * de qué columna sale cada dato.
 */
export type ClaseOtra = "proveedores" | "correo" | "kof";

export interface MovimientoClase {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  /** Cédula; en KOF, el código de empleado del encargado. */
  identificacion: string | null;
  nombre: string;
  /** Empresa del proveedor o motivo de la visita por correo; KOF no tiene. */
  detalle?: string | null;
  /** KOF no tiene placa. */
  placa?: string | null;
  gafete_numero: number | null;
  /** En KOF, la entrega del gafete. */
  hora_entrada: string;
  /** En KOF, la devolución. */
  hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
  dispositivo_entrada_tipo: string | null;
}

interface ConfigClase {
  vista: string;
  /** Columnas pedidas, con alias de PostgREST (`identificacion:cedula`) para
   * que las tres clases lleguen con la misma forma. */
  select: string;
  /** `colId` de la grilla → columna de la vista contra la que filtra. */
  filtrables: Record<string, ColumnaFiltrable>;
  /** `colId` de la grilla → columna de la vista por la que ordena. */
  ordenables: Record<string, string>;
  /** Columnas `*_p` del buscador. */
  busqueda: { identificacion: string; nombre: string };
}

const COMUNES_FILTRABLES: Record<string, ColumnaFiltrable> = {
  sitio_nombre: { tipo: "texto", columna: "unidad_p" },
  nombre: { tipo: "texto", columna: "nombre_p" },
  gafete_numero: { tipo: "numero", columna: "gafete_numero" },
  dispositivo_entrada_tipo: { tipo: "texto", columna: "dispositivo_entrada_tipo" },
  fecha_entrada: { tipo: "fecha", columna: "hora_entrada" },
  fecha_salida: { tipo: "fecha", columna: "hora_salida" },
  usuario_entrada_nombre: { tipo: "texto", columna: "usuario_entrada_p" },
  usuario_salida_nombre: { tipo: "texto", columna: "usuario_salida_p" },
};

const COMUNES_ORDENABLES: Record<string, string> = {
  sitio_nombre: "sitio_nombre",
  nombre: "nombre",
  gafete_numero: "gafete_numero",
  dispositivo_entrada_tipo: "dispositivo_entrada_tipo",
  // Fecha y hora comparten el mismo instante.
  fecha_entrada: "hora_entrada",
  hora_entrada: "hora_entrada",
  fecha_salida: "hora_salida",
  hora_salida: "hora_salida",
  usuario_entrada_nombre: "usuario_entrada_nombre",
  usuario_salida_nombre: "usuario_salida_nombre",
};

const COMUNES_SELECT =
  "id, sitio_id, sitio_nombre, nombre, gafete_numero, hora_entrada, hora_salida, " +
  "usuario_entrada_nombre, usuario_salida_nombre, dispositivo_entrada_tipo";

export const CONFIG_CLASES: Record<ClaseOtra, ConfigClase> = {
  proveedores: {
    vista: "panel_movimientos_proveedor",
    select: `${COMUNES_SELECT}, identificacion:cedula, detalle:empresa_nombre, placa`,
    filtrables: {
      ...COMUNES_FILTRABLES,
      identificacion: { tipo: "texto", columna: "cedula_p" },
      detalle: { tipo: "texto", columna: "empresa_p" },
      placa: { tipo: "texto", columna: "placa_p" },
    },
    ordenables: { ...COMUNES_ORDENABLES, identificacion: "cedula", detalle: "empresa_nombre", placa: "placa" },
    busqueda: { identificacion: "cedula_p", nombre: "nombre_p" },
  },
  correo: {
    vista: "panel_movimientos_correo",
    select: `${COMUNES_SELECT}, identificacion:cedula, detalle:motivo, placa`,
    filtrables: {
      ...COMUNES_FILTRABLES,
      identificacion: { tipo: "texto", columna: "cedula_p" },
      detalle: { tipo: "texto", columna: "motivo_p" },
      placa: { tipo: "texto", columna: "placa_p" },
    },
    ordenables: { ...COMUNES_ORDENABLES, identificacion: "cedula", detalle: "motivo", placa: "placa" },
    busqueda: { identificacion: "cedula_p", nombre: "nombre_p" },
  },
  kof: {
    vista: "panel_movimientos_kof",
    select: `${COMUNES_SELECT}, identificacion:codigo_empleado`,
    filtrables: { ...COMUNES_FILTRABLES, identificacion: { tipo: "texto", columna: "codigo_p" } },
    ordenables: { ...COMUNES_ORDENABLES, identificacion: "codigo_empleado" },
    busqueda: { identificacion: "codigo_p", nombre: "nombre_p" },
  },
};

const movimientoEsquema = z.object({
  id: z.string(),
  sitio_id: z.string(),
  sitio_nombre: z.string().nullable(),
  identificacion: z.string().nullable(),
  nombre: z.string(),
  detalle: z.string().nullable().optional(),
  placa: z.string().nullable().optional(),
  gafete_numero: z.number().nullable(),
  hora_entrada: z.string(),
  hora_salida: z.string().nullable(),
  usuario_entrada_nombre: z.string().nullable(),
  usuario_salida_nombre: z.string().nullable(),
  dispositivo_entrada_tipo: z.string().nullable(),
});

export interface ConsultaClase {
  desde?: string;
  hasta?: string;
  /** `undefined` = todas las unidades; lista vacía = ninguna (cero filas). */
  sitioIds?: string[];
  busqueda?: string;
  filtros?: ModeloFiltros;
  /** `colId` de la grilla; se traduce con `ordenables`. */
  orden?: { colId: string; descendente: boolean };
}

/** Columna del servidor por la que ordena la columna `colId` de la grilla, o
 * `null` si esa columna no ordena en el servidor. */
export function campoOrdenClase(clase: ClaseOtra, colId: string): string | null {
  return CONFIG_CLASES[clase].ordenables[colId] ?? null;
}

async function pedirTramo(
  clase: ClaseOtra,
  consulta: ConsultaClase,
  primera: number,
  cantidad: number,
): Promise<MovimientoClase[]> {
  const config = CONFIG_CLASES[clase];
  const campoOrden = consulta.orden ? campoOrdenClase(clase, consulta.orden.colId) : null;
  let peticion = supabase
    .from(config.vista)
    .select(config.select)
    .order(campoOrden ?? "hora_entrada", {
      ascending: campoOrden ? !consulta.orden?.descendente : false,
      nullsFirst: false,
    })
    // Desempate estable entre páginas, como en contratistas.
    .order("id")
    .range(primera, primera + cantidad - 1);

  // Mismo criterio de día calendario de Costa Rica que contratistas.
  if (consulta.desde) peticion = peticion.gte("hora_entrada", inicioDiaCostaRicaUtc(consulta.desde));
  if (consulta.hasta) peticion = peticion.lt("hora_entrada", inicioDiaSiguienteCostaRicaUtc(consulta.hasta));
  if (consulta.sitioIds) peticion = peticion.in("sitio_id", consulta.sitioIds);
  const busqueda = expresionBusqueda(consulta.busqueda ?? "", config.busqueda.identificacion, config.busqueda.nombre);
  // Igual que los filtros de columna: cada expresión es un `.or()` propio.
  if (busqueda) peticion = peticion.or(busqueda);
  for (const expresion of expresionesDeFiltros(consulta.filtros, config.filtrables)) {
    peticion = peticion.or(expresion);
  }

  const { data, error } = await peticion;
  if (error) throw new Error(error.message);
  return z.array(movimientoEsquema).parse(data);
}

export interface PaginaClase {
  filas: MovimientoClase[];
  /** Total sólo cuando la página vino incompleta (ver `PaginaMovimientos`). */
  total?: number;
}

export async function listarPaginaClase(
  clase: ClaseOtra,
  consulta: ConsultaClase,
  pagina: number,
  tamano: number,
): Promise<PaginaClase> {
  const primera = pagina * tamano;
  const filas = await pedirTramo(clase, consulta, primera, tamano);
  return { filas, total: filas.length < tamano ? primera + filas.length : undefined };
}

// Supabase entrega como máximo 1.000 filas por petición.
const TRAMO_EXPORTACION = 1_000;
/** Mismo tope que la exportación de contratistas. */
export const MAXIMO_EXPORTACION_CLASE = 50_000;

/** Todas las filas del filtro y orden actuales, en tramos de 1.000, hasta el
 * máximo de una exportación. */
export async function listarClaseParaExportar(
  clase: ClaseOtra,
  consulta: ConsultaClase,
  maximo: number = MAXIMO_EXPORTACION_CLASE,
): Promise<{ filas: MovimientoClase[]; truncado: boolean }> {
  const filas: MovimientoClase[] = [];
  while (filas.length < maximo) {
    const cantidad = Math.min(TRAMO_EXPORTACION, maximo - filas.length);
    const tramo = await pedirTramo(clase, consulta, filas.length, cantidad);
    filas.push(...tramo);
    if (tramo.length < cantidad) return { filas, truncado: false };
  }
  const siguiente = await pedirTramo(clase, consulta, maximo, 1);
  return { filas, truncado: siguiente.length > 0 };
}
