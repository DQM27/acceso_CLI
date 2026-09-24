import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarFilaGafeteProvisionalActiva,
  claveFilaGafeteProvisionalActiva,
  listarGafetesProvisionalesHistorialSitio,
  listarTodosLosGafetesProvisionalesActivos,
} from "../api/gafetesProvisionales";
import type {
  FilaGafeteProvisionalActiva,
  PrestamoGafeteProvisionalHistorialSitio,
} from "../api/gafetesProvisionales";
import { fechaHaceMeses, fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const EntregarGafeteProvisionalModal = lazy(() => import("./EntregarGafeteProvisionalModal"));

type Vista = "activos" | "historial";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  activos: "Activos",
  historial: "Historial",
};

// Duplicado a propósito de `Proveedores.tsx`/`Visitas.tsx`/`CatalogoRutas.tsx`
// -- es puro markup de un botón, no vale la pena compartirlo entre
// pantallas que no se importan una a otra (ver la convención de App.tsx).
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
 * Entrega/devolución de gafetes provisionales KOF -- contraparte de
 * escritorio de `PantallaGafetesProvisionales.kt` (mobile). Mismo patrón
 * que Rutas/Proveedores: grilla de "prestados" + botón "+ Nuevo" que
 * abre el registro en un modal, y un botón "Devolver" por fila -- a
 * diferencia de mobile (tarjeta completa tocable), acá se sigue la
 * convención ya establecida de escritorio (botón de acción en la columna,
 * ver `Proveedores.tsx`/`Rutas.tsx`), no la de mobile.
 *
 * La lista de "prestados" fusiona locales + lo que otro dispositivo del
 * sitio tiene abierto ahora (`listarTodosLosGafetesProvisionalesActivos`),
 * mismo criterio que `listarTodosLosProveedoresActivos` -- faltaba por
 * completo hasta esta sesión (bug reportado en pruebas reales,
 * 2026-09-17: un préstamo hecho en el celular nunca aparecía acá).
 *
 * El toggle "Activos/Historial" (mismo patrón que Proveedores.tsx/
 * Visitas.tsx, agregado 2026-09-22 tras una falencia detectada por el
 * usuario) es EXCLUSIVO de escritorio: "Historial" lee
 * `prestamos_gafete_provisional_historial_sitio`, una caché que sólo
 * sincroniza la PC -- el celular no la trae a propósito (mismo criterio ya
 * usado en `historial_visitas_sitio`/`historial_ingresos_proveedor_sitio`:
 * auditar el historial completo del sitio es tarea de escritorio).
 */
/** Identidad de fila para el destello de celdas cambiadas (`idFila` de
 * `Tabla`) -- a nivel de módulo para que sea una función estable. */
const idPorUuid = (fila: { uuid: string }) => fila.uuid;

export default function GafetesProvisionales({ refrescarSenal }: { refrescarSenal?: number }) {
  const [vista, setVista] = useState<Vista>("activos");
  // Período del historial -- mismo selector y mismo arranque ("Últimos 6
  // meses", `hasta` abierto) que Historial de contratistas (pedido del
  // usuario 2026-09-23: el selector en todos los historiales).
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  const [filasActivos, setFilasActivos] = useState<FilaGafeteProvisionalActiva[]>([]);
  const [filasHistorial, setFilasHistorial] = useState<PrestamoGafeteProvisionalHistorialSitio[]>(
    [],
  );
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  const total = vista === "activos" ? filasActivos.length : filasHistorial.length;
  useBarraEstado(
    cargando
      ? "Cargando…"
      : `${total} ${vista === "historial" ? "movimiento(s)" : "gafete(s) prestado(s)"}`,
  );

  const recargarActivos = useCallback(() => {
    setCargando(true);
    return listarTodosLosGafetesProvisionalesActivos()
      .then(setFilasActivos)
      .finally(() => setCargando(false));
  }, []);

  const recargarHistorial = useCallback(() => {
    setCargando(true);
    return listarGafetesProvisionalesHistorialSitio(desde || undefined, hasta || undefined)
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
    // `refrescarSenal` sube en cada sincronización -- mismo motivo que
    // Proveedores.tsx/Visitas.tsx: sin esto, una entrega/devolución hecha
    // desde otro dispositivo del sitio sólo aparecía acá al cambiar de
    // pestaña y volver.
  }, [vista, refrescarSenal, recargarActivos, recargarHistorial]);

  const registrarDevolucion = useCallback(
    async (fila: FilaGafeteProvisionalActiva) => {
      try {
        await cerrarFilaGafeteProvisionalActiva(fila);
        await recargarActivos();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargarActivos],
  );

  const columnasActivos: ColDef<FilaGafeteProvisionalActiva>[] = useMemo(
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
        headerName: "Hora entrega",
        flex: 0.8,
        minWidth: 90,
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_entrega) : ""),
      },
      {
        field: "usuario_entrega_nombre",
        headerName: "Entregó",
        flex: 1.1,
        minWidth: 130,
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<FilaGafeteProvisionalActiva>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => registrarDevolucion(fila)}
            >
              Devolver
            </button>
          ) : null;
        },
      },
    ],
    [registrarDevolucion],
  );

  const columnasHistorial: ColDef<PrestamoGafeteProvisionalHistorialSitio>[] = useMemo(
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
          {vista === "activos" ? (
            <Tabla<FilaGafeteProvisionalActiva>
              cargando={cargando}
              filtrosPorColumna
              id="gafetes-provisionales-activos"
              idFila={claveFilaGafeteProvisionalActiva}
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
                      placeholder="Nombre o código de empleado…"
                      value={busqueda}
                      onChange={(evento) => setBusqueda(evento.target.value)}
                    />
                  </div>
                </>
              }
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          ) : (
            <Tabla<PrestamoGafeteProvisionalHistorialSitio>
              cargando={cargando}
              filtrosPorColumna
              id="gafetes-provisionales-historial"
              idFila={idPorUuid}
              columnas={columnasHistorial}
              filas={filasHistorial}
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
          <EntregarGafeteProvisionalModal
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
