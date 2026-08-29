import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import PantallaEncabezado from "../componentes/PantallaEncabezado";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import FormularioUsuario from "./FormularioUsuario";
import { actualizarUsuario, buscarUsuarios } from "../api";
import type { UsuarioResumen } from "../api";

const columnas: ColDef<UsuarioResumen>[] = [
  { field: "cedula", headerName: "Cédula", width: 140, cellStyle: { textAlign: "left" } },
  { field: "nombre", headerName: "Nombre", flex: 1, cellStyle: { textAlign: "left" } },
  { field: "rol", headerName: "Rol", width: 140 },
  { field: "activo", headerName: "Activo", width: 100, cellDataType: "boolean", editable: true },
];

export default function Usuarios() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<UsuarioResumen[]>([]);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | UsuarioResumen | null>(
    null,
  );

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  const recargar = useCallback(() => {
    return buscarUsuarios({ texto: texto || undefined }).then(setFilas);
  }, [texto]);

  useCargaAlCambiar(recargar);

  async function manejarEdicion(fila: UsuarioResumen) {
    try {
      await actualizarUsuario(fila.id, {
        cedula: fila.cedula,
        nombre: fila.nombre,
        rol: fila.rol,
        activo: fila.activo,
      });
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <PantallaEncabezado
        titulo="Usuarios"
        acciones={
          <button
            className="boton boton-primario"
            title="Ctrl+N"
            onClick={() => setFormularioAbierto("crear")}
          >
            + Nuevo usuario
          </button>
        }
      />

      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<UsuarioResumen>
            id="usuarios"
            columnas={columnas}
            filas={filas}
            onCeldaEditada={manejarEdicion}
            onFilaDobleClic={setFormularioAbierto}
            controles={
              <div className="campo" style={{ flex: "1 1 16rem" }}>
                Buscar
                <input
                  placeholder="Cédula o nombre…"
                  value={texto}
                  onChange={(evento) => setTexto(evento.target.value)}
                />
              </div>
            }
          />
        </div>
      </div>

      {formularioAbierto && (
        <FormularioUsuario
          usuario={formularioAbierto === "crear" ? undefined : formularioAbierto}
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
