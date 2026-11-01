import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import { CalendarDays, CalendarRange, Clock } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Modal from "../componentes/Modal";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { useLista } from "../componentes/useLista";
import { listarFlotaTelemetria, listarRendimiento, listarRendimientoPorVersion } from "../api/telemetriaDatos";
import type { EquipoFlota, RendimientoOperacion, RendimientoVersion, VentanaRendimiento } from "../api/telemetriaDatos";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { estadoEnvioTelemetria, TEXTO_ESTADO_ENVIO, textoMs, textoPorcentajeErrores } from "./Telemetria.logica";

/**
 * Los datos de telemetría de producción en el panel (fase 4 del plan de
 * observabilidad): el estado de la flota y el rendimiento de cada operación
 * por equipo y versión. Las horas se muestran en la del navegador, que en el
 * panel es Costa Rica (UTC-6, sin horario de verano).
 */

function textoFechaHora(iso: string | null): string {
  if (!iso) return "—";
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

function nombreEquipo(etiqueta: string | null, plataforma: string): string {
  return etiqueta ?? (plataforma === "movil" ? "Teléfono sin etiqueta" : "PC sin etiqueta");
}

const textoPlataforma = (plataforma: string) => (plataforma === "movil" ? "Móvil" : "Escritorio");

// ---------------------------------------------------------------------------
// Flota

export function VistaFlota({ controles, busqueda }: { controles: ReactNode; busqueda: string }) {
  const { datos, cargando } = useLista(["telemetria-flota"], listarFlotaTelemetria, {
    intervaloMs: 60_000,
    tablas: "",
  });
  const ahora = new Date();

  const columnas = useMemo<ColDef<EquipoFlota>[]>(
    () => [
      {
        colId: "equipo",
        headerName: "Equipo",
        flex: 1.4,
        minWidth: 160,
        valueGetter: ({ data }) => (data ? nombreEquipo(data.etiqueta, data.plataforma) : ""),
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "estado",
        headerName: "Envío",
        flex: 0.8,
        minWidth: 110,
        valueGetter: ({ data }) => (data ? TEXTO_ESTADO_ENVIO[estadoEnvioTelemetria(data.ultimo_lote_en, ahora)] : ""),
        tooltipValueGetter: () => "Al día: llegó un lote en los últimos 20 min. Atrasado: hasta 2 h. Sin señal: más.",
      },
      {
        colId: "version",
        headerName: "Versión",
        flex: 0.9,
        minWidth: 110,
        valueGetter: ({ data }) =>
          data
            ? `${textoPlataforma(data.plataforma)} ${data.version_app}${data.version_codigo ? ` (${data.version_codigo})` : ""}`
            : "",
      },
      {
        colId: "sistema",
        headerName: "Sistema",
        flex: 1,
        minWidth: 130,
        valueGetter: ({ data }) => (data ? [data.sistema, data.modelo].filter(Boolean).join(" · ") || "—" : ""),
      },
      {
        colId: "ultimo",
        headerName: "Último lote",
        flex: 1,
        minWidth: 140,
        sort: "desc",
        valueGetter: ({ data }) => textoFechaHora(data?.ultimo_lote_en ?? null),
        comparator: (_a, _b, nodoA, nodoB) =>
          (nodoA.data?.ultimo_lote_en ?? "").localeCompare(nodoB.data?.ultimo_lote_en ?? ""),
        filter: false,
      },
      {
        colId: "sesion",
        headerName: "Sesión desde",
        flex: 1,
        minWidth: 140,
        valueGetter: ({ data }) => textoFechaHora(data?.sesion_iniciada_en ?? null),
        filter: false,
      },
      {
        colId: "filas",
        headerName: "Filas 24 h",
        flex: 0.7,
        minWidth: 100,
        valueGetter: ({ data }) => data?.filas_24h ?? 0,
        valueFormatter: ({ value }) => Number(value).toLocaleString("es-CR"),
        filter: false,
      },
      {
        colId: "descartadas",
        headerName: "Descartadas 24 h",
        flex: 0.8,
        minWidth: 120,
        valueGetter: ({ data }) => data?.descartadas_24h ?? 0,
        valueFormatter: ({ value }) => Number(value).toLocaleString("es-CR"),
        tooltipValueGetter: () => "Eventos que la nube no guardó: tipo desconocido, muy viejos o demasiado grandes.",
        filter: false,
      },
    ],
    // `ahora` cambia en cada render a propósito: el estado se recalcula al refrescar.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [datos],
  );

  const filas = datos ?? [];
  return (
    <>
      <Tabla<EquipoFlota>
        key="flota"
        id="telemetria-flota"
        columnas={columnas}
        filas={filas}
        busqueda={busqueda}
        filtrosPorColumna
        idFila={(fila) => fila.dispositivo_id}
        controles={
          <>
            {controles}
            <span role="status">{filas.length.toLocaleString("es-CR")} equipos</span>
          </>
        }
      />
      {cargando && filas.length === 0 && <p className="text-muted">Cargando…</p>}
    </>
  );
}

// ---------------------------------------------------------------------------
// Rendimiento

type ValorVentana = `${VentanaRendimiento}`;

const OPCIONES_VENTANA: OpcionSegmentada<ValorVentana>[] = [
  { valor: "1", Icono: Clock, titulo: "Último día" },
  { valor: "7", Icono: CalendarDays, titulo: "Últimos 7 días" },
  { valor: "30", Icono: CalendarRange, titulo: "Últimos 30 días" },
];

const COLUMNAS_RENDIMIENTO: ColDef<RendimientoOperacion>[] = [
  {
    field: "operacion",
    headerName: "Operación",
    flex: 1.3,
    minWidth: 170,
    cellStyle: { textAlign: "left" },
  },
  {
    colId: "equipo",
    headerName: "Equipo",
    flex: 1.1,
    minWidth: 140,
    valueGetter: ({ data }) => (data ? nombreEquipo(data.etiqueta, data.plataforma) : ""),
  },
  {
    colId: "version",
    headerName: "Versión",
    flex: 0.8,
    minWidth: 110,
    valueGetter: ({ data }) => (data ? `${textoPlataforma(data.plataforma)} ${data.version_app}` : ""),
  },
  {
    field: "llamadas",
    headerName: "Llamadas",
    flex: 0.7,
    minWidth: 95,
    sort: "desc",
    valueFormatter: ({ value }) => Number(value).toLocaleString("es-CR"),
    filter: false,
  },
  {
    colId: "errores",
    headerName: "Errores",
    flex: 0.7,
    minWidth: 95,
    valueGetter: ({ data }) => (data && data.llamadas > 0 ? data.errores / data.llamadas : 0),
    valueFormatter: ({ data }) => (data ? textoPorcentajeErrores(data.llamadas, data.errores) : ""),
    filter: false,
  },
  {
    field: "p50_ms",
    headerName: "Típico (p50)",
    flex: 0.8,
    minWidth: 110,
    valueFormatter: ({ value }) => textoMs(value as number | null),
    filter: false,
  },
  {
    field: "p90_ms",
    headerName: "Lento (p90)",
    flex: 0.8,
    minWidth: 110,
    valueFormatter: ({ value }) => textoMs(value as number | null),
    filter: false,
  },
  {
    field: "max_ms",
    headerName: "Máximo",
    flex: 0.7,
    minWidth: 100,
    valueFormatter: ({ value }) => textoMs(value as number | null),
    filter: false,
  },
];

export function VistaRendimiento({ controles, busqueda }: { controles: ReactNode; busqueda: string }) {
  const [ventana, setVentana] = useState<ValorVentana>("1");
  const [operacion, setOperacion] = useState<string | null>(null);
  const dias = Number(ventana) as VentanaRendimiento;
  const { datos, cargando } = useLista(["telemetria-rendimiento", dias], () => listarRendimiento(dias), {
    intervaloMs: 120_000,
    tablas: "",
  });
  const filas = datos ?? [];

  return (
    <>
      <Tabla<RendimientoOperacion>
        key="rendimiento"
        id="telemetria-rendimiento"
        columnas={COLUMNAS_RENDIMIENTO}
        filas={filas}
        busqueda={busqueda}
        filtrosPorColumna
        idFila={(fila) => `${fila.dispositivo_id}|${fila.version_app}|${fila.operacion}`}
        onFilaDobleClic={(fila) => setOperacion(fila.operacion)}
        controles={
          <>
            {controles}
            <SegmentadoOpciones opciones={OPCIONES_VENTANA} valor={ventana} onCambiar={setVentana} etiqueta="Período" />
          </>
        }
        accionesDerecha={
          <span className="text-sm text-(--muted)">Doble clic en una operación: cómo le fue en cada versión</span>
        }
      />
      {cargando && filas.length === 0 && <p className="text-muted">Cargando…</p>}
      {operacion && <PorVersion operacion={operacion} onCerrar={() => setOperacion(null)} />}
    </>
  );
}

/** Una operación en cada versión, todos los equipos juntos, del último año. */
function PorVersion({ operacion, onCerrar }: { operacion: string; onCerrar: () => void }) {
  const { datos, cargando } = useLista(
    ["telemetria-versiones", operacion],
    () => listarRendimientoPorVersion(operacion),
    { intervaloMs: 300_000, tablas: "" },
  );
  const filas: RendimientoVersion[] = datos ?? [];
  return (
    <Modal titulo={`${operacion}: por versión`} onCerrar={onCerrar}>
      <div className="flex flex-col gap-3">
        <p className="text-sm text-(--muted)">
          Del resumen diario del último año (se arma de madrugada: el día de hoy todavía no está).
        </p>
        {cargando && filas.length === 0 && <p className="text-muted">Cargando…</p>}
          {!cargando && filas.length === 0 && <p>Todavía no hay datos resumidos de esta operación.</p>}
        {filas.length > 0 && (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left">
                <th>Versión</th>
                <th>Días</th>
                <th>Equipos</th>
                <th className="text-right">Llamadas</th>
                <th className="text-right">Errores</th>
                <th className="text-right">p50</th>
                <th className="text-right">p90</th>
                <th className="text-right">Máximo</th>
              </tr>
            </thead>
            <tbody>
              {filas.map((fila) => (
                <tr key={`${fila.plataforma}|${fila.version_app}`}>
                  <td>
                    {textoPlataforma(fila.plataforma)} {fila.version_app}
                  </td>
                  <td>
                    {textoFechaDDMMYYYY(fila.primer_dia)} – {textoFechaDDMMYYYY(fila.ultimo_dia)}
                  </td>
                  <td>{fila.equipos}</td>
                  <td className="text-right">{fila.llamadas.toLocaleString("es-CR")}</td>
                  <td className="text-right">{textoPorcentajeErrores(fila.llamadas, fila.errores)}</td>
                  <td className="text-right">{textoMs(fila.p50_ms)}</td>
                  <td className="text-right">{textoMs(fila.p90_ms)}</td>
                  <td className="text-right">{textoMs(fila.max_ms)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </Modal>
  );
}
