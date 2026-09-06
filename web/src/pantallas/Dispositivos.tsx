import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  crearSitio,
  eliminarDispositivo,
  listarDispositivosYSitios,
  provisionarDispositivo,
  revocarDispositivo,
  suspenderDispositivo,
} from "../api/dispositivos";
import type { Dispositivo, DispositivoProvisionado, TipoDispositivo } from "../api/dispositivos";

const ETIQUETAS_TIPO: Record<TipoDispositivo, string> = {
  pc: "PC",
  mobile: "Celular",
  visor: "Visor web (solo lectura)",
};

interface FilaDispositivo extends Dispositivo {
  sitio_nombre: string;
}

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Alta/baja/suspensión de dispositivos -- reemplaza
 * `admin-panel/panel-dispositivos.html` (clave compartida, sin saber quién
 * hizo qué) por esta pantalla dentro del panel nuevo, autenticada con la
 * misma sesión de Google que el resto (ver `api/dispositivos.ts`). El
 * secreto de un dispositivo nuevo se muestra UNA sola vez al crearlo -- no
 * queda guardado en texto plano en ningún lado que se pueda volver a leer,
 * ni siquiera acá.
 */
export default function Dispositivos() {
  const [sitios, setSitios] = useState<{ id: string; nombre: string }[]>([]);
  const [dispositivos, setDispositivos] = useState<Dispositivo[]>([]);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);
  const [provisionado, setProvisionado] = useState<DispositivoProvisionado | null>(null);

  const [sitioId, setSitioId] = useState("");
  const [tipo, setTipo] = useState<TipoDispositivo>("pc");
  const [etiqueta, setEtiqueta] = useState("");

  const [modalSitioAbierto, setModalSitioAbierto] = useState(false);
  const [nuevoSitioNombre, setNuevoSitioNombre] = useState("");
  const [nuevoSitioDireccion, setNuevoSitioDireccion] = useState("");
  const [creandoSitio, setCreandoSitio] = useState(false);
  const [errorSitio, setErrorSitio] = useState<string | null>(null);

  // Modal genérico de confirmación (Revocar/Suspender/Eliminar) -- reemplaza
  // el confirm() nativo del navegador, que se ve fuera de lugar (barra con
  // el dominio, botones del sistema) al lado del resto de la app. Mismo
  // patrón que `confirmarSalidaMasiva` en desktop/src/pantallas/Activos.tsx.
  const [confirmacion, setConfirmacion] = useState<{
    titulo: string;
    mensaje: string;
    textoConfirmar: string;
    accion: () => Promise<void>;
  } | null>(null);
  const [confirmando, setConfirmando] = useState(false);

  async function ejecutarConfirmacion() {
    if (!confirmacion) return;
    setConfirmando(true);
    try {
      await confirmacion.accion();
      setConfirmacion(null);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setConfirmando(false);
    }
  }

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    if (!silencioso) setCargando(true);
    return listarDispositivosYSitios()
      .then(({ sitios, dispositivos }) => {
        setSitios(sitios);
        setDispositivos(dispositivos);
      })
      .catch((error) => {
        if (!silencioso) toast.error(String(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Cambia rara vez (alta/baja/reasignación de dispositivos) -- mismo
  // intervalo que usan desktop/mobile para su propio sync periódico.
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000);

  const nombrePorSitio = useMemo(() => {
    const mapa = new Map(sitios.map((s) => [s.id, s.nombre]));
    return (sitioId: string) => mapa.get(sitioId) ?? "?";
  }, [sitios]);

  // Los ocultos (ver alEliminar) no se muestran nunca desde acá a
  // propósito -- recuperar uno es por SQL directo en Supabase, no hay
  // botón para eso en el panel.
  const filas: FilaDispositivo[] = useMemo(
    () =>
      dispositivos
        .filter((d) => !d.oculto_en_panel)
        .map((d) => ({ ...d, sitio_nombre: nombrePorSitio(d.sitio_id) })),
    [dispositivos, nombrePorSitio],
  );

  function abrirModal() {
    setModalAbierto(true);
    setSitioId((actual) => actual || sitios[0]?.id || "");
  }

  function cerrarModal() {
    setModalAbierto(false);
    setTipo("pc");
    setEtiqueta("");
    setErrorForm(null);
    setProvisionado(null);
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    const sitio = sitios.find((s) => s.id === sitioId);
    if (!sitio) {
      setErrorForm("Elegí una unidad operativa (o creá una con el botón +).");
      return;
    }
    setCreando(true);
    setErrorForm(null);
    try {
      // Manda el nombre, no el id -- admin-provision-device resuelve el
      // sitio por nombre (upsert), así que reutiliza el que ya existe en
      // vez de duplicarlo.
      const resultado = await provisionarDispositivo({
        sitio_nombre: sitio.nombre,
        tipo,
        etiqueta: etiqueta.trim(),
      });
      setProvisionado(resultado);
      recargar();
    } catch (error) {
      setErrorForm(String(error));
    } finally {
      setCreando(false);
    }
  }

  function abrirModalSitio() {
    setModalSitioAbierto(true);
    setNuevoSitioNombre("");
    setNuevoSitioDireccion("");
    setErrorSitio(null);
  }

  function cerrarModalSitio() {
    setModalSitioAbierto(false);
  }

  async function alCrearSitio(evento: React.FormEvent) {
    evento.preventDefault();
    setCreandoSitio(true);
    setErrorSitio(null);
    try {
      const nuevo = await crearSitio({
        nombre: nuevoSitioNombre.trim(),
        direccion: nuevoSitioDireccion.trim() || undefined,
      });
      setSitios((actual) => (actual.some((s) => s.id === nuevo.id) ? actual : [...actual, nuevo]));
      setSitioId(nuevo.id);
      setModalSitioAbierto(false);
    } catch (error) {
      setErrorSitio(String(error));
    } finally {
      setCreandoSitio(false);
    }
  }

  const alEliminar = useCallback((fila: FilaDispositivo) => {
    setConfirmacion({
      titulo: "Borrar dispositivo",
      mensaje: `¿Borrar "${fila.etiqueta}" de la lista? Esto no se puede deshacer.`,
      textoConfirmar: "Borrar",
      accion: async () => {
        const { borrado } = await eliminarDispositivo(fila.id);
        toast.success(
          borrado
            ? `${fila.etiqueta} borrado.`
            : `${fila.etiqueta} ya tiene historial y no se puede borrar del todo -- se ocultó de la lista.`,
        );
        await recargar();
      },
    });
  }, [recargar]);

  const alRevocar = useCallback((fila: FilaDispositivo) => {
    setConfirmacion({
      titulo: "Revocar dispositivo",
      mensaje: `¿Revocar "${fila.etiqueta}"? Ese dispositivo va a dejar de poder sincronizar.`,
      textoConfirmar: "Revocar",
      accion: async () => {
        await revocarDispositivo(fila.id);
        toast.success(`${fila.etiqueta} revocado.`);
        await recargar();
      },
    });
  }, [recargar]);

  const alSuspender = useCallback((fila: FilaDispositivo) => {
    setConfirmacion({
      titulo: "Suspender dispositivo",
      mensaje: `¿Suspender "${fila.etiqueta}"? Va a dejar de poder sincronizar hasta que lo reactivés.`,
      textoConfirmar: "Suspender",
      accion: async () => {
        await suspenderDispositivo(fila.id, true);
        toast.success(`${fila.etiqueta} suspendido.`);
        await recargar();
      },
    });
  }, [recargar]);

  const alReactivar = useCallback(
    async (fila: FilaDispositivo) => {
      try {
        await suspenderDispositivo(fila.id, false);
        toast.success(`${fila.etiqueta} reactivado.`);
        recargar();
      } catch (error) {
        toast.error(String(error));
      }
    },
    [recargar],
  );

  async function copiarSecreto(secret: string) {
    try {
      await navigator.clipboard.writeText(secret);
      toast.success("Secreto copiado.");
    } catch {
      toast.error("No se pudo copiar -- seleccioná el texto a mano.");
    }
  }

  const columnas: ColDef<FilaDispositivo>[] = useMemo(
    () => [
      { field: "etiqueta", headerName: "Etiqueta", flex: 1.6, minWidth: 180, cellStyle: { textAlign: "left" } },
      {
        field: "tipo",
        headerName: "Tipo",
        flex: 1,
        minWidth: 150,
        valueFormatter: ({ value }) => ETIQUETAS_TIPO[value as TipoDispositivo],
      },
      { field: "sitio_nombre", headerName: "Unidad operativa", flex: 1.3, minWidth: 160 },
      {
        field: "created_at",
        headerName: "Creado",
        flex: 1.2,
        minWidth: 160,
        valueFormatter: ({ value }) => textoFechaHora(value),
      },
      {
        field: "last_seen_at",
        headerName: "Último uso",
        flex: 1.2,
        minWidth: 160,
        valueFormatter: ({ value }) => (value ? textoFechaHora(value) : "Nunca"),
      },
      {
        field: "revoked_at",
        headerName: "Estado",
        flex: 0.9,
        minWidth: 110,
        filter: false,
        cellRenderer: ({ data }: { data: FilaDispositivo }) => {
          const [texto, color] = data.revoked_at
            ? ["Revocado", "var(--error)"]
            : data.suspended_at
              ? ["Suspendido", "var(--advertencia)"]
              : ["Activo", "var(--exito)"];
          return (
            <span className="chip" style={{ ["--chip-color" as string]: color }}>
              {texto}
            </span>
          );
        },
      },
      {
        colId: "acciones",
        headerName: "Acción",
        flex: 1.8,
        minWidth: 260,
        sortable: false,
        filter: false,
        cellRenderer: ({ data }: { data: FilaDispositivo }) => (
          <div
            style={{ display: "flex", gap: "0.4rem", justifyContent: "center", alignItems: "center", height: "100%" }}
          >
            {!data.revoked_at &&
              (data.suspended_at ? (
                <button
                  type="button"
                  className="boton"
                  style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
                  onClick={() => alReactivar(data)}
                >
                  Reactivar
                </button>
              ) : (
                <button
                  type="button"
                  className="boton"
                  style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
                  onClick={() => alSuspender(data)}
                >
                  Suspender
                </button>
              ))}
            {!data.revoked_at && (
              <button
                type="button"
                className="boton"
                style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
                onClick={() => alRevocar(data)}
              >
                Revocar
              </button>
            )}
            <button
              type="button"
              className="boton"
              style={{ padding: "0.2rem 0.6rem", fontSize: "0.8rem" }}
              onClick={() => alEliminar(data)}
            >
              Eliminar
            </button>
          </div>
        ),
      },
    ],
    [alReactivar, alSuspender, alRevocar, alEliminar],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<FilaDispositivo>
            id="dispositivos"
            columnas={columnas}
            filas={filas}
            filtrosPorColumna
            controles={
              <button type="button" className="boton" onClick={abrirModal}>
                + Nuevo
              </button>
            }
          />
        </div>
        {cargando && filas.length === 0 && <p style={{ color: "var(--muted)" }}>Cargando…</p>}
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo dispositivo" onCerrar={cerrarModal}>
          {provisionado ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
              <p style={{ margin: 0 }}>
                Dispositivo creado en <strong>{provisionado.sitio_nombre}</strong>. Pegá este
                secreto en la app del dispositivo — no se va a volver a mostrar acá.
              </p>
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "stretch" }}>
                <input
                  readOnly
                  value={provisionado.secret}
                  onFocus={(evento) => evento.currentTarget.select()}
                  style={{ flex: 1, fontFamily: "monospace", fontSize: "0.8rem" }}
                />
                <button
                  type="button"
                  className="boton"
                  onClick={() => copiarSecreto(provisionado.secret)}
                >
                  Copiar
                </button>
              </div>
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
                Unidad operativa
                <div style={{ display: "flex", gap: "0.5rem" }}>
                  <select
                    required
                    autoFocus
                    style={{ flex: 1 }}
                    value={sitioId}
                    disabled={creando}
                    onChange={(evento) => setSitioId(evento.target.value)}
                  >
                    <option value="" disabled>
                      Seleccionar…
                    </option>
                    {sitios.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.nombre}
                      </option>
                    ))}
                  </select>
                  <button type="button" className="boton" disabled={creando} onClick={abrirModalSitio}>
                    + Crear
                  </button>
                </div>
              </label>

              <label className="campo">
                Tipo de dispositivo
                <select
                  value={tipo}
                  disabled={creando}
                  onChange={(evento) => setTipo(evento.target.value as TipoDispositivo)}
                >
                  <option value="pc">PC</option>
                  <option value="mobile">Celular</option>
                  <option value="visor">Visor web (solo lectura)</option>
                </select>
              </label>

              <label className="campo">
                Etiqueta
                <input
                  required
                  value={etiqueta}
                  disabled={creando}
                  placeholder="ej. Brisas - PC recepción"
                  onChange={(evento) => setEtiqueta(evento.target.value)}
                />
              </label>

              {errorForm && (
                <p className="login-error" role="alert">
                  {errorForm}
                </p>
              )}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
                <button type="button" className="boton" disabled={creando} onClick={cerrarModal}>
                  Cancelar
                </button>
                <button type="submit" className="boton boton-primario" disabled={creando}>
                  {creando ? "Creando…" : "Crear dispositivo"}
                </button>
              </div>
            </form>
          )}
        </Modal>
      )}

      {modalSitioAbierto && (
        <Modal titulo="Nueva unidad operativa" onCerrar={cerrarModalSitio}>
          <form onSubmit={alCrearSitio} style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
            <label className="campo">
              Nombre
              <input
                required
                autoFocus
                value={nuevoSitioNombre}
                disabled={creandoSitio}
                placeholder="ej. Brisas"
                onChange={(evento) => setNuevoSitioNombre(evento.target.value)}
              />
            </label>

            <label className="campo">
              Dirección (opcional)
              <input
                value={nuevoSitioDireccion}
                disabled={creandoSitio}
                placeholder="ej. San Rafael"
                onChange={(evento) => setNuevoSitioDireccion(evento.target.value)}
              />
            </label>

            {errorSitio && (
              <p className="login-error" role="alert">
                {errorSitio}
              </p>
            )}

            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button type="button" className="boton" disabled={creandoSitio} onClick={cerrarModalSitio}>
                Cancelar
              </button>
              <button type="submit" className="boton boton-primario" disabled={creandoSitio}>
                {creandoSitio ? "Creando…" : "Crear unidad operativa"}
              </button>
            </div>
          </form>
        </Modal>
      )}

      {confirmacion && (
        <Modal titulo={confirmacion.titulo} onCerrar={() => setConfirmacion(null)}>
          <p style={{ marginTop: 0 }}>{confirmacion.mensaje}</p>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              disabled={confirmando}
              onClick={() => setConfirmacion(null)}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-primario"
              disabled={confirmando}
              onClick={ejecutarConfirmacion}
            >
              {confirmando ? "Un momento…" : confirmacion.textoConfirmar}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
