import { useCallback, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { exportarHistorial, listarHistorial, textoMedio } from "../api";
import type { MovimientoIngresoResumen } from "../api";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

type FilaHistorial = MovimientoIngresoResumen;

/** colId/field de la grilla → clave de `ColumnaHistorial` en el núcleo
 * (`src/historial/exportacion.rs`, `ColumnaHistorial::clave`) — así el
 * export sabe qué columnas del XLSX corresponden a las que el usuario
 * dejó visibles acá. "Fecha salida" no tiene equivalente propio en el
 * núcleo salvo `fecha_salida` (agregado junto con esta columna); ambas
 * hoy están 1 a 1. */
const CLAVES_COLUMNA: Record<string, string> = {
  cedula: "cedula",
  contratista_nombre: "nombre",
  empresa_nombre: "empresa",
  tipo_ingreso: "tipo",
  medio_ingreso: "medio",
  gafete_numero: "gafete",
  fecha_ingreso: "fecha",
  hora_ingreso: "entrada",
  fecha_salida: "fecha_salida",
  hora_salida: "salida",
  usuario_ingreso_nombre: "ingreso",
  usuario_salida_nombre: "egreso",
};

export default function Historial() {
  const [filas, setFilas] = useState<FilaHistorial[]>([]);
  const [cargando, setCargando] = useState(true);
  const [exportando, setExportando] = useState(false);
  // Por defecto trae los últimos 6 meses — antes traía todo desde el año
  // 2000 (rango fijo en el backend). `hasta` vacío queda abierto (hoy + 1
  // día en el backend, ver `rango_utc`), así no se pierden movimientos del
  // día en curso. El usuario puede ampliar `desde` para ver más atrás.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const tablaRef = useRef<TablaHandle<FilaHistorial>>(null);

  const recargar = useCallback(async () => {
    setCargando(true);
    try {
      setFilas(await listarHistorial(desde || undefined, hasta || undefined));
    } finally {
      setCargando(false);
    }
  }, [desde, hasta]);
  useCargaAlCambiar(recargar);

  async function exportar() {
    // Lo que la grilla tiene visible AHORA (filtro por columna y selector
    // "Columnas ▾" de AG Grid, ambos del lado del cliente) — exportar
    // respeta ese recorte en vez de siempre mandar todo el historial sin
    // acotar.
    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    if (visibles.length === 0) {
      toast.error("No hay filas para exportar con el filtro actual.");
      return;
    }
    const claves = (tablaRef.current?.columnasVisibles() ?? Object.keys(CLAVES_COLUMNA))
      .map((colId) => CLAVES_COLUMNA[colId])
      .filter((clave): clave is string => clave !== undefined);
    if (claves.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }

    const destino = await save({
      title: "Exportar historial a Excel",
      defaultPath: "historial.xlsx",
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });
    if (!destino) return;

    setExportando(true);
    toast.promise(
      exportarHistorial(
        destino,
        visibles.map((fila) => fila.registro_id),
        claves,
      ).finally(() => setExportando(false)),
      {
        loading: "Exportando…",
        success: (cantidad) => `${cantidad} fila(s) exportadas.`,
        error: (error) => String(error),
      },
    );
  }

  // useMemo a propósito — mismo motivo que Activos.tsx: si `columnas` se
  // recrea en cada render, AG Grid reaplica el orden/ancho literales de acá
  // encima del layout que el usuario ya acomodó (persistido en localStorage
  // vía `Tabla`).
  const columnas: ColDef<FilaHistorial>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", width: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 130 },
      { field: "tipo_ingreso", headerName: "Tipo", width: 100 },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        width: 100,
        valueFormatter: (p) => textoMedio(p.value),
      },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        width: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_ingreso",
        headerName: "Fecha ingreso",
        width: 120,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        width: 110,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      {
        // "Activo" (no sólo vacío) cuando no hay salida — mismo texto que
        // ya escribe la exportación a Excel (`escribir_movimiento`,
        // `src/historial/exportacion.rs`) para un movimiento sin
        // `fecha_hora_salida`; antes la grilla dejaba la celda en blanco y
        // no coincidía con lo que se veía en el archivo exportado.
        colId: "fecha_salida",
        headerName: "Fecha salida",
        width: 120,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? fechaLocalYMD(p.data.fecha_hora_salida) : "Activo",
        valueFormatter: (p) => (p.value === "Activo" ? "Activo" : textoFechaDDMMYYYY(p.value)),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        width: 110,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : "Activo",
      },
      { field: "usuario_ingreso_nombre", headerName: "Dio ingreso", width: 130 },
      { field: "usuario_salida_nombre", headerName: "Dio salida", width: 130 },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado
        titulo="Historial"
        acciones={
          <>
            <label style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
              Desde
              <input
                type="date"
                value={desde}
                max={hasta || undefined}
                onChange={(e) => setDesde(e.target.value)}
              />
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "0.35rem" }}>
              Hasta
              <input
                type="date"
                value={hasta}
                min={desde || undefined}
                onChange={(e) => setHasta(e.target.value)}
              />
            </label>
            <button
              className="boton"
              title="Exporta lo que está filtrado en la grilla, no todo el historial"
              onClick={exportar}
              disabled={exportando}
            >
              {exportando ? "Exportando…" : "Exportar a Excel"}
            </button>
          </>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaHistorial>
            ref={tablaRef}
            id="historial"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {cargando ? "Cargando…" : `${filas.length} movimiento(s)`}
        </p>
      </div>
    </div>
  );
}
