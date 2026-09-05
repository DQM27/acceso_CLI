import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { useVerificacionPorCorreo } from "../componentes/useVerificacionPorCorreo";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { guardarAccionPendiente } from "../componentes/accionesPendientes";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { listarAdministradores } from "../api/administradores";
import type { AdministradorPanel } from "../api/administradores";
import type { UsuarioSesion } from "../api";

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Alta/baja de quién puede entrar al panel — esto ES la autorización real
 * (ver `AuthContexto.tsx` y la migración `crea_administradores_panel`), no
 * una pantalla de conveniencia. Agregar Y quitar piden confirmación por
 * correo (ver `useVerificacionPorCorreo`): el correo llega a quien hace la
 * acción, no al admin nuevo/afectado -- es un "sos vos ahora mismo", no
 * una verificación del correo ajeno. La acción de verdad (el INSERT/DELETE
 * en `administradores_panel`) no pasa acá -- queda guardada
 * (`guardarAccionPendiente`) y `App.tsx` la retoma cuando la persona vuelve
 * a abrir el panel después de hacer clic en el link del correo.
 */
export default function Administradores({ sesion }: { sesion: UsuarioSesion }) {
  const [filas, setFilas] = useState<AdministradorPanel[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [correoNuevo, setCorreoNuevo] = useState("");

  const confirmacionAlta = useVerificacionPorCorreo(sesion.correo);
  const confirmacionBaja = useVerificacionPorCorreo(sesion.correo);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarAdministradores()
      .then(setFilas)
      .catch((error) => {
        if (!silencioso) toast.error(String(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, []);

  // Cambia rara vez (alta/baja de admins del panel) -- mismo intervalo que
  // usan desktop/mobile para su propio sync periódico.
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000);

  useEffect(() => {
    recargar();
  }, [recargar]);

  function cerrarModal() {
    setModalAbierto(false);
    setCorreoNuevo("");
    confirmacionAlta.reiniciar();
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    guardarAccionPendiente({
      tipo: "agregar_admin",
      correoSolicitante: sesion.correo,
      correoNuevo: correoNuevo.trim().toLowerCase(),
    });
    await confirmacionAlta.pedirConfirmacion();
  }

  async function alBorrar(fila: AdministradorPanel) {
    if (
      !confirm(
        `Se te va a mandar un link de confirmación a ${sesion.correo}. ¿Continuar para sacarle el acceso a ${fila.correo}?`,
      )
    )
      return;
    guardarAccionPendiente({
      tipo: "quitar_admin",
      correoSolicitante: sesion.correo,
      correoAQuitar: fila.correo,
    });
    await confirmacionBaja.pedirConfirmacion();
    if (!confirmacionBaja.error) {
      toast.info(`Revisá tu correo (${sesion.correo}) y hacé clic en el link para confirmar.`);
    }
  }

  const columnas: ColDef<AdministradorPanel>[] = [
    { field: "correo", headerName: "Correo", flex: 1.8, minWidth: 220, cellStyle: { textAlign: "left" } },
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
          {confirmacionAlta.enviado ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
              <p style={{ margin: 0 }}>
                Te mandamos un link de confirmación a <strong>{sesion.correo}</strong>. Abrilo y
                hacé clic — cuando vuelvas a abrir el panel, {correoNuevo} ya va a tener acceso.
              </p>
              <p style={{ margin: 0, color: "var(--muted)", fontSize: "0.85rem" }}>
                Podés cerrar esta ventana, no hace falta esperar acá.
              </p>
              <div style={{ display: "flex", justifyContent: "flex-end" }}>
                <button type="button" className="boton boton-primario" onClick={cerrarModal}>
                  Listo
                </button>
              </div>
            </div>
          ) : (
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
                  disabled={confirmacionAlta.enviando}
                  placeholder="nombre@gmail.com"
                  onChange={(evento) => setCorreoNuevo(evento.target.value)}
                />
              </label>

              {confirmacionAlta.error && (
                <p className="login-error" role="alert">
                  {confirmacionAlta.error}
                </p>
              )}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button
                  type="button"
                  className="boton"
                  disabled={confirmacionAlta.enviando}
                  onClick={cerrarModal}
                >
                  Cancelar
                </button>
                <button
                  type="submit"
                  className="boton boton-primario"
                  disabled={confirmacionAlta.enviando}
                >
                  {confirmacionAlta.enviando ? "Enviando…" : "Enviar link de confirmación"}
                </button>
              </div>
            </form>
          )}
        </Modal>
      )}
    </div>
  );
}
