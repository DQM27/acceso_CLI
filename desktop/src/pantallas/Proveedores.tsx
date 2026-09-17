import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { cerrarFilaProveedorActiva, listarTodosLosProveedoresActivos } from "../api/proveedores";
import type { FilaProveedorActiva } from "../api/proveedores";
import { textoFechaDDMMYYYY, textoHora } from "../tiempo";

const IngresoProveedorModal = lazy(() => import("./IngresoProveedorModal"));

/**
 * Pantalla operativa de proveedores -- mismo patrón que Rutas.tsx: grilla de
 * "activos" + botón "+ Ingreso" que abre el registro en un modal, y un botón
 * directo "Salida" por fila. `refrescarSenal` (de `Shell`) recarga sola
 * cuando llega cualquier sincronización, igual que Activos/Rutas. La lista
 * fusiona locales + los que otro dispositivo del sitio tiene abiertos ahora
 * (`listarTodosLosProveedoresActivos`), mismo criterio que
 * `listarTodosLosActivos` para contratistas -- un ingreso abierto en otra
 * PC/celular del mismo sitio debe verse y poder cerrarse desde acá también.
 */
export default function Proveedores({ refrescarSenal }: { refrescarSenal?: number }) {
  const [filas, setFilas] = useState<FilaProveedorActiva[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} proveedor(es) activo(s)`);

  const recargar = useCallback(() => {
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarTodosLosProveedoresActivos())
      .then(setFilas)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
  }, [refrescarSenal, recargar]);

  const registrarSalida = useCallback(
    async (fila: FilaProveedorActiva) => {
      try {
        await cerrarFilaProveedorActiva(fila);
        await recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  const columnas: ColDef<FilaProveedorActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 0.9, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.2, minWidth: 140 },
      {
        field: "placa",
        headerName: "Placa",
        flex: 0.8,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "A pie",
      },
      { field: "gafete_numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
      {
        colId: "fecha_ingreso",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? textoFechaDDMMYYYY(p.data.fecha_hora_ingreso) : ""),
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora ingreso",
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<FilaProveedorActiva>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => registrarSalida(fila)}
            >
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [registrarSalida],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaProveedorActiva>
            id="proveedores-activos"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <>
                <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                  + Ingreso
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Cédula, nombre, empresa…"
                    value={busqueda}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <IngresoProveedorModal
            onRegistrado={() => {
              setModalAbierto(false);
              recargar();
            }}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
