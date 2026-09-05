import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { useVerificacionPorCorreo } from "../componentes/useVerificacionPorCorreo";
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
  const [codigo, setCodigo] = useState("");

  // Acción sensible: agregar un admin nuevo exige confirmar con un código
  // que llega AL CORREO DE QUIEN LO AGREGA (no al del admin nuevo) — es un
  // "sos realmente vos frente a la pantalla ahora mismo", no una
  // verificación del correo ajeno. Ver useVerificacionPorCorreo.
  const verificacion = useVerificacionPorCorreo(sesion.correo);

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

  function cerrarModal() {
    setModalAbierto(false);
    setCorreoNuevo("");
    setRolNuevo("admin_regional");
    setCodigo("");
    verificacion.reiniciar();
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();

    if (verificacion.paso === "inicial") {
      await verificacion.pedirCodigo();
      return;
    }

    const codigoValido = await verificacion.verificarCodigo(codigo);
    if (!codigoValido) return;

    try {
      await agregarAdministrador(correoNuevo, rolNuevo);
      toast.success(`${correoNuevo} ya puede entrar al panel.`);
      cerrarModal();
      recargar();
    } catch (error) {
      toast.error(String(error));
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
        <Modal titulo="Nuevo administrador" onCerrar={cerrarModal}>
          <form
            onSubmit={alEnviarFormulario}
            style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
          >
            <label className="campo">
              Correo de Google
              <input
                type="email"
                required
                autoFocus
                value={correoNuevo}
                disabled={verificacion.paso !== "inicial" || verificacion.enviando}
                placeholder="nombre@gmail.com"
                onChange={(evento) => setCorreoNuevo(evento.target.value)}
              />
            </label>

            <label className="campo">
              Rol
              <select
                value={rolNuevo}
                disabled={verificacion.paso !== "inicial" || verificacion.enviando}
                onChange={(evento) => setRolNuevo(evento.target.value as RolAdminPanel)}
              >
                <option value="admin_regional">Administrador regional</option>
                <option value="admin_global">Administrador global</option>
              </select>
            </label>

            {verificacion.paso === "codigo_enviado" && (
              <label className="campo">
                Código de confirmación
                <input
                  type="text"
                  inputMode="numeric"
                  required
                  autoFocus
                  maxLength={6}
                  value={codigo}
                  disabled={verificacion.enviando}
                  placeholder="000000"
                  onChange={(evento) => setCodigo(evento.target.value)}
                />
                <span style={{ color: "var(--muted)", fontSize: "0.78rem" }}>
                  Te mandamos un código a <strong>{sesion.correo}</strong> para confirmar que sos
                  vos — no le llega nada al correo nuevo todavía.
                </span>
              </label>
            )}

            {verificacion.error && (
              <p className="login-error" role="alert">
                {verificacion.error}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button
                type="button"
                className="boton"
                disabled={verificacion.enviando}
                onClick={cerrarModal}
              >
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario" disabled={verificacion.enviando}>
                {verificacion.enviando
                  ? "Un momento…"
                  : verificacion.paso === "inicial"
                    ? "Enviar código"
                    : "Confirmar y agregar"}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </div>
  );
}
