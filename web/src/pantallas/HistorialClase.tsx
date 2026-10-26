import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { isCancelledError, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { FileSpreadsheet, FileText, Sheet } from "lucide-react";
import Tabla from "../componentes/Tabla";
import type { OrigenServidor, TablaHandle } from "../componentes/Tabla";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { useDebounced } from "../componentes/useDebounced";
import { useAuth } from "../contexto/useAuth";
import { listarClaseParaExportar, listarPaginaClase } from "../api/historialClases";
import type { ClaseOtra, ConsultaClase, MovimientoClase } from "../api/historialClases";
import { mensajeError } from "../mensajeError";
import { definicionesVisibles, descargarArchivo, descargarExcel, generarCsv, generarHtmlTabla, imprimirHtml } from "../exportacion";
import { ETIQUETAS_CLASE, columnasClase, definicionesExportClase } from "./HistorialClase.logica";
import type { FiltroHistorial } from "./Historial.logica";

/** Filas por página, como en contratistas. */
const TAMANO_PAGINA = 100;

/**
 * Historial de una clase sin catálogo propio en el panel: proveedores,
 * visitas por correo o gafetes KOF. Lo mismo que contratistas
 * (`HistorialContratistas.tsx`): el servidor pagina, ordena y aplica los
 * filtros por columna; el buscador busca por cédula (o código de empleado) y
 * por nombre; Excel, CSV y PDF traen todo el resultado del filtro con las
 * columnas visibles. Las unidades y el rango de fechas los comparte con las
 * otras clases (`Historial.tsx`).
 */
export default function HistorialClase({
  clase,
  selector,
  compartidos,
  filtro,
  textoFiltro,
}: {
  clase: ClaseOtra;
  /** Selector de clase (va junto al buscador). */
  selector: ReactNode;
  /** Unidades operativas y rango de fechas, comunes a todas las clases. */
  compartidos: ReactNode;
  filtro: FiltroHistorial;
  /** "Filtro: (rango) — (unidades)", para el encabezado del PDF. */
  textoFiltro: string;
}) {
  const { sesion } = useAuth();
  const clienteConsultas = useQueryClient();
  const [busqueda, setBusqueda] = useState("");
  const [exportando, setExportando] = useState(false);
  const tablaRef = useRef<TablaHandle<MovimientoClase>>(null);
  const etiquetas = ETIQUETAS_CLASE[clase];
  const columnas = useMemo(() => columnasClase(clase), [clase]);
  const definiciones = useMemo(() => definicionesExportClase(clase), [clase]);

  // La búsqueda va al servidor: se espera a que la persona deje de escribir.
  const busquedaDebounced = useDebounced(busqueda, 300);
  const consultaPantalla = useMemo(() => ({ ...filtro, busqueda: busquedaDebounced }), [filtro, busquedaDebounced]);
  const consultaRef = useRef(consultaPantalla);
  useEffect(() => {
    // Primer render: la grilla ya pidió su primera página con esta consulta.
    if (consultaRef.current === consultaPantalla) return;
    consultaRef.current = consultaPantalla;
    tablaRef.current?.refrescar(true);
  }, [consultaPantalla]);

  const origenServidor: OrigenServidor<MovimientoClase> = {
    tamanoPagina: TAMANO_PAGINA,
    cargarPagina: ({ inicio, cantidad, orden, filtros }) => {
      const consulta: ConsultaClase = {
        ...consultaRef.current,
        filtros,
        orden: orden ? { colId: orden.colId, descendente: orden.descendente } : undefined,
      };
      const pagina = Math.floor(inicio / cantidad);
      return clienteConsultas.fetchQuery({
        queryKey: ["historial", clase, consulta, pagina, cantidad],
        queryFn: () => listarPaginaClase(clase, consulta, pagina, cantidad),
        staleTime: 10_000,
      });
    },
    // Ver `HistorialContratistas`: el refresco automático cancela la página en
    // curso y la vuelve a pedir; no es un fallo.
    alFallar: (error) => {
      if (isCancelledError(error)) return;
      toast.error(mensajeError(error));
    },
  };

  // Estas tablas no están en la publicación de Realtime: se refrescan cada 30 s,
  // sólo la página visible.
  useAutoRefresh(() => {
    clienteConsultas.removeQueries({ queryKey: ["historial", clase] });
    tablaRef.current?.refrescar();
  }, 30_000);

  /** Todas las filas del filtro, orden y filtros por columna actuales (no sólo
   * la página en pantalla), con las columnas visibles. */
  async function preparar() {
    const visibles = definicionesVisibles(definiciones, tablaRef.current?.columnasVisibles());
    if (visibles.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return null;
    }
    setExportando(true);
    try {
      const orden = tablaRef.current?.orden();
      const { filas, truncado } = await listarClaseParaExportar(clase, {
        ...consultaPantalla,
        filtros: tablaRef.current?.modeloFiltros(),
        orden: orden ? { colId: orden.colId, descendente: orden.descendente } : undefined,
      });
      if (filas.length === 0) {
        toast.error("No hay filas para exportar con el filtro actual.");
        return null;
      }
      if (truncado) {
        toast.warning(
          `Se exportan las primeras ${filas.length.toLocaleString("es-CR")} filas; el filtro tiene más. Acote las fechas o la unidad para exportar el resto.`,
        );
      }
      return { filas, visibles };
    } catch (error) {
      toast.error(mensajeError(error));
      return null;
    } finally {
      setExportando(false);
    }
  }

  async function exportarAExcel() {
    const datos = await preparar();
    if (!datos) return;
    try {
      await descargarExcel(datos.filas, datos.visibles, { hoja: "Historial", archivo: `${etiquetas.archivo}.xlsx` });
    } catch (error) {
      toast.error(`No se pudo exportar a Excel: ${mensajeError(error)}`);
    }
  }

  async function exportarACsv() {
    const datos = await preparar();
    if (!datos) return;
    descargarArchivo(`${etiquetas.archivo}.csv`, generarCsv(datos.filas, datos.visibles), "text/csv;charset=utf-8");
  }

  async function exportarAPdf() {
    const datos = await preparar();
    if (!datos) return;
    const html = generarHtmlTabla(datos.filas, datos.visibles, {
      titulo: etiquetas.titulo,
      generadoPor: sesion?.nombre ?? sesion?.correo ?? "",
      filtro: textoFiltro,
    });
    if (!imprimirHtml(html)) toast.error("No se pudo preparar el PDF.");
  }

  const tituloExportar = (formato: string) =>
    exportando ? "Exportando…" : `Exportar a ${formato} — todo el resultado del filtro actual, con las columnas visibles`;

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          <Tabla<MovimientoClase>
            ref={tablaRef}
            id={`historial-${clase}`}
            idFila={idPorMovimiento}
            columnas={columnas}
            filtrosPorColumna
            origenServidor={origenServidor}
            controles={
              <>
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder={`${etiquetas.identificacion} o nombre…`}
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
                {selector}
              </>
            }
            accionesDerecha={
              <>
                {compartidos}
                <div className="segmentado" role="group" aria-label="Exportar">
                  <button
                    type="button"
                    title={tituloExportar("Excel")}
                    aria-label="Exportar a Excel"
                    onClick={exportarAExcel}
                    disabled={exportando}
                  >
                    <FileSpreadsheet size={16} />
                  </button>
                  <button
                    type="button"
                    title={tituloExportar("CSV")}
                    aria-label="Exportar a CSV"
                    onClick={exportarACsv}
                    disabled={exportando}
                  >
                    <Sheet size={16} />
                  </button>
                  <button
                    type="button"
                    title={tituloExportar("PDF")}
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
function idPorMovimiento(fila: MovimientoClase): string {
  return fila.id;
}
