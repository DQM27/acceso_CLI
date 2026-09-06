import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { FileSpreadsheet, FileText } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha, { textoRangoFecha } from "../componentes/SelectorRangoFecha";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { useAuth } from "../contexto/AuthContexto";
import { listarHistorial } from "../api/historial";
import type { MovimientoHistorial } from "../api/historial";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

export function textoMedio(medio: string | null): string {
  if (medio === "CAMINANDO") return "Caminando";
  if (medio === "VEHICULO") return "Vehículo";
  return "";
}

/** "pc"/"mobile"/"visor" (`dispositivos.tipo`) → sólo el ícono, para la
 * columna "Dispositivo" en pantalla -- mismo mapeo que `textoDispositivo`
 * en `desktop/src/pantallas/Historial.tsx` y `HistorialViewModel.kt` del
 * móvil (esas sí devuelven ícono + palabra, no exportan a Excel/PDF). Acá
 * ver [`textoDispositivoExport`] para Excel/PDF, donde un emoji solo no
 * sirve. */
export function textoDispositivo(tipo: string | null): string {
  if (tipo === "pc") return "💻";
  if (tipo === "mobile") return "📱";
  return tipo ?? "—";
}

/** Versión con palabra completa de [`textoDispositivo`] -- la pantalla
 * quiere sólo el ícono (más parejo visualmente que "💻 PC"/"📱 Celular"),
 * pero un reporte impreso o una celda de Excel sí necesita texto legible,
 * no un glifo solo. */
export function textoDispositivoExport(tipo: string | null): string {
  if (tipo === "pc") return "💻 PC";
  if (tipo === "mobile") return "📱 Celular";
  return tipo ?? "—";
}

/**
 * Una sola definición por columna (etiqueta + alineación + cómo sacar el
 * texto de una fila) que alimenta la grilla, el Excel y el PDF -- mismo
 * criterio que `ColumnaHistorial` en `src/historial/exportacion.rs` del
 * lado de escritorio: ahí también hay una sola fuente de verdad para que la
 * tabla en pantalla y las exportaciones no diverjan. `colId` coincide con
 * el `field`/`colId` real de la columna en `columnas` (más abajo), así
 * `tablaRef.current?.columnasVisibles()` (qué mostró/ocultó/reordenó la
 * persona) se puede mapear directo a qué exportar y en qué orden --
 * respetando el filtro/columnas visibles actuales de la grilla, igual que
 * hace escritorio.
 */
export interface DefinicionColumnaExport {
  colId: string;
  etiqueta: string;
  izquierda?: boolean;
  valor: (fila: MovimientoHistorial) => string;
}

export const DEFINICIONES_EXPORT: DefinicionColumnaExport[] = [
  { colId: "sitio_nombre", etiqueta: "Sitio", valor: (f) => f.sitio_nombre ?? "" },
  {
    colId: "contratista_cedula",
    etiqueta: "Cédula",
    izquierda: true,
    valor: (f) => f.contratista_cedula ?? "",
  },
  {
    colId: "contratista_nombre",
    etiqueta: "Nombre",
    izquierda: true,
    valor: (f) => f.contratista_nombre,
  },
  { colId: "empresa_nombre", etiqueta: "Empresa", izquierda: true, valor: (f) => f.empresa_nombre ?? "" },
  {
    colId: "dispositivo_entrada_tipo",
    etiqueta: "Dispositivo",
    valor: (f) => textoDispositivoExport(f.dispositivo_entrada_tipo),
  },
  { colId: "tipo_ingreso", etiqueta: "Tipo", valor: (f) => f.tipo_ingreso ?? "" },
  { colId: "medio_ingreso", etiqueta: "Medio", valor: (f) => textoMedio(f.medio_ingreso) },
  {
    colId: "gafete_numero",
    etiqueta: "Gafete",
    valor: (f) => (f.gafete_numero == null ? "S/G" : String(f.gafete_numero)),
  },
  {
    colId: "fecha_ingreso",
    etiqueta: "Fecha ingreso",
    valor: (f) => textoFechaDDMMYYYY(fechaLocalYMD(f.hora_entrada)),
  },
  { colId: "hora_ingreso", etiqueta: "Hora ingreso", valor: (f) => textoHora(f.hora_entrada) },
  {
    colId: "fecha_salida",
    etiqueta: "Fecha salida",
    valor: (f) => (f.hora_salida ? textoFechaDDMMYYYY(fechaLocalYMD(f.hora_salida)) : "Activo"),
  },
  {
    colId: "hora_salida",
    etiqueta: "Hora salida",
    valor: (f) => (f.hora_salida ? textoHora(f.hora_salida) : "Activo"),
  },
  {
    colId: "usuario_entrada_nombre",
    etiqueta: "Dio ingreso",
    izquierda: true,
    valor: (f) => f.usuario_entrada_nombre ?? "",
  },
  {
    colId: "usuario_salida_nombre",
    etiqueta: "Dio salida",
    izquierda: true,
    valor: (f) => f.usuario_salida_nombre ?? "—",
  },
];

