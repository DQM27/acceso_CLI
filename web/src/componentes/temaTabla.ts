import { themeQuartz } from "ag-grid-community";

/**
 * Tema de TODAS las grillas de la app (Tabla.tsx y la tabla del lienzo): las
 * mismas custom properties que el resto de la interfaz (index.css) en vez de
 * colores fijos, para que la grilla siga el tema claro/oscuro del sistema.
 */
export const temaBrisas = themeQuartz.withParams({
  backgroundColor: "var(--panel)",
  foregroundColor: "var(--texto)",
  headerBackgroundColor: "var(--panel-suave)",
  headerTextColor: "var(--muted)",
  borderColor: "var(--borde)",
  fontFamily: "var(--fuente)",
  accentColor: "var(--acento)",
  selectedRowBackgroundColor: "var(--acento-suave)",
  // Sin cebra: con filas de 36 px y líneas entre filas alcanza, y así los
  // resaltados se distinguen de una fila normal (auditoría 2026-11-14, U7).
  borderRadius: "var(--radio-chico)",
  wrapperBorderRadius: "var(--radio)",
});
