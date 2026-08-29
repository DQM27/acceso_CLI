import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioEmpresa from "./FormularioEmpresa";
import { buscarEmpresas, establecerEmpresaActiva } from "../api";
import type { EmpresaResumen } from "../api";

const columnas: ColDef<EmpresaResumen>[] = [
  { field: "nombre", headerName: "Nombre", flex: 1, cellStyle: { textAlign: "left" } },
  { field: "contratistas", headerName: "Contratistas", width: 130 },
  { field: "activo", headerName: "Activa", width: 100, cellDataType: "boolean", editable: true },
];

export default function Empresas() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<EmpresaResumen[]>([]);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | EmpresaResumen | null>(
    null,
  );

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  const recargar = useCallback(() => {
    return buscarEmpresas({ texto: texto || undefined }).then(setFilas);
  }, [texto]);

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
      <PantallaEncabezado
        titulo="Empresas"
        acciones={
          <button
            className="boton boton-primario"
            title="Ctrl+N"
            onClick={() => setFormularioAbierto("crear")}
          >
            + Nueva empresa
          </button>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<EmpresaResumen>
            id="empresas"
            columnas={columnas}
            filas={filas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
            controles={
              <div className="campo" style={{ flex: "1 1 16rem" }}>
                Buscar
                <input
                  placeholder="Nombre…"
                  value={texto}
                  onChange={(evento) => setTexto(evento.target.value)}
                />
              </div>
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
