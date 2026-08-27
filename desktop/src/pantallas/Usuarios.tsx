import { useCallback, useEffect, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import FormularioUsuario from "./FormularioUsuario";
import { actualizarUsuario, buscarUsuarios } from "../api";
import type { UsuarioResumen } from "../api";

const columnas: ColDef<UsuarioResumen>[] = [
  { field: "cedula", headerName: "Cédula", width: 140 },
  { field: "nombre", headerName: "Nombre", flex: 1 },
  { field: "rol", headerName: "Rol", width: 140 },
  { field: "activo", headerName: "Activo", width: 100, cellDataType: "boolean", editable: true },
];

export default function Usuarios() {
  const [texto, setTexto] = useState("");
  const [filas, setFilas] = useState<UsuarioResumen[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [formularioAbierto, setFormularioAbierto] = useState<"crear" | UsuarioResumen | null>(
    null,
  );

  const recargar = useCallback(() => {
    return buscarUsuarios({ texto: texto || undefined }).then(setFilas);
  }, [texto]);

  useEffect(() => {
    let vigente = true;
    recargar()
      .then(() => vigente && setError(null))
      .catch((error) => vigente && setError(String(error)));
    return () => {
      vigente = false;
    };
  }, [recargar]);

  async function manejarEdicion(fila: UsuarioResumen) {
    try {
      await actualizarUsuario(fila.id, {
        cedula: fila.cedula,
        nombre: fila.nombre,
        rol: fila.rol,
        activo: fila.activo,
      });
      setError(null);
    } catch (error) {
      setError(String(error));
      recargar();
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "1rem" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: "0.75rem",
        }}
      >
        <h1 style={{ margin: 0, fontSize: "1.2rem", color: "var(--acento)" }}>Usuarios</h1>
        <button className="boton boton-primario" onClick={() => setFormularioAbierto("crear")}>
          + Nuevo usuario
        </button>
      </div>

      {error && <p style={{ color: "var(--error)" }}>{error}</p>}
      <div style={{ flex: 1, minHeight: 0 }}>
        <Tabla<UsuarioResumen>
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
