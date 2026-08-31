import { useCallback, useMemo, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioGafete from "./FormularioGafete";
import GestionGafeteModal from "./GestionGafeteModal";
import HistorialGafeteModal from "./HistorialGafeteModal";
import { buscarGafetes } from "../api";
import type { GafeteResumen } from "../api";

/**
 * Catálogo de gafetes (`docs/plan-gafetes.md`) — sin restricción de rol a
 * propósito, mismo criterio que el núcleo: cualquier operador con sesión
 * gestiona alta/baja/perdido/resolver. Doble click en una fila abre las
 * acciones disponibles según su estado (mismo criterio que la TUI: B/P/R
 * sólo aplican según el estado actual, ver `src/tui/gafetes/state.rs`) — la
 * columna "Resolver" es el mismo modal, explícita a propósito para el
 * operador que no conozca el doble click (sólo visible en estado Perdido,
 * que es cuando resolver aplica). La columna "Historial" abre por separado
 * (`HistorialGafeteModal`) quién marcó perdido/resolvió cada incidente —
 * separado de la gestión para no mezclar "qué puedo hacer" con "qué le pasó
 * antes".
 */
export default function Gafetes() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<GafeteResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState(false);
  const [gestionAbierta, setGestionAbierta] = useState<GafeteResumen | null>(null);
  const [detalleAbierto, setDetalleAbierto] = useState<GafeteResumen | null>(null);

  const columnas: ColDef<GafeteResumen>[] = useMemo(
    () => [
      {
        field: "numero",
        headerName: "Número",
        width: 110,
        valueFormatter: ({ value }) => String(value).padStart(2, "0"),
      },
      { field: "estado", headerName: "Estado", width: 130 },
      {
        field: "contratista_deudor_nombre",
        headerName: "Asignado a",
        flex: 1,
        cellStyle: { textAlign: "left" },
      },
      {
        headerName: "Resolver",
        width: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<GafeteResumen>) =>
          p.data && p.data.estado === "Perdido" ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => setGestionAbierta(p.data!)}
            >
              Resolver
            </button>
          ) : null,
      },
      {
        headerName: "Historial",
        width: 110,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<GafeteResumen>) =>
          p.data ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
              onClick={() => setDetalleAbierto(p.data!)}
            >
              Detalles
            </button>
          ) : null,
      },
    ],
    [],
  );

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      const numero = /^\d+$/.test(texto.trim()) ? Number(texto.trim()) : undefined;
      return buscarGafetes({ numero })
        .then((datos) => {
          if (estaVigente()) setFilas(datos);
        })
        .finally(() => {
          if (estaVigente()) setCargando(false);
        });
    },
    [texto],
  );

  useHotkeys("ctrl+n", () => setFormularioAbierto(true), { preventDefault: true });
  useCargaAlCambiar(recargar);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado titulo="Gafetes" />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<GafeteResumen>
            id="gafetes"
            columnas={columnas}
            filas={filas}
            onFilaDobleClic={setGestionAbierta}
            controles={
              <>
                <button className="boton" title="Ctrl+N" onClick={() => setFormularioAbierto(true)}>
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "1 1 16rem" }}>
                  <input
                    placeholder="Número…"
                    value={texto}
                    onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
                    inputMode="numeric"
                  />
                </div>
              </>
            }
          />
        </div>
        <p style={{ color: "var(--muted)", margin: 0 }}>
          {cargando ? "Cargando…" : `${filas.length} resultado(s)`}
        </p>
      </div>

      {formularioAbierto && (
        <FormularioGafete
          onCerrar={() => setFormularioAbierto(false)}
          onGuardado={() => {
            setFormularioAbierto(false);
            recargar();
          }}
        />
      )}

      {gestionAbierta && (
        <GestionGafeteModal
          gafete={gestionAbierta}
          onCerrar={() => setGestionAbierta(null)}
          onCambiado={() => {
            setGestionAbierta(null);
            recargar();
          }}
        />
      )}

      {detalleAbierto && (
        <HistorialGafeteModal gafete={detalleAbierto} onCerrar={() => setDetalleAbierto(null)} />
      )}
    </div>
  );
}
