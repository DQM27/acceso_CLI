import type { ICellRendererParams } from "ag-grid-community";

/** Celda "Adentro" de Contratistas: punto verde y "Sí" con el estilo de
 * estado de las grillas (`.ag-cell .chip`); vacía si no está adentro. */
export default function CeldaAdentro({ value }: ICellRendererParams) {
  if (!value) return null;
  return (
    <span className="chip" style={{ ["--chip-color" as string]: "var(--exito)" }}>
      {String(value)}
    </span>
  );
}
