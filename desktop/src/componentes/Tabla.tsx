import { useMemo, useState } from "react";
import { AgGridReact } from "ag-grid-react";
import { themeQuartz } from "ag-grid-community";
import type { ColDef } from "ag-grid-community";

/**
 * Tema y comportamiento compartido de TODAS las tablas de la app — un solo
 * punto para cambiar cómo se ven/comportan las grillas. Paleta igual a
 * index.css (FADE_* de src/comandos/render/estilos.rs).
 *
 * Deliberadamente NO expone cualquier prop de AgGridReact — sólo columnas y
 * filas. Mostrar/ocultar columnas sí vive acá como capacidad permanente
 * (no un flag opcional) porque ya es un patrón establecido en varias
 * pantallas de comandos/TUI (Contratistas, Historial) y se espera que se
 * repita en el resto. Otras funciones de grilla (paginación server-side,
 * exportar, selección de filas) se agregan recién cuando una pantalla real
 * las necesite — no antes.
 */
const temaBrisas = themeQuartz.withParams({
  backgroundColor: "#0a0a0c",
  foregroundColor: "#e1e1e6",
  headerBackgroundColor: "#16161a",
  borderColor: "#2a2a30",
  accentColor: "#56c8d6",
  oddRowBackgroundColor: "#111114",
  borderRadius: 8,
  wrapperBorderRadius: 8,
});

const columnaPorDefecto: ColDef = {
  sortable: true,
  resizable: true,
  minWidth: 90,
};

export interface TablaProps<T> {
  columnas: ColDef<T>[];
  filas: T[];
}

export default function Tabla<T>({ columnas, filas }: TablaProps<T>) {
  const [ocultas, setOcultas] = useState<Set<string>>(new Set());
  const [selectorAbierto, setSelectorAbierto] = useState(false);

  const columnasConVisibilidad = useMemo(
    () =>
      columnas.map((columna) =>
        typeof columna.field === "string"
          ? { ...columna, hide: ocultas.has(columna.field) }
          : columna,
      ),
    [columnas, ocultas],
  );

  function alternar(campo: string) {
    setOcultas((actual) => {
      const siguiente = new Set(actual);
      if (siguiente.has(campo)) {
        siguiente.delete(campo);
      } else {
        siguiente.add(campo);
      }
      return siguiente;
    });
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          position: "relative",
          display: "flex",
          justifyContent: "flex-end",
          marginBottom: "0.5rem",
        }}
      >
        <button type="button" className="boton" onClick={() => setSelectorAbierto((a) => !a)}>
          Columnas ▾
        </button>

        {selectorAbierto && (
          <>
            {/* Backdrop invisible: cierra el selector al hacer click afuera. */}
            <div
              onClick={() => setSelectorAbierto(false)}
              style={{ position: "fixed", inset: 0, zIndex: 9 }}
            />
            <div
              className="tarjeta"
              style={{
                position: "absolute",
                top: "2.35rem",
                right: 0,
                zIndex: 10,
                padding: "0.75rem 1rem",
                display: "flex",
                flexDirection: "column",
                gap: "0.4rem",
                minWidth: "13rem",
              }}
            >
              {columnas
                .filter((columna): columna is ColDef<T> & { field: string } =>
                  typeof columna.field === "string",
                )
                .map((columna) => (
                  <label
                    key={columna.field}
                    style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}
                  >
                    <input
                      type="checkbox"
                      checked={!ocultas.has(columna.field)}
                      onChange={() => alternar(columna.field)}
                    />
                    {columna.headerName ?? columna.field}
                  </label>
                ))}
            </div>
          </>
        )}
      </div>

      <div style={{ flex: 1, minHeight: 0 }}>
        <AgGridReact<T>
          theme={temaBrisas}
          defaultColDef={columnaPorDefecto}
          rowData={filas}
          columnDefs={columnasConVisibilidad}
          rowHeight={36}
          headerHeight={38}
        />
      </div>
    </div>
  );
}
