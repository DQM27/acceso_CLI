import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  agregarAdministrador,
  eliminarAdministrador,
  listarAdministradores,
} from "../api/administradores";
import type { AdministradorPanel } from "../api/administradores";
import type { RolAdminPanel, UsuarioSesion } from "../api";

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Alta/baja de quién puede entrar al panel — esto ES la autorización real
 * (ver `AuthContexto.tsx` y la migración `crea_administradores_panel`),
 * no una pantalla de conveniencia. Sin correo automático al agregar a
 * alguien: se le avisa la persona que lo agrega, por fuera del panel
 * (decisión explícita, ver conversación — agregar el envío de correo es
 * una Edge Function más para mantener, sin beneficio real con tan pocos
 * admins).
 */
export default function Administradores({ sesion }: { sesion: UsuarioSesion }) {
  const [filas, setFilas] = useState<AdministradorPanel[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [correoNuevo, setCorreoNuevo] = useState("");
  const [rolNuevo, setRolNuevo] = useState<RolAdminPanel>("admin_regional");
  const [guardando, setGuardando] = useState(false);

  const recargar = useCallback(() => {
    setCargando(true);
    return listarAdministradores()
      .then(setFilas)
      .catch((error) => toast.error(String(error)))
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  async function alAgregar(evento: React.FormEvent) {
    evento.preventDefault();
    setGuardando(true);
    try {
      await agregarAdministrador(correoNuevo, rolNuevo);
      toast.success(`${correoNuevo} ya puede entrar al panel.`);
      setModalAbierto(false);
      setCorreoNuevo("");
      setRolNuevo("admin_regional");
      recargar();
    } catch (error) {
      toast.error(String(error));
    } finally {
      setGuardando(false);
    }
  }

  async function alBorrar(fila: AdministradorPanel) {
    if (!confirm(`¿Sacarle el acceso a ${fila.correo}?`)) return;
    try {
      await eliminarAdministrador(fila.correo);
      toast.success(`${fila.correo} ya no tiene acceso.`);
      recargar();
    } catch (error) {
      toast.error(String(error));
    }
  }

  const columnas: ColDef<AdministradorPanel>[] = [
    { field: "correo", headerName: "Correo", flex: 1.8, minWidth: 220, cellStyle: { textAlign: "left" } },
    { field: "rol", headerName: "Rol", flex: 1, minWidth: 140 },
    {
      field: "creado_en",
      headerName: "Agregado",
      flex: 1.2,
      minWidth: 160,
      valueFormatter: ({ value }) => textoFechaHora(value),
    },
    {
      colId: "acciones",
      headerName: "",
      flex: 0.8,
      minWidth: 110,
      sortable: false,
      filter: false,
      cellRenderer: ({ data }: { data: AdministradorPanel }) =>
        data.correo === sesion.correo ? null : (
          <button
            type="button"
            className="boton"
            style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
            onClick={() => alBorrar(data)}
          >
            Quitar
          </button>
        ),
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<AdministradorPanel>
            id="administradores"
            columnas={columnas}
            filas={filas}
            controles={
              <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                + Nuevo administrador
              </button>
            }
          />
        </div>
        {cargando && filas.length === 0 && (
          <p style={{ color: "var(--muted)" }}>Cargando…</p>
        )}
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo administrador" onCerrar={() => setModalAbierto(false)}>
          <form onSubmit={alAgregar} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <label className="campo">
              Correo de Google
              <input
                type="email"
                required
                autoFocus
                value={correoNuevo}
                disabled={guardando}
                placeholder="nombre@gmail.com"
                onChange={(evento) => setCorreoNuevo(evento.target.value)}
              />
            </label>

            <label className="campo">
              Rol
              <select
                value={rolNuevo}
                disabled={guardando}
                onChange={(evento) => setRolNuevo(evento.target.value as RolAdminPanel)}
              >
                <option value="admin_regional">Administrador regional</option>
                <option value="admin_global">Administrador global</option>
              </select>
            </label>

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button
                type="button"
                className="boton"
                disabled={guardando}
                onClick={() => setModalAbierto(false)}
              >
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario" disabled={guardando}>
                {guardando ? "Agregando…" : "Agregar"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
