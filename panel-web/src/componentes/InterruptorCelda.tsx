import type { CustomCellRendererProps } from "ag-grid-react";

/**
 * Switch on/off para columnas booleanas editables de las grillas. Un solo
 * click cambia el valor — no hace falta entrar en modo edición como con el
 * checkbox nativo. Copiado de
 * `desktop/src/componentes/InterruptorCelda.tsx`.
 *
 * `critico` (vía `cellRendererParams`): para columnas que deciden si
 * alguien/algo puede entrar — rojo/verde en vez del acento neutro, para no
 * confundirlas con una de puro dato informativo (ver `.interruptor-critico`
 * en index.css).
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
