import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { FileSpreadsheet, FileText, Sheet } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { OrdenTabla, OrigenServidor, TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import SelectorUnidadesOperativas, {
  textoUnidadesOperativas,
} from "../componentes/SelectorUnidadesOperativas";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { useDebounced } from "../componentes/useDebounced";
import { useAuth } from "../contexto/AuthContexto";
import {
  CAMPOS_ORDENABLES,
  listarMovimientosPagina,
  listarMovimientosParaExportar,
  listarUnidadesOperativas,
} from "../api/historial";
import type { CampoOrdenable, ConsultaMovimientos, MovimientoHistorial } from "../api/historial";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";

/** "pc"/"mobile"/"visor" (`dispositivos.tipo`) → sólo el ícono, para la
 * columna "Dispositivo" -- tanto en pantalla como en Excel/PDF (mismo
 * mapeo que `textoDispositivo` en `desktop/src/pantallas/Historial.tsx` y
 * `HistorialViewModel.kt` del móvil). Antes devolvía ícono + palabra
 * ("💻 PC"/"📱 Celular"), pero quedaba desparejo visualmente -- una palabra
 * bastante más larga que la otra. */
export function textoDispositivo(tipo: string | null): string {
  if (tipo === "pc") return "💻";
  if (tipo === "mobile") return "📱";
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
  { colId: "sitio_nombre", etiqueta: "Unidad operativa", valor: (f) => f.sitio_nombre ?? "" },
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
    valor: (f) => textoDispositivo(f.dispositivo_entrada_tipo),
  },
  { colId: "tipo_ingreso", etiqueta: "Tipo", valor: (f) => f.tipo_texto },
  { colId: "medio_ingreso", etiqueta: "Medio", valor: (f) => f.medio_texto },
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

/** Filas por página. */
const TAMANO_PAGINA = 100;

/** Campo del servidor por el que se ordena al tocar el encabezado de una
 * columna. Las columnas de fecha y hora comparten el mismo instante, y las de
 * tipo y medio ordenan por el texto que se ve ("IN HOUSE", la placa). */
const CAMPO_ORDEN_POR_COLUMNA: Record<string, CampoOrdenable> = {
  fecha_ingreso: "hora_entrada",
  hora_ingreso: "hora_entrada",
  fecha_salida: "hora_salida",
  hora_salida: "hora_salida",
  tipo_ingreso: "tipo_texto",
  medio_ingreso: "medio_texto",
};

export function campoOrdenDeColumna(colId: string): CampoOrdenable | null {
  const mapeado = CAMPO_ORDEN_POR_COLUMNA[colId];
  if (mapeado) return mapeado;
  return (CAMPOS_ORDENABLES as readonly string[]).includes(colId) ? (colId as CampoOrdenable) : null;
}

function ordenDeConsulta(orden: OrdenTabla | null | undefined): ConsultaMovimientos["orden"] {
  const campo = orden ? campoOrdenDeColumna(orden.colId) : null;
  return orden && campo ? { campo, descendente: orden.descendente } : undefined;
}

/** CSV como el de escritorio: separador `;`, comillas si el texto lleva `;`,
 * comillas o saltos de línea. Empieza con BOM para que Excel lea las tildes. */
export function generarCsvHistorial(
  filas: MovimientoHistorial[],
  columnas: DefinicionColumnaExport[],
): string {
  const celda = (texto: string) => (/[;"\r\n]/.test(texto) ? `"${texto.replace(/"/g, '""')}"` : texto);
  const lineas = [
    columnas.map((c) => celda(c.etiqueta)).join(";"),
    ...filas.map((fila) => columnas.map((c) => celda(c.valor(fila))).join(";")),
  ];
  return "\uFEFF" + lineas.join("\r\n") + "\r\n";
}

function descargarArchivo(nombre: string, contenido: BlobPart, tipo: string) {
  const url = URL.createObjectURL(new Blob([contenido], { type: tipo }));
  const enlace = document.createElement("a");
  enlace.href = url;
  enlace.download = nombre;
  enlace.click();
  URL.revokeObjectURL(url);
}

export default function Historial() {
  const { sesion } = useAuth();
  const clienteConsultas = useQueryClient();
  const [busqueda, setBusqueda] = useState("");
  // Mismo default que desktop/src/pantallas/Historial.tsx -- últimos 6
  // meses, `hasta` abierto para no perderse movimientos del día en curso.
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  // Sigue el mismo criterio que `ocultas` en `Tabla.tsx`: guarda lo
  // DESMARCADO, no lo marcado -- así una unidad que todavía no cargó (o una
  // nueva que se dio de alta después) nunca queda afuera del filtro por
  // accidente. Vacío = sin filtro (todas), ver `sitioIdsFiltro` más abajo.
  const [unidadesExcluidas, setUnidadesExcluidas] = useState<Set<string>>(new Set());
  const [exportando, setExportando] = useState(false);
  const tablaRef = useRef<TablaHandle<MovimientoHistorial>>(null);

  const { data: unidades = [], error: errorUnidades } = useQuery({
    queryKey: ["historial", "unidades"],
    queryFn: listarUnidadesOperativas,
    staleTime: 5 * 60_000,
  });
  useEffect(() => {
    if (errorUnidades) toast.error(mensajeError(errorUnidades));
  }, [errorUnidades]);

  // `undefined` (sin filtro) cuando nada está excluido -- a propósito
  // distinto de "mandar la lista completa de ids": así el primer render (con
  // `unidades` todavía vacío, antes de que responda `listarUnidadesOperativas`)
  // no pide un `.in(sitio_id, [])` que traería cero filas por una carrera de
  // carga, no porque alguien haya filtrado nada.
  const sitioIdsFiltro =
    unidadesExcluidas.size === 0
      ? undefined
      : unidades.filter((u) => !unidadesExcluidas.has(u.id)).map((u) => u.id);

  // La búsqueda va al servidor: se espera a que la persona deje de escribir
  // para no pedir una consulta por tecla.
  const busquedaDebounced = useDebounced(busqueda, 300);

  // Lo que decide la pantalla (rango, unidades, búsqueda). Los filtros por
  // columna y el orden los lleva la grilla y se le piden al pedir cada página.
  const filtroPantalla = useMemo(
    () => ({
      desde: desde || undefined,
      hasta: hasta || undefined,
      sitioIds: sitioIdsFiltro,
      busqueda: busquedaDebounced,
    }),
    // `sitioIdsFiltro` se recalcula en cada render; sus dependencias reales
    // son las dos de abajo.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [desde, hasta, unidadesExcluidas, unidades, busquedaDebounced],
  );
  const filtroPantallaRef = useRef(filtroPantalla);
  useEffect(() => {
    // Primer render (o repetición de StrictMode): la grilla ya pidió su primera
    // página con este mismo filtro, no hace falta pedirla otra vez.
    if (filtroPantallaRef.current === filtroPantalla) return;
    filtroPantallaRef.current = filtroPantalla;
    // Cambió el filtro de pantalla: la grilla vuelve a pedir desde la página 1.
    tablaRef.current?.refrescar(true);
  }, [filtroPantalla]);

  const origenServidor: OrigenServidor<MovimientoHistorial> = {
    tamanoPagina: TAMANO_PAGINA,
    cargarPagina: ({ inicio, cantidad, orden, filtros }) => {
      const consulta: ConsultaMovimientos = {
        ...filtroPantallaRef.current,
        filtros,
        orden: ordenDeConsulta(orden),
        pagina: Math.floor(inicio / cantidad),
        tamano: cantidad,
      };
      return clienteConsultas.fetchQuery({
        queryKey: ["historial", "movimientos", consulta],
        queryFn: () => listarMovimientosPagina(consulta),
        staleTime: 10_000,
      });
    },
    alFallar: (error) => toast.error(mensajeError(error)),
  };

  // Ver `useAutoRefresh` -- sin esto, un ingreso ya cerrado/sincronizado no
  // aparecía hasta apretar "actualizar" a mano. Realtime sólo vuelve a pedir la
  // página visible; no baja todo el historial.
  useAutoRefresh(
    () => {
      clienteConsultas.removeQueries({ queryKey: ["historial", "movimientos"] });
      tablaRef.current?.refrescar();
    },
    30_000,
    "ingresos",
  );

  /** Columnas visibles AHORA en la grilla (el selector de columnas + el orden
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

  /** Trae del servidor TODAS las filas del rango, unidades, búsqueda, filtros
   * por columna y orden actuales (no sólo la página en pantalla), hasta el
   * máximo de una exportación. */
  async function filasParaExportar(): Promise<MovimientoHistorial[] | null> {
    setExportando(true);
    try {
      const { filas: todas, truncado } = await listarMovimientosParaExportar({
        ...filtroPantalla,
        filtros: tablaRef.current?.modeloFiltros(),
        orden: ordenDeConsulta(tablaRef.current?.orden()),
      });
      if (todas.length === 0) {
        toast.error("No hay filas para exportar con el filtro actual.");
        return null;
      }
      if (truncado) {
        toast.warning(
          `Se exportan las primeras ${todas.length.toLocaleString("es-CR")} filas; el filtro tiene más. Acote las fechas o la unidad para exportar el resto.`,
        );
      }
      return todas;
    } catch (error) {
      toast.error(mensajeError(error));
      return null;
    } finally {
      setExportando(false);
    }
  }

  async function exportarAExcel() {
    const filas = await filasParaExportar();
    if (!filas) return;
    const definiciones = definicionesVisibles();
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }

    try {
      const XLSX = await import("xlsx");
      const encabezados = definiciones.map((d) => d.etiqueta);
      const filasHoja = filas.map((fila) => definiciones.map((d) => d.valor(fila)));
      const hoja = XLSX.utils.aoa_to_sheet([encabezados, ...filasHoja]);
      const libro = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(libro, hoja, "Historial");
      XLSX.writeFile(libro, "historial.xlsx");
    } catch (error) {
      toast.error(`No se pudo exportar a Excel: ${mensajeError(error)}`);
    }
  }

  async function exportarACsv() {
    const filas = await filasParaExportar();
    if (!filas) return;
    const definiciones = definicionesVisibles();
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }
    descargarArchivo("historial.csv", generarCsvHistorial(filas, definiciones), "text/csv;charset=utf-8");
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
  async function exportarAPdf() {
    const filas = await filasParaExportar();
    if (!filas) return;
    const definiciones = definicionesVisibles();
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return;
    }

    const filtroUnidades = textoUnidadesOperativas(unidades.length, unidadesExcluidas.size);
    const html = generarHtmlHistorial(filas, definiciones, {
      generadoPor: sesion?.nombre ?? sesion?.correo ?? "",
      filtro: `Filtro: ${textoRangoFecha(desde, hasta)} — ${filtroUnidades}`,
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
  // `Tabla`). Los textos de Tipo y Medio vienen ya armados de la vista
  // (`tipo_texto`, `medio_texto`), así el filtro y el orden usan lo que se ve.
  const columnas: ColDef<MovimientoHistorial>[] = useMemo(
    () => [
      { field: "sitio_nombre", headerName: "Unidad operativa", flex: 1.3, minWidth: 160 },
      {
        field: "contratista_cedula",
        headerName: "Cédula",
        flex: 1.2,
        minWidth: 120,
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
      {
        field: "tipo_ingreso",
        headerName: "Tipo",
        flex: 1,
        minWidth: 100,
        valueGetter: (p) => p.data?.tipo_texto ?? "",
      },
      {
        field: "medio_ingreso",
        headerName: "Medio",
        flex: 1,
        minWidth: 100,
        valueGetter: (p) => p.data?.medio_texto ?? "",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_ingreso",
        type: "fecha",
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
        // Sin filtro propio: un filtro de texto sobre "HH:MI" era engañoso
        // ("7" también trae 17:xx y 10:07). Para acotar por tiempo se usa
        // "Fecha ingreso", que filtra el instante completo con índice.
        filter: false,
        valueGetter: (p) => (p.data ? textoHora(p.data.hora_entrada) : ""),
      },
      {
        colId: "fecha_salida",
        type: "fecha",
        headerName: "Fecha salida",
        flex: 1.4,
        minWidth: 140,
        valueGetter: (p) => (p.data ? (p.data.hora_salida ? fechaLocalYMD(p.data.hora_salida) : "Activo") : ""),
        valueFormatter: (p) => (p.value === "Activo" || !p.value ? (p.value ?? "") : textoFechaDDMMYYYY(p.value)),
      },
      {
        colId: "hora_salida",
        headerName: "Hora salida",
        flex: 1.3,
        minWidth: 130,
        // Ver "Hora ingreso": se filtra con "Fecha salida" (vacío = sigue adentro).
        filter: false,
        valueGetter: (p) => (p.data ? (p.data.hora_salida ? textoHora(p.data.hora_salida) : "Activo") : ""),
      },
      { field: "usuario_entrada_nombre", headerName: "Dio ingreso", flex: 1.3, minWidth: 130 },
      { field: "usuario_salida_nombre", headerName: "Dio salida", flex: 1.3, minWidth: 130 },
    ],
    [],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          <Tabla<MovimientoHistorial>
            ref={tablaRef}
            id="historial"
            idFila={idPorMovimiento}
            columnas={columnas}
            filtrosPorColumna
            origenServidor={origenServidor}
            controles={
              <div className="campo flex-[0_1_16rem]">
                <input
                  placeholder="Cédula o nombre…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            accionesDerecha={
              <>
                <SelectorUnidadesOperativas
                  unidades={unidades}
                  excluidas={unidadesExcluidas}
                  onCambiar={setUnidadesExcluidas}
                />
                <SelectorRangoFecha
                  desde={desde}
                  hasta={hasta}
                  onAplicar={(nuevoDesde, nuevoHasta) => {
                    setDesde(nuevoDesde);
                    setHasta(nuevoHasta);
                  }}
                />
                {/* Excel · CSV · PDF en una sola pieza de íconos, como en
                    escritorio. Los tres traen TODO el resultado del filtro
                    actual (no sólo la página en pantalla), con las columnas
                    visibles y el orden vigente. */}
                <div className="segmentado" role="group" aria-label="Exportar">
                  <button
                    type="button"
                    title={
                      exportando
                        ? "Exportando…"
                        : "Exportar a Excel — todo el resultado del filtro actual, con las columnas visibles"
                    }
                    aria-label="Exportar a Excel"
                    onClick={exportarAExcel}
                    disabled={exportando}
                  >
                    <FileSpreadsheet size={16} />
                  </button>
                  <button
                    type="button"
                    title={
                      exportando
                        ? "Exportando…"
                        : "Exportar a CSV — todo el resultado del filtro actual, con las columnas visibles"
                    }
                    aria-label="Exportar a CSV"
                    onClick={exportarACsv}
                    disabled={exportando}
                  >
                    <Sheet size={16} />
                  </button>
                  <button
                    type="button"
                    title={
                      exportando
                        ? "Exportando…"
                        : "Exportar a PDF — todo el resultado del filtro actual, con las columnas visibles"
                    }
                    aria-label="Exportar a PDF"
                    onClick={exportarAPdf}
                    disabled={exportando}
                  >
                    <FileText size={16} />
                  </button>
                </div>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}

// Fuera del componente: `idFila` tiene que ser una función estable.
function idPorMovimiento(fila: MovimientoHistorial): string {
  return fila.id;
}
