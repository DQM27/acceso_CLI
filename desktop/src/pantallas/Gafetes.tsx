import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioGafete from "./FormularioGafete";
import GestionGafeteModal from "./GestionGafeteModal";
import { buscarGafetes } from "../api";
import type { GafeteResumen } from "../api";

const columnas: ColDef<GafeteResumen>[] = [
  {
    field: "numero",
    headerName: "Número",
    width: 110,
    valueFormatter: ({ value }) => String(value).padStart(2, "0"),
  },
  { field: "estado", headerName: "Estado", width: 130 },
  {
    field: "contratista_deudor_nombre",
    headerName: "Deudor",
    flex: 1,
    cellStyle: { textAlign: "left" },
  },
];

/**
 * Catálogo de gafetes (`docs/plan-gafetes.md`) — sin restricción de rol a
 * propósito, mismo criterio que el núcleo: cualquier operador con sesión
 * gestiona alta/baja/perdido/resolver. Doble click en una fila abre las
 * acciones disponibles según su estado (mismo criterio que la TUI: B/P/R
 * sólo aplican según el estado actual, ver `src/tui/gafetes/state.rs`).
 */
export default function Gafetes() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<GafeteResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState(false);
  const [gestionAbierta, setGestionAbierta] = useState<GafeteResumen | null>(null);

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
      <PantallaEncabezado
        titulo="Gafetes"
        acciones={
          <button
            className="boton boton-primario"
            title="Ctrl+N"
            onClick={() => setFormularioAbierto(true)}
          >
            + Nuevo gafete
          </button>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<GafeteResumen>
            id="gafetes"
            columnas={columnas}
            filas={filas}
            onFilaDobleClic={setGestionAbierta}
            controles={
              <div className="campo" style={{ flex: "1 1 16rem" }}>
                Buscar por número
                <input
                  placeholder="Número…"
                  value={texto}
                  onChange={(evento) => setTexto(evento.target.value.replace(/\D/g, ""))}
                  inputMode="numeric"
                />
              </div>
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
    </div>
  );
}
