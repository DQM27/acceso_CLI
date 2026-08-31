import type { CustomCellRendererProps } from "ag-grid-react";

/**
 * Switch on/off para columnas booleanas editables de las grillas (antes:
 * checkbox nativo de AG Grid, que con el border-radius del tema se veía
 * casi circular y se confundía con un radio button). Un solo click cambia
 * el valor — no hace falta entrar en modo edición como con el checkbox
 * nativo, así que la columna deja de necesitar `editable`/`cellDataType`.
 */
export default function InterruptorCelda<TData>({
  value,
  setValue,
}: CustomCellRendererProps<TData, boolean>) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={!!value}
      className={`interruptor${value ? " interruptor-activo" : ""}`}
      onClick={() => setValue?.(!value)}
    />
  );
}
