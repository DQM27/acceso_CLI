import { useCallback, useMemo, useRef, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { listarHistorialIngresosProveedorSitio } from "../api/proveedores";
import type { HistorialIngresoProveedorRemoto } from "../api/proveedores";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import type { PropsHistorialClase } from "./Historial.logica";

/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

/** Historial de proveedores: lee `historial_ingresos_proveedor_sitio`, una
 * caché que sólo sincroniza la PC (el celular no la trae a propósito). */
export default function HistorialProveedores({ selector, desde, hasta, onRango }: PropsHistorialClase) {
  const [filas, setFilas] = useState<HistorialIngresoProveedorRemoto[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const tablaRef = useRef<TablaHandle<HistorialIngresoProveedorRemoto>>(null);

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} movimiento(s)`);

  const recargar = useCallback(
    async (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      try {
        const items = await listarHistorialIngresosProveedorSitio(desde || undefined, hasta || undefined);
        if (estaVigente()) setFilas(items);
      } finally {
        if (estaVigente()) setCargando(false);
      }
    },
    [desde, hasta],
  );
  useCargaAlCambiar(recargar, true);

  const columnas: ColDef<HistorialIngresoProveedorRemoto>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 0.9, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      {
        field: "empresa_nombre",
        headerName: "Empresa",
        flex: 1.2,
        minWidth: 140,
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "placa",
        headerName: "Placa",
        flex: 0.8,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "Caminando",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.7,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      {
        colId: "fecha_entrada",
        type: "fecha",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        colId: "hora_entrada",
        headerName: "Entrada",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      {
        colId: "hora_salida",
        headerName: "Salida",
        flex: 0.8,
        minWidth: 85,
        valueGetter: (p) => (p.data?.fecha_hora_salida ? textoHora(p.data.fecha_hora_salida) : ""),
        valueFormatter: (p) => p.value || "—",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "usuario_salida_nombre",
        headerName: "Dio salida",
        flex: 1.1,
        minWidth: 120,
        valueFormatter: (p) => p.value ?? "—",
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<HistorialIngresoProveedorRemoto>
            ref={tablaRef}
            cargando={cargando}
            filtrosPorColumna
            id="proveedores-historial"
            idFila={idPorUuid}
            columnas={columnas}
            filas={filas}
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
            accionesCentro={selector}
            accionesDerecha={
              <>
                <SelectorRangoFecha desde={desde} hasta={hasta} onAplicar={onRango} />
                <BotonesExportacion
                  tablaRef={tablaRef}
                  nombreArchivo="historial-proveedores"
                  titulo="Historial de Proveedores"
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
