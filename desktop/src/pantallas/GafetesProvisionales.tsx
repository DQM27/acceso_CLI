import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import {
  cerrarFilaGafeteProvisionalActiva,
  listarTodosLosGafetesProvisionalesActivos,
} from "../api/gafetesProvisionales";
import type { FilaGafeteProvisionalActiva } from "../api/gafetesProvisionales";
import { textoFechaDDMMYYYY, textoHora, fechaLocalYMD } from "../tiempo";

const EntregarGafeteProvisionalModal = lazy(() => import("./EntregarGafeteProvisionalModal"));

/**
 * Entrega/devolución de gafetes provisionales KOF -- contraparte de
 * escritorio de `PantallaGafetesProvisionales.kt` (mobile). Mismo patrón
 * que Rutas/Proveedores: grilla de "prestados" + botón "+ Entregar" que
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
 * Sin vista de "Historial" aparte -- a diferencia de Proveedores/Visitas,
 * el volumen de este flujo es bajo (uno o dos olvidos por turno, no
 * decenas), no amerita una segunda grilla; quien necesite auditar
 * devoluciones ya puede hacerlo contra `prestamos_gafete_provisional` en
 * Supabase directamente.
 */
export default function GafetesProvisionales({ refrescarSenal }: { refrescarSenal?: number }) {
  const [filas, setFilas] = useState<FilaGafeteProvisionalActiva[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [busqueda, setBusqueda] = useState("");

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} gafete(s) prestado(s)`);

  const recargar = useCallback(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- de lo contrario `react-hooks/set-state-in-effect` marca
    // esta actualización de estado como síncrona dentro del cuerpo del
    // efecto que la dispara (abajo). Mismo patrón que Rutas.tsx/Activos.tsx.
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarTodosLosGafetesProvisionalesActivos())
      .then(setFilas)
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && toast.error(String(error)));
    return () => {
      vigente = false;
    };
    // `refrescarSenal` sube en cada sincronización -- mismo motivo que
    // Proveedores.tsx/Visitas.tsx: sin esto, una entrega/devolución hecha
    // desde otro dispositivo del sitio sólo aparecía acá al cambiar de
    // pestaña y volver.
  }, [refrescarSenal, recargar]);

  const registrarDevolucion = useCallback(
    async (fila: FilaGafeteProvisionalActiva) => {
      try {
        await cerrarFilaGafeteProvisionalActiva(fila);
        await recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  const columnas: ColDef<FilaGafeteProvisionalActiva>[] = useMemo(
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
      { field: "gafete_numero", headerName: "Gafete", flex: 0.7, minWidth: 90 },
      {
        colId: "fecha_entrega",
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

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaGafeteProvisionalActiva>
            id="gafetes-provisionales-activos"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <>
                <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                  + Entregar
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
          />
        </div>
      </div>

      <Suspense fallback={null}>
        {modalAbierto && (
          <EntregarGafeteProvisionalModal
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
