import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { useVerificacionPorCorreo } from "../componentes/useVerificacionPorCorreo";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { agregarAdministrador, eliminarAdministrador, listarAdministradores } from "../api/administradores";
import type { AdministradorPanel } from "../api/administradores";
import type { UsuarioSesion } from "../api";

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

function mensajeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Alta/baja de quién puede entrar al panel — esto ES la autorización real
 * (ver `AuthContexto.tsx` y la migración `crea_administradores_panel`), no
 * una pantalla de conveniencia. Agregar Y quitar piden confirmación por
 * correo (ver `useVerificacionPorCorreo`): el código llega a quien hace la
 * acción, no al admin nuevo/afectado -- es un "sos vos ahora mismo", no una
 * verificación del correo ajeno. A diferencia de la versión anterior (magic
 * link + retomar la acción al volver del redirect, ver `App.tsx` en git
 * history), el código de 6 dígitos se confirma en esta misma pestaña, así
 * que el INSERT/DELETE en `administradores_panel` pasa directo acá, sin
 * pasar el estado a otra pantalla.
 */
export default function Administradores({ sesion }: { sesion: UsuarioSesion }) {
  const [filas, setFilas] = useState<AdministradorPanel[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [correoNuevo, setCorreoNuevo] = useState("");
  const [codigoAlta, setCodigoAlta] = useState("");
  const [confirmandoAlta, setConfirmandoAlta] = useState(false);

  const [bajaEnCurso, setBajaEnCurso] = useState<AdministradorPanel | null>(null);
  const [codigoBaja, setCodigoBaja] = useState("");
  const [confirmandoBaja, setConfirmandoBaja] = useState(false);

  const confirmacionAlta = useVerificacionPorCorreo(sesion.correo);
  const confirmacionBaja = useVerificacionPorCorreo(sesion.correo);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarAdministradores()
      .then(setFilas)
      .catch((error) => {
        if (!silencioso) toast.error(mensajeError(error));
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
    setCodigoAlta("");
    confirmacionAlta.reiniciar();
  }

  async function alPedirCodigoAlta(evento: React.FormEvent) {
    evento.preventDefault();
    await confirmacionAlta.pedirConfirmacion();
  }

  async function alConfirmarCodigoAlta(evento: React.FormEvent) {
    evento.preventDefault();
    setConfirmandoAlta(true);
    try {
      await confirmacionAlta.confirmarCodigo(codigoAlta);
    } catch {
      setConfirmandoAlta(false);
      return; // el error ya quedó en confirmacionAlta.error, mostrado inline
    }
    try {
      await agregarAdministrador(correoNuevo);
      toast.success(`${correoNuevo} ya tiene acceso.`);
      cerrarModal();
      recargar();
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setConfirmandoAlta(false);
    }
  }

  function cerrarBaja() {
    setBajaEnCurso(null);
    setCodigoBaja("");
    confirmacionBaja.reiniciar();
  }

  async function alBorrar(fila: AdministradorPanel) {
    if (
      !confirm(
        `Se te va a mandar un código de confirmación a ${sesion.correo}. ¿Continuar para sacarle el acceso a ${fila.correo}?`,
      )
    )
      return;
    setBajaEnCurso(fila);
    await confirmacionBaja.pedirConfirmacion();
  }

  async function alConfirmarCodigoBaja(evento: React.FormEvent) {
    evento.preventDefault();
    if (!bajaEnCurso) return;
    setConfirmandoBaja(true);
    try {
      await confirmacionBaja.confirmarCodigo(codigoBaja);
    } catch {
      setConfirmandoBaja(false);
      return;
    }
    try {
      await eliminarAdministrador(bajaEnCurso.correo);
      toast.success(`${bajaEnCurso.correo} ya no tiene acceso.`);
      cerrarBaja();
      recargar();
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setConfirmandoBaja(false);
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
            <form
              onSubmit={alConfirmarCodigoAlta}
              style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
            >
              <p style={{ margin: 0 }}>
                Te mandamos un código a <strong>{sesion.correo}</strong>. Escribilo acá para
                confirmar que agregás a {correoNuevo}.
              </p>
              <label className="campo">
                Código de 6 dígitos
                <input
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  maxLength={6}
                  required
                  autoFocus
                  value={codigoAlta}
                  disabled={confirmandoAlta}
                  placeholder="123456"
                  onChange={(evento) => setCodigoAlta(evento.target.value)}
                />
              </label>

              {confirmacionAlta.error && (
                <p className="login-error" role="alert">
                  {confirmacionAlta.error}
                </p>
              )}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button type="button" className="boton" disabled={confirmandoAlta} onClick={cerrarModal}>
                  Cancelar
                </button>
                <button type="submit" className="boton boton-primario" disabled={confirmandoAlta}>
                  {confirmandoAlta ? "Confirmando…" : "Confirmar"}
                </button>
              </div>
            </form>
          ) : (
            <form
              onSubmit={alPedirCodigoAlta}
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
                  onChange={(evento) => setCorreoNuevo(evento.target.value.trim().toLowerCase())}
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
                  {confirmacionAlta.enviando ? "Enviando…" : "Enviar código de confirmación"}
                </button>
              </div>
            </form>
          )}
        </Modal>
      )}

      {bajaEnCurso && (
        <Modal titulo={`Confirmar — sacarle el acceso a ${bajaEnCurso.correo}`} onCerrar={cerrarBaja}>
          {confirmacionBaja.enviado ? (
            <form
              onSubmit={alConfirmarCodigoBaja}
              style={{ display: "flex", flexDirection: "column", gap: "1rem" }}
            >
              <p style={{ margin: 0 }}>
                Te mandamos un código a <strong>{sesion.correo}</strong>. Escribilo acá para
                confirmar.
              </p>
              <label className="campo">
                Código de 6 dígitos
                <input
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  maxLength={6}
                  required
                  autoFocus
                  value={codigoBaja}
                  disabled={confirmandoBaja}
                  placeholder="123456"
                  onChange={(evento) => setCodigoBaja(evento.target.value)}
                />
              </label>

              {confirmacionBaja.error && (
                <p className="login-error" role="alert">
                  {confirmacionBaja.error}
                </p>
              )}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button type="button" className="boton" disabled={confirmandoBaja} onClick={cerrarBaja}>
                  Cancelar
                </button>
                <button type="submit" className="boton boton-primario" disabled={confirmandoBaja}>
                  {confirmandoBaja ? "Confirmando…" : "Confirmar"}
                </button>
              </div>
            </form>
          ) : confirmacionBaja.error ? (
            <p className="login-error" role="alert">
              {confirmacionBaja.error}
            </p>
          ) : (
            <p style={{ margin: 0, color: "var(--muted)" }}>Enviando código a {sesion.correo}…</p>
          )}
        </Modal>
      )}
    </div>
  );
}
