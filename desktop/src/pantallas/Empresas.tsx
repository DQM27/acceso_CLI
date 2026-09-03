import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioEmpresa from "./FormularioEmpresa";
import { buscarEmpresas, establecerEmpresaActiva } from "../api";
import type { EmpresaResumen } from "../api";

const columnas: ColDef<EmpresaResumen>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  { field: "contratistas", headerName: "Contratistas", flex: 1.3, minWidth: 130 },
  {
    field: "activo",
    headerName: "Activa",
    flex: 1,
    minWidth: 100,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
  },
];

export default function Empresas() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<EmpresaResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | EmpresaResumen | null>(
    null,
  );

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      return buscarEmpresas({ texto: texto || undefined })
        .then((datos) => {
          if (estaVigente()) setFilas(datos);
        })
        .finally(() => {
          if (estaVigente()) setCargando(false);
        });
    },
    [texto],
  );

  useCargaAlCambiar(recargar);

  async function manejarEdicion(fila: EmpresaResumen) {
    try {
      await establecerEmpresaActiva(fila.id, fila.activo);
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<EmpresaResumen>
            id="empresas"
            columnas={columnas}
            filas={filas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
            controles={
              <>
                <button
                  className="boton"
                  title="Ctrl+N"
                  onClick={() => setFormularioAbierto("crear")}
                >
                  + Nuevo
                </button>
                <div className="campo" style={{ flex: "0 1 16rem" }}>
                  <input
                    placeholder="Nombre…"
                    value={texto}
                    onChange={(evento) => setTexto(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioEmpresa
          empresa={formularioAbierto === "crear" ? undefined : formularioAbierto}
          onCerrar={() => setFormularioAbierto(null)}
          onGuardado={() => {
            setFormularioAbierto(null);
            recargar();
          }}
        />
      )}
    </div>
  );
}
