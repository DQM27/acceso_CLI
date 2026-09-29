import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import Modal from "../componentes/Modal";
import ConfirmacionSensible from "../componentes/ConfirmacionSensible";
import CodigoVinculacionEmitido from "../componentes/CodigoVinculacionEmitido";
import { TEXTO_CREDENCIAL, TEXTO_EVENTO, tiempoRestante } from "../componentes/CodigoVinculacion.logica";
import { useAutoRefresh } from "../componentes/useAutoRefresh";
import { usePresenciaPorSitio } from "../presenciaSitios";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";
import {
  crearCodigoVinculacion,
  crearSitio,
  eliminarDispositivo,
  listarDispositivosYSitios,
  provisionarDispositivo,
  revocarDispositivo,
  suspenderDispositivo,
} from "../api/dispositivos";
import type {
  CodigoPendiente,
  Dispositivo,
  EventoSeguridad,
  TipoDispositivo,
} from "../api/dispositivos";
import type { UsuarioSesion } from "../api";

const ETIQUETAS_TIPO: Record<TipoDispositivo, string> = {
  pc: "PC",
  mobile: "Celular",
  visor: "Visor web (solo lectura)",
};

interface FilaDispositivo extends Dispositivo {
  sitio_nombre: string;
  conectado: boolean;
  /** Vencimiento del código de vinculación pendiente, si hay uno. */
  codigo_pendiente_hasta: string | null;
}

/** Código recién emitido, para mostrarlo una sola vez. */
interface CodigoMostrado {
  titulo: string;
  codigo: string;
  expira_en: string;
}

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Alta, vinculación, baja y suspensión de dispositivos, autenticada con la
 * misma sesión de Google que el resto del panel (ver `api/dispositivos.ts`).
 *
 * El panel es la única autoridad: un equipo nunca se da de alta solo. Al
 * crear un dispositivo (o con "Re-vincular") se emite un código de un solo
 * uso, con QR, que vence en minutos y se muestra UNA sola vez; el equipo lo
 * canjea y genera su propia clave (ver
 * docs/features-futuras/propuesta-registro-dispositivos.md). Los rechazos
 * (código usado, vencido, de otro dispositivo, firma inválida...) quedan en
 * "Intentos y alertas", con la IP.
 *
 * Revocar, Eliminar y Re-vincular un equipo que está en uso piden código de
 * confirmación por correo -- misma "sos vos ahora mismo" que alta/baja de
 * administradores (ver `ConfirmacionSensible`): las tres le cortan el paso
 * al equipo actual. Suspender no lo pide: es reversible con un click
 * (Reactivar).
 */
