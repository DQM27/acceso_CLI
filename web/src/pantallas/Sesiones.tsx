import { useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { FileSpreadsheet, FileText, Sheet } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import AvisoTruncado from "../componentes/AvisoTruncado";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import { useLista } from "../componentes/useLista";
import { useAuth } from "../contexto/AuthContexto";
import { MAXIMO_SESIONES, listarSesiones, textoDuracion, textoEstadoSesion } from "../api/sesiones";
import type { SesionBitacora } from "../api/sesiones";
import {
  definicionesVisibles,
  descargarArchivo,
  descargarExcel,
  generarCsv,
  generarHtmlTabla,
  imprimirHtml,
} from "../exportacion";
import type { DefinicionColumnaExport } from "../exportacion";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";

function fechaHora(iso: string | null): string {
  return iso ? `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}` : "";
}

/** "Hace 30 días" en `AAAA-MM-DD` (día de Costa Rica). */
function hace30Dias(): string {
  return fechaLocalYMD(new Date(Date.now() - 30 * 86_400_000).toISOString());
}

const TIPO_EQUIPO: Record<string, string> = { pc: "PC", mobile: "Celular", visor: "Visor" };

function textoEquipo(sesion: SesionBitacora): string {
  const tipo = sesion.dispositivo_tipo ? (TIPO_EQUIPO[sesion.dispositivo_tipo] ?? sesion.dispositivo_tipo) : "";
  const etiqueta = sesion.dispositivo_etiqueta ?? "Equipo eliminado";
  return tipo ? `${etiqueta} (${tipo})` : etiqueta;
}

/** Una sola definición por columna para la grilla y las tres exportaciones. */
export const COLUMNAS_SESIONES: DefinicionColumnaExport<SesionBitacora>[] = [
  { colId: "inicio", etiqueta: "Inicio", valor: (s) => fechaHora(s.iniciada_en) },
  { colId: "cierre", etiqueta: "Cierre", valor: (s) => (s.abierta ? "—" : fechaHora(s.cerrada_en)) },
  {
    colId: "duracion",
    etiqueta: "Duración",
    valor: (s) => textoDuracion(s.iniciada_en, s.cerrada_en ?? s.ultima_actividad),
  },
  { colId: "nombre", etiqueta: "Usuario", izquierda: true, valor: (s) => s.nombre },
  { colId: "cedula", etiqueta: "Cédula", izquierda: true, valor: (s) => s.cedula },
  { colId: "sitio_nombre", etiqueta: "Unidad", valor: (s) => s.sitio_nombre ?? "—" },
  { colId: "equipo", etiqueta: "Equipo", izquierda: true, valor: textoEquipo },
  { colId: "estado", etiqueta: "Estado", valor: textoEstadoSesion },
  { colId: "ultima_actividad", etiqueta: "Última actividad", valor: (s) => fechaHora(s.ultima_actividad) },
];

/** Columnas de fecha que ordenan por el instante, no por el texto. */
const ORDEN_POR_INSTANTE: Partial<Record<string, (s: SesionBitacora) => string>> = {
  inicio: (s) => s.iniciada_en,
  cierre: (s) => s.cerrada_en ?? "",
  ultima_actividad: (s) => s.ultima_actividad,
};

/**
 * Bitácora de sesiones: quién inició sesión, en qué unidad y equipo, cuándo
 * y cómo terminó. Pedido del usuario 2026-09-30, junto con la sesión única
 * por unidad (ver `api/sesiones.ts`).
 */
export default function Sesiones() {
  const { sesion } = useAuth();
  const [desde, setDesde] = useState(hace30Dias);
  const [hasta, setHasta] = useState("");
  const [busqueda, setBusqueda] = useState("");
  const tablaRef = useRef<TablaHandle<SesionBitacora>>(null);

  // La bitácora no está en la publicación de Realtime: se refresca cada minuto.
  const { datos, cargando } = useLista(["sesiones", desde, hasta], () => listarSesiones({ desde, hasta }), {
    intervaloMs: 60_000,
    tablas: "",
  });
  const filas = useMemo(() => datos?.filas ?? [], [datos]);

  const columnas = useMemo<ColDef<SesionBitacora>[]>(
    () =>
      COLUMNAS_SESIONES.map((definicion) => {
        const instante = ORDEN_POR_INSTANTE[definicion.colId];
        const columna: ColDef<SesionBitacora> = {
          colId: definicion.colId,
          headerName: definicion.etiqueta,
          flex: 1,
          minWidth: definicion.colId === "nombre" || definicion.colId === "equipo" ? 170 : 120,
          valueGetter: (p) => (p.data ? definicion.valor(p.data) : ""),
        };
        if (instante) {
          columna.comparator = (_a, _b, nodoA, nodoB) =>
            (nodoA.data ? instante(nodoA.data) : "").localeCompare(nodoB.data ? instante(nodoB.data) : "");
        }
        if (definicion.colId === "estado") {
          columna.cellStyle = (p) => (p.data?.abierta ? { color: "var(--exito)", fontWeight: 600 } : null);
        }
        return columna;
      }),
    [],
  );

  function aExportar(): { filas: SesionBitacora[]; columnas: DefinicionColumnaExport<SesionBitacora>[] } | null {
    const visibles = tablaRef.current?.filasFiltradas() ?? filas;
    const definiciones = definicionesVisibles(COLUMNAS_SESIONES, tablaRef.current?.columnasVisibles());
    if (visibles.length === 0) {
      toast.error("No hay sesiones para exportar con el filtro actual.");
      return null;
    }
    if (definiciones.length === 0) {
      toast.error("No hay columnas visibles para exportar.");
      return null;
    }
    return { filas: visibles, columnas: definiciones };
  }

  async function exportarAExcel() {
    const datosExportar = aExportar();
    if (!datosExportar) return;
    try {
      await descargarExcel(datosExportar.filas, datosExportar.columnas, { hoja: "Sesiones", archivo: "sesiones.xlsx" });
    } catch (error) {
      toast.error(`No se pudo exportar a Excel: ${mensajeError(error)}`);
    }
  }

  function exportarACsv() {
    const datosExportar = aExportar();
    if (!datosExportar) return;
    descargarArchivo("sesiones.csv", generarCsv(datosExportar.filas, datosExportar.columnas), "text/csv;charset=utf-8");
  }

  function exportarAPdf() {
    const datosExportar = aExportar();
    if (!datosExportar) return;
    const html = generarHtmlTabla(datosExportar.filas, datosExportar.columnas, {
      titulo: "Bitácora de sesiones",
      generadoPor: sesion?.nombre ?? sesion?.correo ?? "",
      filtro: `Filtro: ${textoRangoFecha(desde, hasta)}`,
    });
    if (!imprimirHtml(html)) toast.error("No se pudo preparar el PDF.");
  }

  const abiertas = filas.filter((fila) => fila.abierta).length;

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        {datos?.truncado && (
          <AvisoTruncado
            mensaje={`Se muestran las ${MAXIMO_SESIONES.toLocaleString("es-CR")} sesiones más recientes del rango; acote las fechas para ver el resto.`}
          />
        )}
        <div className="min-h-0 flex-1">
          <Tabla<SesionBitacora>
            ref={tablaRef}
            id="sesiones"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            idFila={(fila) => String(fila.id)}
            controles={
              <>
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Usuario, cédula, unidad…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
                <span role="status">
                  {filas.length.toLocaleString("es-CR")} sesiones · {abiertas.toLocaleString("es-CR")} abiertas
                </span>
              </>
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
                <div className="segmentado" role="group" aria-label="Exportar">
                  <button
                    type="button"
                    title="Exportar a Excel — las sesiones filtradas, con las columnas visibles"
                    aria-label="Exportar a Excel"
                    onClick={exportarAExcel}
                  >
                    <FileSpreadsheet size={16} />
                  </button>
                  <button
                    type="button"
                    title="Exportar a CSV — las sesiones filtradas, con las columnas visibles"
                    aria-label="Exportar a CSV"
                    onClick={exportarACsv}
                  >
                    <Sheet size={16} />
                  </button>
                  <button
                    type="button"
                    title="Exportar a PDF — las sesiones filtradas, con las columnas visibles"
                    aria-label="Exportar a PDF"
                    onClick={exportarAPdf}
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
