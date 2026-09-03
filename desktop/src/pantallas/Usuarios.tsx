import { useCallback, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioUsuario from "./FormularioUsuario";
import { actualizarUsuario, buscarUsuarios } from "../api";
import type { RolUsuario, UsuarioResumen } from "../api";

const columnas: ColDef<UsuarioResumen>[] = [
  { field: "cedula", headerName: "Cédula", flex: 1.4, minWidth: 140, cellStyle: { textAlign: "left" } },
  { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
  { field: "rol", headerName: "Rol", flex: 1.4, minWidth: 140 },
  {
    field: "activo",
    headerName: "Activo",
    flex: 1,
    minWidth: 100,
    cellRenderer: InterruptorCelda,
    cellRendererParams: { critico: true },
  },
];

export default function Usuarios({ actorRol }: { actorRol: RolUsuario }) {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<UsuarioResumen[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | UsuarioResumen | null>(
    null,
  );

  useBarraEstado(cargando ? "Cargando…" : `${filas.length} resultado(s)`);

  useHotkeys("ctrl+n", () => setFormularioAbierto("crear"), { preventDefault: true });

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      return buscarUsuarios({ texto: texto || undefined })
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
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<UsuarioResumen>
            id="usuarios"
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
                    placeholder="Cédula o nombre…"
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
        <FormularioUsuario
          actorRol={actorRol}
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
