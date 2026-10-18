import { useCallback, useMemo, useRef, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { listarGafetesProvisionalesHistorialSitio } from "../api/gafetesProvisionales";
import type { PrestamoGafeteProvisionalHistorialSitio } from "../api/gafetesProvisionales";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import type { PropsHistorialClase } from "./Historial.logica";

/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

/** Historial de gafetes provisionales KOF: lee
 * `prestamos_gafete_provisional_historial_sitio`, una caché que sólo
 * sincroniza la PC. */
export default function HistorialKof({ selector, desde, hasta, onRango }: PropsHistorialClase) {
  const [filas, setFilas] = useState<PrestamoGafeteProvisionalHistorialSitio[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const tablaRef = useRef<TablaHandle<PrestamoGafeteProvisionalHistorialSitio>>(null);

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} movimiento(s)`);

  const recargar = useCallback(
    async (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      try {
        const items = await listarGafetesProvisionalesHistorialSitio(desde || undefined, hasta || undefined);
        if (estaVigente()) setFilas(items);
      } finally {
        if (estaVigente()) setCargando(false);
      }
    },
    [desde, hasta],
  );
  useCargaAlCambiar(recargar, true);

  const columnas: ColDef<PrestamoGafeteProvisionalHistorialSitio>[] = useMemo(
    () => [
      {
        field: "encargado_nombre",
        headerName: "Encargado",
        flex: 1.4,
        minWidth: 160,
        cellStyle: { textAlign: "left" },
      },
      {
        field: "encargado_codigo_empleado",
        headerName: "Código empleado",
        flex: 0.9,
        minWidth: 120,
      },
      { field: "gafete_numero", type: "numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
      {
        colId: "fecha_entrega",
        type: "fecha",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_entrega) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrega",
        headerName: "Entrega",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrega) : ""),
      },
      {
        colId: "hora_devolucion",
        headerName: "Devolución",
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) =>
          p.data?.fecha_hora_devolucion ? textoHora(p.data.fecha_hora_devolucion) : "",
        valueFormatter: (p) => p.value || "—",
      },
      {
        field: "usuario_entrega_nombre",
        headerName: "Entregó",
        flex: 1.1,
        minWidth: 130,
      },
      {
        field: "usuario_devolucion_nombre",
        headerName: "Recibió",
        flex: 1.1,
        minWidth: 130,
        valueFormatter: (p) => p.value ?? "—",
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<PrestamoGafeteProvisionalHistorialSitio>
            ref={tablaRef}
            cargando={cargando}
            filtrosPorColumna
            id="gafetes-provisionales-historial"
            idFila={idPorUuid}
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder="Nombre o código de empleado…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            accionesCentro={selector}
            accionesDerecha={
              <>
                <SelectorRangoFecha desde={desde} hasta={hasta} onAplicar={onRango} />
                <BotonesExportacion
                  tablaRef={tablaRef}
                  nombreArchivo="historial-kof"
                  titulo="Historial de Gafetes Provisionales KOF"
                  filtroDescripcion={`Filtro: ${textoRangoFecha(desde, hasta)}`}
                />
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
