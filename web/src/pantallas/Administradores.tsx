import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import ConfirmacionSensible from "../componentes/ConfirmacionSensible";
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
 * correo (ver `ConfirmacionSensible`/`useVerificacionPorCorreo`): el código
 * llega a quien hace la acción, no al admin nuevo/afectado -- es un "sos vos
 * ahora mismo", no una verificación del correo ajeno.
 */
export default function Administradores({ sesion }: { sesion: UsuarioSesion }) {
  const [filas, setFilas] = useState<AdministradorPanel[]>([]);
  const [cargando, setCargando] = useState(true);

  const [modalAbierto, setModalAbierto] = useState(false);
  const [correoNuevo, setCorreoNuevo] = useState("");
  const [pidiendoCodigoAlta, setPidiendoCodigoAlta] = useState(false);

  const [bajaEnCurso, setBajaEnCurso] = useState<AdministradorPanel | null>(null);

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
  }

  function alPedirCorreoNuevo(evento: React.FormEvent) {
    evento.preventDefault();
    setPidiendoCodigoAlta(true);
  }

  async function alConfirmarAlta() {
    try {
      await agregarAdministrador(correoNuevo);
      toast.success(`${correoNuevo} ya tiene acceso.`);
      cerrarModal();
      recargar();
    } catch (error) {
      toast.error(mensajeError(error));
    }
  }

  function cerrarConfirmacionAlta() {
    setPidiendoCodigoAlta(false);
    cerrarModal();
  }

  function alBorrar(fila: AdministradorPanel) {
    setBajaEnCurso(fila);
  }

  async function alConfirmarBaja() {
    if (!bajaEnCurso) return;
    try {
      await eliminarAdministrador(bajaEnCurso.correo);
      toast.success(`${bajaEnCurso.correo} ya no tiene acceso.`);
      setBajaEnCurso(null);
      recargar();
    } catch (error) {
      toast.error(mensajeError(error));
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

      {modalAbierto && !pidiendoCodigoAlta && (
        <Modal titulo="Nuevo administrador" onCerrar={cerrarModal}>
          <form onSubmit={alPedirCorreoNuevo} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <label className="campo">
              Correo de Google
              <input
                type="email"
                required
                autoFocus
                value={correoNuevo}
                placeholder="nombre@gmail.com"
                onChange={(evento) => setCorreoNuevo(evento.target.value.trim().toLowerCase())}
              />
            </label>

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button type="button" className="boton" onClick={cerrarModal}>
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario">
                Enviar código de confirmación
              </button>
            </div>
          </form>
        </Modal>
      )}

      <ConfirmacionSensible
        abierto={modalAbierto && pidiendoCodigoAlta}
        correo={sesion.correo}
        titulo="Nuevo administrador"
        pregunta={`¿Agregar a ${correoNuevo} como administrador del panel?`}
        descripcion={`confirmar que agregás a ${correoNuevo}`}
        onConfirmar={alConfirmarAlta}
        onCerrar={cerrarConfirmacionAlta}
      />

      <ConfirmacionSensible
        abierto={bajaEnCurso !== null}
        correo={sesion.correo}
        titulo={bajaEnCurso ? `Confirmar — sacarle el acceso a ${bajaEnCurso.correo}` : ""}
        pregunta={`¿Sacarle el acceso a ${bajaEnCurso?.correo}? Va a dejar de poder entrar al panel.`}
        descripcion="confirmar"
        onConfirmar={alConfirmarBaja}
        onCerrar={() => setBajaEnCurso(null)}
      />
    </div>
  );
}