export default function Dispositivos({ sesion }: { sesion: UsuarioSesion }) {
  const [sitios, setSitios] = useState<{ id: string; nombre: string }[]>([]);
  const [dispositivos, setDispositivos] = useState<Dispositivo[]>([]);
  const [codigosPendientes, setCodigosPendientes] = useState<CodigoPendiente[]>([]);
  const [eventos, setEventos] = useState<EventoSeguridad[]>([]);
  const [codigoMostrado, setCodigoMostrado] = useState<CodigoMostrado | null>(null);
  const [cargando, setCargando] = useState(true);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);

  const [sitioId, setSitioId] = useState("");
  const [tipo, setTipo] = useState<TipoDispositivo>("pc");
  const [etiqueta, setEtiqueta] = useState("");

  const [modalSitioAbierto, setModalSitioAbierto] = useState(false);
  const [nuevoSitioNombre, setNuevoSitioNombre] = useState("");
  const [creandoSitio, setCreandoSitio] = useState(false);
  const [errorSitio, setErrorSitio] = useState<string | null>(null);

  // Modal de confirmación simple (Suspender -- reversible con un click,
  // Reactivar) -- reemplaza el confirm() nativo del navegador, que se ve
  // fuera de lugar (barra con el dominio, botones del sistema) al lado del
  // resto de la app. Mismo patrón que `confirmarSalidaMasiva` en
  // desktop/src/pantallas/Activos.tsx.
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
      toast.error(mensajeError(error));
    } finally {
      setConfirmando(false);
    }
  }

  // Revocar/Eliminar piden código de correo además de confirmar -- ver el
  // doc-comment del componente. `accion` maneja su propio error/toast (no
  // tira), para que `ConfirmacionSensible` no se quede con una excepción
  // sin atrapar entre medio de su propio manejo del código.
  const [confirmacionSensible, setConfirmacionSensible] = useState<{
    titulo: string;
    pregunta: string;
    descripcion: string;
    accion: () => Promise<void>;
  } | null>(null);

  const recargar = useCallback((opciones?: { silencioso?: boolean }) => {
    const silencioso = opciones?.silencioso ?? false;
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- evita que `react-hooks/set-state-in-effect` marque esta
    // actualización como síncrona dentro del efecto que dispara la carga.
    return Promise.resolve()
      .then(() => {
        if (!silencioso) setCargando(true);
      })
      .then(() => listarDispositivosYSitios())
      .then(({ sitios, dispositivos, codigos_pendientes, eventos }) => {
        setSitios(sitios);
        setDispositivos(dispositivos);
        setCodigosPendientes(codigos_pendientes);
        setEventos(eventos);
      })
      .catch((error) => {
        if (!silencioso) toast.error(mensajeError(error));
      })
      .finally(() => {
        if (!silencioso) setCargando(false);
      });
  }, []);

  useEffect(() => {
    recargar();
  }, [recargar]);

  // Canal Realtime sobre la tabla `dispositivos` para reflejar altas/bajas/
  // suspensiones/último uso al instante (antes sólo refrescaba con el pulso
  // de 2 minutos, o a mano) -- el intervalo queda como respaldo ante una
  // reconexión de Realtime que tarde.
  useAutoRefresh(() => recargar({ silencioso: true }), 120_000, "dispositivos");

  // Presencia en tiempo real (docs/features-futuras/plan-sesion-unica-dispositivos.md,
  // "Panel de presencia en tiempo real") -- suscripción compartida
  // (`usePresenciaPorSitio`, ver ese archivo) porque Usuarios.tsx también
  // la necesita para el mismo `sitio:{id}`, y las secciones del panel
  // quedan montadas de fondo una vez visitadas.
  const sitioIds = useMemo(() => sitios.map((s) => s.id), [sitios]);
  const presenciaPorSitio = usePresenciaPorSitio(sitioIds);
  const conectados = useMemo(() => {
    const todos = new Set<string>();
    for (const estado of Object.values(presenciaPorSitio)) {
      for (const presencias of Object.values(estado)) {
        for (const presencia of presencias as { dispositivo_id?: string }[]) {
          if (presencia.dispositivo_id) todos.add(presencia.dispositivo_id);
        }
      }
    }
    return todos;
  }, [presenciaPorSitio]);

  const nombrePorSitio = useMemo(() => {
    const mapa = new Map(sitios.map((s) => [s.id, s.nombre]));
    return (sitioId: string) => mapa.get(sitioId) ?? "?";
  }, [sitios]);

  // Los ocultos (ver alEliminar) no se muestran nunca desde acá a
  // propósito -- recuperar uno es por SQL directo en Supabase, no hay
  // botón para eso en el panel.
  const filas: FilaDispositivo[] = useMemo(() => {
    const pendientePorDispositivo = new Map(codigosPendientes.map((c) => [c.dispositivo_id, c.expira_en]));
    return dispositivos
      .filter((d) => !d.oculto_en_panel)
      .map((d) => ({
        ...d,
        sitio_nombre: nombrePorSitio(d.sitio_id),
        conectado: conectados.has(d.id),
        codigo_pendiente_hasta: pendientePorDispositivo.get(d.id) ?? null,
      }));
  }, [dispositivos, codigosPendientes, nombrePorSitio, conectados]);

  const etiquetaPorDispositivo = useMemo(() => {
    const mapa = new Map(dispositivos.map((d) => [d.id, d.etiqueta]));
    return (id: string | null) => (id ? (mapa.get(id) ?? "Dispositivo borrado") : "—");
  }, [dispositivos]);

  function abrirModal() {
    setModalAbierto(true);
    setSitioId((actual) => actual || sitios[0]?.id || "");
  }

  function cerrarModal() {
    setModalAbierto(false);
    setTipo("pc");
    setEtiqueta("");
    setErrorForm(null);
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
      const resultado = await provisionarDispositivo({
        sitio_id: sitio.id,
        tipo,
        etiqueta: etiqueta.trim(),
      });
      cerrarModal();
      setCodigoMostrado({
        titulo: `${etiqueta.trim()} (${resultado.sitio_nombre})`,
        codigo: resultado.codigo,
        expira_en: resultado.expira_en,
      });
      recargar();
    } catch (error) {
      setErrorForm(mensajeError(error));
    } finally {
      setCreando(false);
    }
  }

  function abrirModalSitio() {
    setModalSitioAbierto(true);
    setNuevoSitioNombre("");
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
      const nuevo = await crearSitio({ nombre: nuevoSitioNombre.trim() });
      setSitios((actual) => (actual.some((s) => s.id === nuevo.id) ? actual : [...actual, nuevo]));
      setSitioId(nuevo.id);
      setModalSitioAbierto(false);
    } catch (error) {
      setErrorSitio(mensajeError(error));
    } finally {
      setCreandoSitio(false);
    }
  }

  const alEliminar = useCallback((fila: FilaDispositivo) => {
    setConfirmacionSensible({
      titulo: "Borrar dispositivo",
      pregunta: `¿Borrar "${fila.etiqueta}" de la lista? Esto no se puede deshacer.`,
      descripcion: `borrar "${fila.etiqueta}" de la lista -- esto no se puede deshacer`,
      accion: async () => {
        try {
          const { borrado } = await eliminarDispositivo(fila.id);
          toast.success(
            borrado
              ? `${fila.etiqueta} borrado.`
              : `${fila.etiqueta} ya tiene historial y no se puede borrar del todo -- se ocultó de la lista.`,
          );
          setConfirmacionSensible(null);
          await recargar();
        } catch (error) {
          toast.error(mensajeError(error));
        }
      },
    });
  }, [recargar]);

  const alRevocar = useCallback((fila: FilaDispositivo) => {
    setConfirmacionSensible({
      titulo: "Revocar dispositivo",
      pregunta: `¿Revocar "${fila.etiqueta}"? Ese dispositivo va a dejar de poder sincronizar.`,
      descripcion: `revocar "${fila.etiqueta}" -- ese dispositivo va a dejar de poder sincronizar`,
      accion: async () => {
        try {
          await revocarDispositivo(fila.id);
          toast.success(`${fila.etiqueta} revocado.`);
          setConfirmacionSensible(null);
          await recargar();
        } catch (error) {
          toast.error(mensajeError(error));
        }
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
        toast.error(mensajeError(error));
      }
    },
    [recargar],
  );

  // Re-vincular: código nuevo para el MISMO dispositivo (conserva su
  // historial). Si hay un equipo usándolo, al canjearse queda fuera: por eso
  // pide confirmación por correo, igual que Revocar. Sin equipo vinculado
  // (nunca se usó, o el código anterior venció) es sólo reemplazar el código.
  const alRevincular = useCallback(
    (fila: FilaDispositivo) => {
      async function emitir() {
        const emitido = await crearCodigoVinculacion(fila.id);
        setCodigoMostrado({ titulo: fila.etiqueta, codigo: emitido.codigo, expira_en: emitido.expira_en });
        await recargar();
      }
      if (fila.credencial === "sin_vincular") {
        emitir().catch((error) => toast.error(mensajeError(error)));
        return;
      }
      setConfirmacionSensible({
        titulo: "Re-vincular dispositivo",
        pregunta: `¿Generar un código nuevo para "${fila.etiqueta}"? Cuando se use, el equipo actual deja de sincronizar.`,
        descripcion: `re-vincular "${fila.etiqueta}" -- cuando se use el código nuevo, el equipo actual deja de sincronizar`,
        accion: async () => {
          try {
            await emitir();
            setConfirmacionSensible(null);
          } catch (error) {
            toast.error(mensajeError(error));
          }
        },
      });
    },
    [recargar],
  );

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
        colId: "dispositivo_fisico",
        headerName: "Dispositivo",
        flex: 1.3,
        minWidth: 170,
        valueGetter: ({ data }) =>
          [data?.plataforma, data?.nombre_dispositivo].filter(Boolean).join(" ") || "—",
      },
      {
        field: "last_ip",
        headerName: "IP",
        flex: 0.9,
        minWidth: 120,
        valueFormatter: ({ value }) => value ?? "—",
      },
      {
        field: "credencial",
        headerName: "Vinculación",
        flex: 1.1,
        minWidth: 150,
        filter: false,
        // Un código pendiente manda sobre lo demás: es lo próximo que va a
        // pasar con este dispositivo.
        cellRenderer: ({ data }: { data: FilaDispositivo }) => {
          const restante = data.codigo_pendiente_hasta
            ? tiempoRestante(data.codigo_pendiente_hasta, Date.now())
            : null;
          const [texto, color] = restante
            ? [`Código pendiente (${restante})`, "var(--advertencia)"]
            : [
                TEXTO_CREDENCIAL[data.credencial],
                data.credencial === "clave" ? "var(--exito)" : "var(--muted)",
              ];
          return (
            <span className="chip" style={{ ["--chip-color" as string]: color }} title={data.clave_huella ?? undefined}>
              {texto}
            </span>
          );
        },
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
        field: "conectado",
        headerName: "Conexión",
        flex: 0.9,
        minWidth: 130,
        filter: false,
        // Distinto de "Estado": esto es presencia en vivo (¿tiene la app
        // abierta ahora mismo?), no si su credencial está habilitada. Un
        // dispositivo puede estar "Activo" y "Desconectado" a la vez.
        cellRenderer: ({ data }: { data: FilaDispositivo }) => {
          const [texto, color] = data.conectado
            ? ["Conectado", "var(--exito)"]
            : ["Desconectado", "var(--muted)"];
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
        flex: 2.2,
        minWidth: 340,
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
                onClick={() => alRevincular(data)}
              >
                Re-vincular
              </button>
            )}
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
    [alReactivar, alSuspender, alRevincular, alRevocar, alEliminar],
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
        <details style={{ marginTop: "0.75rem" }}>
          <summary style={{ cursor: "pointer" }}>
            Intentos y alertas ({eventos.length})
          </summary>
          {eventos.length === 0 ? (
            <p style={{ color: "var(--muted)" }}>Sin intentos rechazados ni alertas recientes.</p>
          ) : (
            <table className="tabla-eventos">
              <thead>
                <tr>
                  <th>Cuándo</th>
                  <th>Qué pasó</th>
                  <th>Dispositivo</th>
                  <th>IP</th>
                </tr>
              </thead>
              <tbody>
                {eventos.map((evento) => (
                  <tr key={evento.id}>
                    <td>{textoFechaHora(evento.ocurrido_en)}</td>
                    <td>{TEXTO_EVENTO[evento.tipo] ?? evento.tipo}</td>
                    <td>{etiquetaPorDispositivo(evento.dispositivo_id)}</td>
                    <td>{evento.ip ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </details>
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo dispositivo" onCerrar={cerrarModal}>
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
                  {creando ? "Creando…" : "Crear y generar código"}
                </button>
              </div>
            </form>
        </Modal>
      )}

      {codigoMostrado && (
        <Modal titulo={`Código de vinculación — ${codigoMostrado.titulo}`} onCerrar={() => setCodigoMostrado(null)}>
          <CodigoVinculacionEmitido
            codigo={codigoMostrado.codigo}
            expiraEn={codigoMostrado.expira_en}
            onListo={() => setCodigoMostrado(null)}
          />
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

      <ConfirmacionSensible
        abierto={confirmacionSensible !== null}
        correo={sesion.correo}
        titulo={confirmacionSensible?.titulo ?? ""}
        pregunta={confirmacionSensible?.pregunta ?? ""}
        descripcion={confirmacionSensible?.descripcion ?? ""}
        onConfirmar={() => confirmacionSensible?.accion() ?? Promise.resolve()}
        onCerrar={() => setConfirmacionSensible(null)}
      />
    </div>
  );
}
