import { useCallback, useMemo, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { FileSpreadsheet, FileText } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import SelectorRangoFecha, { textoRangoFecha } from "../componentes/SelectorRangoFecha";
import { exportarHistorial, exportarHistorialPdf, listarHistorial, textoMedio } from "../api";
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
  // `true` cuando el rango actual supera el tope de carga completa del
  // núcleo (`LIMITE_CARGA_COMPLETA_MAXIMO`, `CargaCompleta.truncado`) — la
  // grilla sigue funcionando exactamente igual (client-side, filtro por
  // columna instantáneo) sobre las filas que sí trajo, pero el filtro por
  // columna y "exportar lo visible" dejan de representar el rango
  // completo. Mientras el total esté bajo el tope (el caso normal) nada de
  // esto se activa — un solo camino de código, sin modo aparte que
  // mantener.
  const [truncado, setTruncado] = useState(false);
  // Por defecto trae los últimos 6 meses — antes traía todo desde el año
  // 2000 (rango fijo en el backend). `hasta` vacío queda abierto (hoy + 1
  // día en el backend, ver `rango_utc`), así no se pierden movimientos del
  // día en curso. El usuario puede ampliar `desde` para ver más atrás.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const tablaRef = useRef<TablaHandle<FilaHistorial>>(null);

  useBarraEstado(
    cargando
      ? "Cargando…"
      : truncado
        ? `${filas.length}+ movimiento(s) (rango truncado)`
        : `${filas.length} movimiento(s)`,
  );

  const recargar = useCallback(
    async (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      try {
        const { items, truncado } = await listarHistorial(desde || undefined, hasta || undefined);
        if (!estaVigente()) return;
        setFilas(items);
        setTruncado(truncado);
      } finally {
        if (estaVigente()) setCargando(false);
      }
    },
    [desde, hasta],
  );
  useCargaAlCambiar(recargar);

  // Lo que la grilla tiene visible AHORA (filtro por columna y selector
  // "Columnas ▾" de AG Grid, ambos del lado del cliente) — Excel y PDF
  // exportan ese mismo recorte en vez de siempre mandar todo el historial
  // sin acotar. Devuelve `null` (con el toast de error ya disparado) si no
  // hay nada exportable, para que quien llama corte ahí sin duplicar el
  // chequeo. Cuando `truncado` es `true`, el cliente sólo tiene una
  // porción del rango — `ids: null` le dice al backend que exporte todo
  // `desde`/`hasta` directo de la base en vez de la porción cargada (ver
  // `exportarHistorial`/`exportarHistorialPdf`); el filtro por columna deja
  // de aplicar ahí porque ya no puede evaluarse sobre el total real.
  function seleccionParaExportar(): { ids: number[] | null; claves: string[] } | null {
    const claves = (tablaRef.current?.columnasVisibles() ?? Object.keys(CLAVES_COLUMNA))
      .map((colId) => CLAVES_COLUMNA[colId])
      .filter((clave): clave is string => clave !== undefined);
    if (claves.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return null;
    }
    if (truncado) return { ids: null, claves };

    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    if (visibles.length === 0) {
      toast.error("No hay filas para exportar con el filtro actual.");
      return null;
    }
    return { ids: visibles.map((fila) => fila.registro_id), claves };
  }

  async function exportar() {
    const seleccion = seleccionParaExportar();
    if (!seleccion) return;

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
        seleccion.ids,
        seleccion.claves,
        desde || undefined,
        hasta || undefined,
      ).finally(() => setExportando(false)),
      {
        loading: "Exportando…",
        success: (cantidad) => `${cantidad} fila(s) exportadas.`,
        error: (error) => String(error),
      },
    );
  }

  async function exportarPdf() {
    const seleccion = seleccionParaExportar();
    if (!seleccion) return;

    const destino = await save({
      title: "Exportar historial a PDF",
      defaultPath: "historial.pdf",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (!destino) return;

    setExportando(true);
    toast.promise(
      exportarHistorialPdf(
        destino,
        seleccion.ids,
        seleccion.claves,
        `Filtro: ${textoRangoFecha(desde, hasta)}`,
        desde || undefined,
        hasta || undefined,
      ).finally(() => setExportando(false)),
      {
        loading: "Exportando…",
        success: "PDF exportado.",
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
      { field: "cedula", headerName: "Cédula", flex: 1.2, minWidth: 120, cellStyle: { textAlign: "left" } },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 130 },
      { field: "tipo_ingreso", headerName: "Tipo", flex: 1, minWidth: 100 },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueFormatter: (p) => textoMedio(p.value),
      },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_ingreso",
        headerName: "Fecha ingreso",
        // 120 truncaba el título en mayúscula ("FECHA ING…") mientras el
        // resto de encabezados entraba completo — 140 es lo que necesita
        // "FECHA INGRESO" para no cortarse.
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 1.3,
        minWidth: 130,
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
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? fechaLocalYMD(p.data.fecha_hora_salida) : "Activo",
        valueFormatter: (p) => (p.value === "Activo" ? "Activo" : textoFechaDDMMYYYY(p.value)),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) =>
          p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : "Activo",
      },
      { field: "usuario_ingreso_nombre", headerName: "Dio ingreso", flex: 1.3, minWidth: 130 },
      { field: "usuario_salida_nombre", headerName: "Dio salida", flex: 1.3, minWidth: 130 },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        {truncado && (
          <p
            role="status"
            style={{
              margin: "0 0 0.5rem",
              padding: "0.5rem 0.75rem",
              borderRadius: "var(--radio-chico)",
              border: "1px solid var(--advertencia)",
              color: "var(--advertencia)",
              fontSize: "0.85rem",
            }}
          >
            Este rango tiene más de {filas.length.toLocaleString("es-CR")} movimientos — se
            muestran solo los primeros. El filtro por columna sólo aplica a lo cargado; acotá las
            fechas para verlo todo, o exportá igual: Excel y PDF traen el rango completo aunque no
            esté cargado en pantalla.
          </p>
        )}
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaHistorial>
            ref={tablaRef}
            id="historial"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            accionesDerecha={
              <>
                <SelectorRangoFecha
                  desde={desde}
                  hasta={hasta}
                  onAplicar={(nuevoDesde, nuevoHasta) => {
                    setDesde(nuevoDesde);
                    setHasta(nuevoHasta);
                  }}
                />
                <button
                  type="button"
                  className="boton boton-icono"
                  title={
                    exportando
                      ? "Exportando…"
                      : truncado
                        ? "Exportar a Excel — trae todo el rango de fechas, no sólo lo cargado"
                        : "Exportar a Excel — respeta el filtro/orden/columnas actuales de la grilla"
                  }
                  onClick={exportar}
                  disabled={exportando}
                >
                  <FileSpreadsheet size={16} />
                </button>
                <button
                  type="button"
                  className="boton boton-icono"
                  title={
                    exportando
                      ? "Exportando…"
                      : truncado
                        ? "Exportar a PDF — trae todo el rango de fechas, no sólo lo cargado"
                        : "Exportar a PDF — respeta el filtro/orden/columnas actuales de la grilla"
                  }
                  onClick={exportarPdf}
                  disabled={exportando}
                >
                  <FileText size={16} />
                </button>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
