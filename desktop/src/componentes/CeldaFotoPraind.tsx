import type { ICellRendererParams } from "ag-grid-community";

/** Celda "Foto PRAIND" de Contratistas: punto verde con "Sí" si la foto está
 * cargada en la nube, gris con "No" si falta. Mismo estilo de estado que
 * "Adentro" (`.ag-cell .chip`). */
export default function CeldaFotoPraind({ value }: ICellRendererParams) {
  if (!value) return null;
  return (
    <span
      className="chip"
      style={{ ["--chip-color" as string]: value === "Sí" ? "var(--exito)" : "var(--muted)" }}
    >
      {String(value)}
    </span>
  );
}
