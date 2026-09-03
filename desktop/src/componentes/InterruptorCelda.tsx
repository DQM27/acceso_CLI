import type { CustomCellRendererProps } from "ag-grid-react";

/**
 * Switch on/off para columnas booleanas editables de las grillas (antes:
 * checkbox nativo de AG Grid, que con el border-radius del tema se veía
 * casi circular y se confundía con un radio button). Un solo click cambia
 * el valor — no hace falta entrar en modo edición como con el checkbox
 * nativo, así que la columna deja de necesitar `editable`/`cellDataType`.
 *
 * `critico` (vía `cellRendererParams`): para columnas que deciden si
 * alguien/algo puede entrar (Acceso, Activa, Activo) — rojo/verde en vez
 * del acento neutro, para no confundirlas con una de puro dato informativo
 * como "Personal de ruta" (ver `.interruptor-critico` en index.css).
 */
export default function InterruptorCelda<TData>({
  value,
  setValue,
  critico,
}: CustomCellRendererProps<TData, boolean> & { critico?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={!!value}
      className={`interruptor${critico ? " interruptor-critico" : ""}${value ? " interruptor-activo" : ""}`}
      onClick={() => setValue?.(!value)}
    />
  );
}
