import { invoke } from "@tauri-apps/api/core";
import type { MedioIngreso, ResultadoIngresoRegistrado } from "./ingresos";
import type { TipoIngreso } from "./contratistas";

// Espejo de comandos/historial.rs y
// src/database/queries/ingresos/historial.rs (`MovimientoIngresoResumen`)
// del núcleo. Sin paginado a propósito: la pantalla trae todo el historial
// de una vez y deja que AG Grid filtre/ordene/virtualice del lado del
// cliente (igual criterio que Activos, ver `Historial.tsx`).
//
// `ResultadoIngresoRegistrado` se reusa de `api/ingresos.ts` — mismo enum
// del núcleo (`src/models/registro_ingreso.rs`), no una copia.

export type MotivoResultadoIngreso = "PraindProximoVencer" | "DatosReconstruidos";

export interface MovimientoIngresoResumen {
  registro_id: number;
  contratista_id: number;
  cedula: string;
  contratista_nombre: string;
  empresa_nombre: string;
  tipo_ingreso: TipoIngreso;
  medio_ingreso: MedioIngreso;
  /** ISO 8601 (UTC) — convertir con `new Date(...)` antes de mostrar. */
  fecha_hora_ingreso: string;
  /** ISO 8601 (UTC), `null` si el movimiento sigue activo (sin salida). */
  fecha_hora_salida: string | null;
  gafete_numero: number | null;
  usuario_ingreso_nombre: string;
  usuario_salida_nombre: string | null;
  resultado_acceso: ResultadoIngresoRegistrado;
  motivo_resultado: MotivoResultadoIngreso | null;
  reglas_version: number;
  empresa_activa_snapshot: boolean;
}

export function mensajeResultado(fila: MovimientoIngresoResumen): string {
  const r = fila.resultado_acceso;
  if (r === "Permitido") return "Permitido";
  if (r === "Migrado") return "Migrado";
  return mensajeMotivoResultado(r.PermitidoConAdvertencia);
}

function mensajeMotivoResultado(motivo: MotivoResultadoIngreso): string {
  switch (motivo) {
    case "PraindProximoVencer":
      return "PRAIND próximo a vencer";
    case "DatosReconstruidos":
      return "Datos reconstruidos";
  }
}

/** `desde`/`hasta`: `"YYYY-MM-DD"` (calendario, Costa Rica) o `undefined`
 * para no acotar ese extremo — ver `rango_utc` en
 * `desktop/src-tauri/src/comandos/historial.rs`. `hasta` es inclusivo del
 * día completo. */
export function listarHistorial(
  desde?: string,
  hasta?: string,
): Promise<MovimientoIngresoResumen[]> {
  return invoke("listar_historial", { desde: desde ?? null, hasta: hasta ?? null });
}

/** `ids`: los `registro_id` que la grilla tiene visibles tras su propio
 * filtro por columna. `columnas`: claves de `ColumnaHistorial::clave`
 * (núcleo, `src/historial/exportacion.rs`) de las columnas que la grilla
 * tiene visibles ahora — ver `CLAVES_COLUMNA` en `Historial.tsx` para el
 * mapeo colId → clave. Ambos son filtros del lado del cliente (AG Grid);
 * el núcleo no los conoce por su cuenta, así que se le pasa el recorte ya
 * resuelto en vez de siempre exportar todo el historial sin acotar.
 * `desde`/`hasta`: el mismo rango que ya filtró `listarHistorial` para traer
 * `ids` — se manda de nuevo para que la consulta SQL de la exportación
 * quede acotada a ese rango en vez de escanear todo el historial acumulado
 * y recortar recién después por `ids`. */
export function exportarHistorial(
  destino: string,
  ids: number[],
  columnas: string[],
  desde?: string,
  hasta?: string,
): Promise<number> {
  return invoke("exportar_historial", {
    destino,
    ids,
    columnas,
    desde: desde ?? null,
    hasta: hasta ?? null,
  });
}

/** Mismo recorte que `exportarHistorial` (`ids`/`columnas`/`desde`/`hasta`),
 * a PDF en vez de Excel. `filtroDescripcion`: texto ya formateado para el
 * encabezado del PDF (ver `textoRangoFecha` en `SelectorRangoFecha.tsx`) —
 * el backend no recalcula el formateo de fechas, sólo lo muestra tal cual.
 * `generadoPor` no se manda: el backend lo saca de la sesión activa, no del
 * cliente. */
export function exportarHistorialPdf(
  destino: string,
  ids: number[],
  columnas: string[],
  filtroDescripcion: string,
  desde?: string,
  hasta?: string,
): Promise<void> {
  return invoke("exportar_historial_pdf", {
    destino,
    ids,
    columnas,
    filtroDescripcion,
    desde: desde ?? null,
    hasta: hasta ?? null,
  });
}
