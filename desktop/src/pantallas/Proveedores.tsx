import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarFilaProveedorActiva,
  claveFilaProveedorActiva,
  listarHistorialIngresosProveedorSitio,
  listarTodosLosProveedoresActivos,
} from "../api/proveedores";
import type { FilaProveedorActiva, HistorialIngresoProveedorRemoto } from "../api/proveedores";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const IngresoProveedorModal = lazy(() => import("./IngresoProveedorModal"));

type Vista = "activos" | "historial";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  activos: "Activos",
  historial: "Historial",
};

// Duplicado a propósito de `Visitas.tsx`/`CatalogoRutas.tsx` -- es puro
// markup de un botón, no vale la pena compartirlo entre pantallas que no
// se importan una a otra (ver la convención de App.tsx).
function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <div style={{ display: "flex", gap: "0.25rem" }}>
      {(Object.keys(ETIQUETAS_VISTA) as Vista[]).map((opcion) => (
        <button
          key={opcion}
          type="button"
          className={opcion === vista ? "boton boton-primario" : "boton"}
          disabled={opcion === vista}
          onClick={() => onCambiar(opcion)}
        >
          {ETIQUETAS_VISTA[opcion]}
        </button>
      ))}
    </div>
  );
}

/**
 * Pantalla operativa de proveedores -- mismo patrón que Rutas.tsx: grilla de
 * "activos" + botón "+ Nuevo" que abre el registro en un modal, y un botón
 * directo "Salida" por fila. `refrescarSenal` (de `Shell`) recarga sola
 * cuando llega cualquier sincronización, igual que Activos/Rutas. La lista
 * de activos fusiona locales + los que otro dispositivo del sitio tiene
 * abiertos ahora (`listarTodosLosProveedoresActivos`), mismo criterio que
 * `listarTodosLosActivos` para contratistas.
 *
 * El toggle "Activos/Historial" (mismo patrón que Visitas.tsx) es
 * EXCLUSIVO de escritorio: "Historial" lee `historial_ingresos_proveedor_sitio`,
 * una caché que sólo sincroniza la PC -- el celular no la trae a propósito
 * (mismo criterio ya usado en `historial_visitas_sitio`: auditar el
 * historial completo del sitio es tarea de escritorio, el celular es para
 * registros rápidos).
 */
/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

export default function Proveedores({ refrescarSenal }: { refrescarSenal?: number }) {
  const [vista, setVista] = useState<Vista>("activos");
  // Período del historial -- mismo selector y mismo arranque ("Últimos 6
  // meses", `hasta` abierto) que Historial de contratistas (pedido del
  // usuario 2026-09-23: el selector en todos los historiales).
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const [filasActivos, setFilasActivos] = useState<FilaProveedorActiva[]>([]);
  const [filasHistorial, setFilasHistorial] = useState<HistorialIngresoProveedorRemoto[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  const total = vista === "activos" ? filasActivos.length : filasHistorial.length;
  useBarraEstado(
    cargando ? "Cargando…" : `${total} ${vista === "historial" ? "movimientos" : "proveedor(es) activo(s)"}`,
  );

  const recargarActivos = useCallback(() => {
    setCargando(true);
    return listarTodosLosProveedoresActivos()
      .then(setFilasActivos)
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return listarHistorialIngresosProveedorSitio(desde || undefined, hasta || undefined)
      .then(setFilasHistorial)
      .finally(() => setCargando(false));
  }, [desde, hasta]);

  useEffect(() => {
    let vigente = true;
    const recargar = vista === "activos" ? recargarActivos : recargarHistorial;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización (Realtime, pulso
    // periódico o manual) -- mismo motivo que Visitas.tsx: sin esto, un
    // ingreso registrado desde otro dispositivo del sitio sólo aparecía
    // acá después de cambiar de pestaña y volver.
  }, [vista, refrescarSenal, recargarActivos, recargarHistorial]);

  const registrarSalida = useCallback(
    async (fila: FilaProveedorActiva) => {
      try {
        await cerrarFilaProveedorActiva(fila);
        await recargarActivos();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargarActivos],
  );

  const columnasActivos: ColDef<FilaProveedorActiva>[] = useMemo(
    () => [
      { field: "cedula", headerName: "Cédula", flex: 0.9, minWidth: 110, cellStyle: { textAlign: "left" } },
      { field: "nombre", headerName: "Nombre", flex: 1.4, minWidth: 160, cellStyle: { textAlign: "left" } },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.2, minWidth: 140 },
      {
        field: "placa",
        headerName: "Placa",
        flex: 0.8,
        minWidth: 100,
        valueFormatter: (p) => p.value ?? "Caminando",
      },
      { field: "gafete_numero", type: "numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
      {
        colId: "fecha_ingreso",
        type: "fecha",
        headerName: "Fecha",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
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

  const columnasHistorial: ColDef<HistorialIngresoProveedorRemoto>[] = useMemo(
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
          {vista === "activos" ? (
            <Tabla<FilaProveedorActiva>
              cargando={cargando}
              filtrosPorColumna
              id="proveedores-activos"
              idFila={claveFilaProveedorActiva}
              columnas={columnasActivos}
              filas={filasActivos}
              busqueda={busqueda}
              controles={
                <>
                  <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                    + Nuevo
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
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          ) : (
            <Tabla<HistorialIngresoProveedorRemoto>
              cargando={cargando}
              filtrosPorColumna
              id="proveedores-historial"
              idFila={idPorUuid}
              columnas={columnasHistorial}
              filas={filasHistorial}
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
                  <ToggleVista vista={vista} onCambiar={setVista} />
                </>
              }
            />
          )}
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <IngresoProveedorModal
            onRegistrado={() => {
              setModalAbierto(false);
              recargarActivos();
            }}
            onCerrar={() => setModalAbierto(false)}
          />
        )}
      </Suspense>
    </div>
  );
}