/**
 * Historial multi-sitio para `admin_global` -- lee `ingresos` en Supabase
 * (ver `api/historial.ts` y la migración `agrega_columnas_historial_a_ingresos`),
 * no la base local de un sitio en particular como la versión de escritorio.
 * "Exportar a Excel" es client-side (SheetJS) en vez del exportador de
 * Rust/Tauri de `desktop/` -- este panel no tiene un proceso nativo del
 * lado del navegador que escriba el archivo.
 */
/** Escapa lo mínimo indispensable para HTML -- mismo motivo que
 * `escapar` en `desktop/src-tauri/src/pdf/html.rs`: nombres/empresas
 * reales nunca se validaron como "sin `<`/`&`". */
function escaparHtml(texto: string): string {
  return texto
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** Misma paleta clara y layout que `desktop/src-tauri/src/pdf/html.rs`
 * (título + subtítulo de filtro a la izquierda, "Generado por"/fecha a la
 * derecha, tabla en cebra, horizontal/landscape) -- así el PDF que sale de
 * la web se ve igual al que ya conocen del escritorio, aunque el motor que
 * lo imprime sea el navegador en vez de WebView2. */
export function generarHtmlHistorial(
  filas: MovimientoHistorial[],
  columnas: DefinicionColumnaExport[],
  opciones: { generadoPor: string; filtro: string },
): string {
  const encabezados = columnas
    .map((c) => `<th${c.izquierda ? ' class="izquierda"' : ""}>${escaparHtml(c.etiqueta.toUpperCase())}</th>`)
    .join("");

  const filasHtml = filas
    .map((fila) => {
      const celdas = columnas
        .map((c) => `<td${c.izquierda ? ' class="izquierda"' : ""}>${escaparHtml(c.valor(fila))}</td>`)
        .join("");
      return `<tr>${celdas}</tr>`;
    })
    .join("");

  const generadoEn = new Date().toLocaleString("es-CR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

  return `<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8" />
<title>Historial de Movimientos</title>
<style>
  :root {
    --acento: #087f91;
    --texto: #172026;
    --muted: #63717c;
    --borde: #d8e0e5;
    --panel-suave: #eef3f5;
    --zebra: #d9eaf7;
  }
  @page { size: letter landscape; margin: 1.1cm 0.9cm; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: Arial, Helvetica, sans-serif;
    font-size: 8.5pt;
    color: var(--texto);
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 2px solid var(--acento);
    padding: 0 0.3cm 0.5em;
    margin-bottom: 0.7em;
  }
  header h1 { margin: 0; font-size: 14pt; color: var(--acento); }
  header .subtitulo { margin: 0.15em 0 0; font-size: 8.5pt; color: var(--muted); }
  .meta { text-align: right; font-size: 8pt; color: var(--muted); line-height: 1.5; }
  .meta strong { color: var(--texto); }
  table { width: 100%; border-collapse: collapse; }
  thead { display: table-header-group; }
  tr { page-break-inside: avoid; }
  th, td { border: 1px solid var(--borde); padding: 0.28em 0.4em; text-align: center; }
  th { background: var(--panel-suave); font-weight: bold; font-size: 7.5pt; }
  td.izquierda, th.izquierda { text-align: left; }
  tbody tr:nth-child(even) { background: var(--zebra); }
</style>
</head>
<body>
  <header>
    <div>
      <h1>Historial de Movimientos</h1>
      <p class="subtitulo">${escaparHtml(opciones.filtro)}</p>
    </div>
    <div class="meta">
      Generado por: <strong>${escaparHtml(opciones.generadoPor)}</strong><br />
      ${escaparHtml(generadoEn)}
    </div>
  </header>
  <table>
    <thead><tr>${encabezados}</tr></thead>
    <tbody>${filasHtml}</tbody>
  </table>
</body>
</html>`;
}

export default function Historial() {
  const { sesion } = useAuth();
  const [filas, setFilas] = useState<MovimientoHistorial[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  // Mismo default que desktop/src/pantallas/Historial.tsx -- últimos 6
  // meses, `hasta` abierto para no perderse movimientos del día en curso.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const tablaRef = useRef<TablaHandle<MovimientoHistorial>>(null);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarHistorial(desde || undefined, hasta || undefined)
      .then(setFilas)
      .catch((error) => {
        if (!silencioso) toast.error(String(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, [desde, hasta]);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Ver `useAutoRefresh` -- sin esto, un ingreso ya cerrado/sincronizado
  // no aparecía hasta apretar "actualizar" a mano.
  useAutoRefresh(() => recargar({ silencioso: true }), 30_000, "ingresos");

  /** Columnas visibles AHORA en la grilla (selector "Columnas ▾" + el orden
   * en que la persona las dejó), mapeadas a su definición de export -- si
   * `columnasVisibles()` no está disponible todavía (ref sin montar) cae a
   * todas, en el orden por defecto. Mismo criterio que
   * `seleccionParaExportar` en la versión de escritorio: exportar debe
   * reflejar lo que la persona ve en pantalla, no siempre todas las
   * columnas sin importar qué ocultó. */
  function definicionesVisibles(): DefinicionColumnaExport[] {
    const colIds = tablaRef.current?.columnasVisibles() ?? DEFINICIONES_EXPORT.map((d) => d.colId);
    return colIds
      .map((colId) => DEFINICIONES_EXPORT.find((d) => d.colId === colId))
      .filter((d): d is DefinicionColumnaExport => d !== undefined);
  }

  function filasParaExportar(): MovimientoHistorial[] | null {
    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    if (visibles.length === 0) {
      toast.error("No hay filas para exportar con el filtro actual.");
      return null;
    }
    return visibles;
  }

  async function exportarAExcel() {
    const visibles = filasParaExportar();
    if (!visibles) return;
    const definiciones = definicionesVisibles();
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }

    try {
      const XLSX = await import("xlsx");
      const encabezados = definiciones.map((d) => d.etiqueta);
      const filasHoja = visibles.map((fila) => definiciones.map((d) => d.valor(fila)));
      const hoja = XLSX.utils.aoa_to_sheet([encabezados, ...filasHoja]);
      const libro = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(libro, hoja, "Historial");
      XLSX.writeFile(libro, "historial.xlsx");
    } catch (error) {
      toast.error(`No se pudo exportar a Excel: ${String(error)}`);
    }
  }

  /** Mismo mecanismo que la exportación de escritorio (WebView2
   * `PrintToPdf` sobre un documento HTML armado a mano, ver
   * `desktop/src-tauri/src/pdf/html.rs`) -- acá el motor de impresión es el
   * propio navegador (Chromium/Edge/Safari todos soportan "Guardar como
   * PDF" en el diálogo de impresión) en vez de WebView2 embebido, así que
   * no hace falta ninguna librería nueva. Un iframe oculto aísla el HTML
   * de impresión del resto de la página -- `window.print()` imprime la
   * ventana completa si no se aísla, arrastrando el sidebar/la grilla.
   */
  function exportarAPdf() {
    const visibles = filasParaExportar();
    if (!visibles) return;
    const definiciones = definicionesVisibles();
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }

    const html = generarHtmlHistorial(visibles, definiciones, {
      generadoPor: sesion?.nombre ?? sesion?.correo ?? "",
      filtro: `Filtro: ${textoRangoFecha(desde, hasta)}`,
    });

    const iframe = document.createElement("iframe");
    iframe.style.position = "fixed";
    iframe.style.top = "-10000px";
    iframe.style.left = "-10000px";
    document.body.appendChild(iframe);

    const documento = iframe.contentWindow?.document;
    if (!documento) {
      document.body.removeChild(iframe);
      toast.error("No se pudo preparar el PDF.");
      return;
    }
    documento.open();
    documento.write(html);
    documento.close();

    iframe.contentWindow?.addEventListener("afterprint", () => {
      document.body.removeChild(iframe);
    });
    // Esperar a que el iframe termine de pintar el HTML recién escrito --
    // sin esto, `print()` puede dispararse sobre un documento todavía en
    // blanco en algunos navegadores.
    setTimeout(() => iframe.contentWindow?.print(), 150);
  }

  // useMemo -- mismo motivo que en desktop/: si `columnas` se recrea en
  // cada render, AG Grid reaplica el orden/ancho literales encima del
  // layout que el usuario ya acomodó (persistido en localStorage vía
  // `Tabla`).
  const columnas: ColDef<MovimientoHistorial>[] = useMemo(
    () => [
      { field: "sitio_nombre", headerName: "Sitio", flex: 1, minWidth: 110 },
      {
        field: "contratista_cedula",
        headerName: "Cédula",
        flex: 1.1,
        minWidth: 110,
        cellStyle: { textAlign: "left" },
      },
      {
        field: "contratista_nombre",
        headerName: "Nombre",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1, minWidth: 130 },
      {
        field: "dispositivo_entrada_tipo",
        headerName: "Dispositivo",
        flex: 1,
        minWidth: 110,
        valueFormatter: (p) => textoDispositivo(p.value ?? null),
      },
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
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.hora_entrada) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoHora(p.data.hora_entrada) : ""),
      },
      {
        colId: "fecha_salida",
        headerName: "Fecha salida",
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data?.hora_salida ? fechaLocalYMD(p.data.hora_salida) : "Activo"),
        valueFormatter: (p) => (p.value === "Activo" ? "Activo" : textoFechaDDMMYYYY(p.value)),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 1.3,
        minWidth: 130,
        valueGetter: (p) => (p.data?.hora_salida ? textoHora(p.data.hora_salida) : "Activo"),
      },
      { field: "usuario_entrada_nombre", headerName: "Dio ingreso", flex: 1.3, minWidth: 130 },
      { field: "usuario_salida_nombre", headerName: "Dio salida", flex: 1.3, minWidth: 130 },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<MovimientoHistorial>
            ref={tablaRef}
            id="historial"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            busqueda={busqueda}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder="Cédula, nombre, empresa…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
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
                  title="Exportar a Excel — respeta el filtro/orden/columnas actuales de la grilla"
                  onClick={exportarAExcel}
                  disabled={cargando}
                >
                  <FileSpreadsheet size={16} />
                </button>
                <button
                  type="button"
                  className="boton boton-icono"
                  title="Exportar a PDF — respeta el filtro/orden/columnas actuales de la grilla"
                  onClick={exportarAPdf}
                  disabled={cargando}
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
