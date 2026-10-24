import { useEffect, useMemo, useRef, useState } from "react";
import { isCancelledError, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { FileSpreadsheet, FileText, Sheet } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { OrdenTabla, OrigenServidor, TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import SelectorUnidadesOperativas from "../componentes/SelectorUnidadesOperativas";
import { textoUnidadesOperativas } from "../componentes/SelectorUnidadesOperativas.logica";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { useDebounced } from "../componentes/useDebounced";
import { useAuth } from "../contexto/useAuth";
import { listarMovimientosPagina, listarMovimientosParaExportar, listarUnidadesOperativas } from "../api/historial";
import type { ConsultaMovimientos, MovimientoHistorial } from "../api/historial";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";
import {
  definicionesVisibles as columnasAExportar,
  descargarArchivo,
  descargarExcel,
  imprimirHtml,
} from "../exportacion";
import {
  DEFINICIONES_EXPORT,
  campoOrdenDeColumna,
  generarCsvHistorial,
  generarHtmlHistorial,
  textoDispositivo,
  type DefinicionColumnaExport,
} from "./Historial.logica";

/** Filas por página. */
const TAMANO_PAGINA = 100;

function ordenDeConsulta(orden: OrdenTabla | null | undefined): ConsultaMovimientos["orden"] {
  const campo = orden ? campoOrdenDeColumna(orden.colId) : null;
  return orden && campo ? { campo, descendente: orden.descendente } : undefined;
}

/**
 * Historial multi-sitio para `admin_global` -- lee `ingresos` en Supabase
 * (ver `api/historial.ts` y la migración `agrega_columnas_historial_a_ingresos`),
 * no la base local de un sitio en particular como la versión de escritorio.
 * "Exportar a Excel" es client-side (SheetJS) en vez del exportador de
 * Rust/Tauri de `desktop/` -- este panel no tiene un proceso nativo del
 * lado del navegador que escriba el archivo.
 */
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
    // El refresco automático (abajo) hace `removeQueries` y eso cancela la
    // página que justo se estaba pidiendo (CancelledError). No es un fallo:
    // el `refrescar()` que le sigue la vuelve a pedir, así que no se avisa.
    alFallar: (error) => {
      if (isCancelledError(error)) return;
      toast.error(mensajeError(error));
    },
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
    return columnasAExportar(DEFINICIONES_EXPORT, tablaRef.current?.columnasVisibles());
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
      await descargarExcel(filas, definiciones, { hoja: "Historial", archivo: "historial.xlsx" });
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

    if (!imprimirHtml(html)) toast.error("No se pudo preparar el PDF.");
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
