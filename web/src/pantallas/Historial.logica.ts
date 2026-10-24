import { CAMPOS_ORDENABLES } from "../api/historial";
import type { CampoOrdenable, MovimientoHistorial } from "../api/historial";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { generarCsv, generarHtmlTabla } from "../exportacion";
import type { DefinicionColumnaExport as DefinicionColumnaExportGenerica } from "../exportacion";

// Lo que no es componente de `Historial.tsx` (columnas, exportación, orden): vive aparte
// para que el archivo de la pantalla sólo exporte componentes (fast refresh).

/** "pc"/"mobile"/"visor" (`dispositivos.tipo`) → sólo el ícono, para la
 * columna "Dispositivo" -- tanto en pantalla como en Excel/PDF (mismo
 * mapeo que `textoDispositivo` en `desktop/src/pantallas/Historial.tsx` y
 * `HistorialViewModel.kt` del móvil). Antes devolvía ícono + palabra
 * ("💻 PC"/"📱 Celular"), pero quedaba desparejo visualmente -- una palabra
 * bastante más larga que la otra. */
export function textoDispositivo(tipo: string | null): string {
  if (tipo === "pc") return "💻";
  if (tipo === "mobile") return "📱";
  return tipo ?? "—";
}

/**
 * Una sola definición por columna (etiqueta + alineación + cómo sacar el
 * texto de una fila) que alimenta la grilla, el Excel y el PDF -- mismo
 * criterio que `ColumnaHistorial` en `src/historial/exportacion.rs` del
 * lado de escritorio: ahí también hay una sola fuente de verdad para que la
 * tabla en pantalla y las exportaciones no diverjan. `colId` coincide con
 * el `field`/`colId` real de la columna en `columnas` (más abajo), así
 * `tablaRef.current?.columnasVisibles()` (qué mostró/ocultó/reordenó la
 * persona) se puede mapear directo a qué exportar y en qué orden --
 * respetando el filtro/columnas visibles actuales de la grilla, igual que
 * hace escritorio.
 */
export type DefinicionColumnaExport = DefinicionColumnaExportGenerica<MovimientoHistorial>;

export const DEFINICIONES_EXPORT: DefinicionColumnaExport[] = [
  { colId: "sitio_nombre", etiqueta: "Unidad operativa", valor: (f) => f.sitio_nombre ?? "" },
  {
    colId: "contratista_cedula",
    etiqueta: "Cédula",
    izquierda: true,
    valor: (f) => f.contratista_cedula ?? "",
  },
  {
    colId: "contratista_nombre",
    etiqueta: "Nombre",
    izquierda: true,
    valor: (f) => f.contratista_nombre,
  },
  { colId: "empresa_nombre", etiqueta: "Empresa", izquierda: true, valor: (f) => f.empresa_nombre ?? "" },
  {
    colId: "dispositivo_entrada_tipo",
    etiqueta: "Dispositivo",
    valor: (f) => textoDispositivo(f.dispositivo_entrada_tipo),
  },
  { colId: "tipo_ingreso", etiqueta: "Tipo", valor: (f) => f.tipo_texto },
  { colId: "medio_ingreso", etiqueta: "Medio", valor: (f) => f.medio_texto },
  {
    colId: "gafete_numero",
    etiqueta: "Gafete",
    valor: (f) => (f.gafete_numero == null ? "S/G" : String(f.gafete_numero)),
  },
  {
    colId: "fecha_ingreso",
    etiqueta: "Fecha ingreso",
    valor: (f) => textoFechaDDMMYYYY(fechaLocalYMD(f.hora_entrada)),
  },
  { colId: "hora_ingreso", etiqueta: "Hora ingreso", valor: (f) => textoHora(f.hora_entrada) },
  {
    colId: "fecha_salida",
    etiqueta: "Fecha salida",
    valor: (f) => (f.hora_salida ? textoFechaDDMMYYYY(fechaLocalYMD(f.hora_salida)) : "Activo"),
  },
  {
    colId: "hora_salida",
    etiqueta: "Hora salida",
    valor: (f) => (f.hora_salida ? textoHora(f.hora_salida) : "Activo"),
  },
  {
    colId: "usuario_entrada_nombre",
    etiqueta: "Dio ingreso",
    izquierda: true,
    valor: (f) => f.usuario_entrada_nombre ?? "",
  },
  {
    colId: "usuario_salida_nombre",
    etiqueta: "Dio salida",
    izquierda: true,
    valor: (f) => f.usuario_salida_nombre ?? "—",
  },
];

/** PDF del historial (ver `generarHtmlTabla`). */
export function generarHtmlHistorial(
  filas: MovimientoHistorial[],
  columnas: DefinicionColumnaExport[],
  opciones: { generadoPor: string; filtro: string },
): string {
  return generarHtmlTabla(filas, columnas, { titulo: "Historial de Movimientos", ...opciones });
}

/** CSV del historial (ver `generarCsv`). */
export function generarCsvHistorial(filas: MovimientoHistorial[], columnas: DefinicionColumnaExport[]): string {
  return generarCsv(filas, columnas);
}

/** Campo del servidor por el que se ordena al tocar el encabezado de una
 * columna. Las columnas de fecha y hora comparten el mismo instante, y las de
 * tipo y medio ordenan por el texto que se ve ("IN HOUSE", la placa). */
const CAMPO_ORDEN_POR_COLUMNA: Record<string, CampoOrdenable> = {
  fecha_ingreso: "hora_entrada",
  hora_ingreso: "hora_entrada",
  fecha_salida: "hora_salida",
  hora_salida: "hora_salida",
  tipo_ingreso: "tipo_texto",
  medio_ingreso: "medio_texto",
};

export function campoOrdenDeColumna(colId: string): CampoOrdenable | null {
  const mapeado = CAMPO_ORDEN_POR_COLUMNA[colId];
  if (mapeado) return mapeado;
  return (CAMPOS_ORDENABLES as readonly string[]).includes(colId) ? (colId as CampoOrdenable) : null;
}
