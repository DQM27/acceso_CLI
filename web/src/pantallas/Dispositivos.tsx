import { useCallback, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import { MonitorSmartphone, ShieldAlert } from "lucide-react";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import Modal from "../componentes/Modal";
import ConfirmacionSensible from "../componentes/ConfirmacionSensible";
import CodigoVinculacionEmitido from "../componentes/CodigoVinculacionEmitido";
import { TEXTO_CREDENCIAL, TEXTO_EVENTO, detalleEvento, tiempoRestante } from "../componentes/CodigoVinculacion.logica";
import { useLista } from "../componentes/useLista";
import { usePresenciaPorSitio } from "../presenciaSitios";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { mensajeError } from "../mensajeError";
import {
  autorizarReconexion,
  crearSitio,
  eliminarDispositivo,
  listarDispositivosYSitios,
  provisionarDispositivo,
  revocarDispositivo,
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

type Vista = "dispositivos" | "eventos";

const OPCIONES_VISTA: OpcionSegmentada<Vista>[] = [
  { valor: "dispositivos", Icono: MonitorSmartphone, titulo: "Dispositivos" },
  { valor: "eventos", Icono: ShieldAlert, titulo: "Intentos y alertas" },
];

interface FilaDispositivo extends Dispositivo {
  sitio_nombre: string;
  conectado: boolean;
  /** Vencimiento del código de vinculación pendiente, si hay uno. */
  codigo_pendiente_hasta: string | null;
}

/** Código recién emitido, para mostrarlo una sola vez. */
interface CodigoMostrado {
  /** De alta (equipo nuevo) o de reconexión (equipo retirado). */
  clase: "vinculacion" | "reconexion";
  titulo: string;
  codigo: string;
  expira_en: string;
}

function textoFechaHora(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/**
 * Las acciones sobre un equipo: Registrar, Retirar y, sobre uno retirado,
 * Autorizar reconexión (ver
 * docs/features-futuras/propuesta-registro-dispositivos.md y
 * docs/features-futuras/plan-reconexion-equipo-retirado.md). Autenticada con
 * la misma sesión de Google que el resto del panel (ver `api/dispositivos.ts`).
 *
 * El panel es la única autoridad: un equipo nunca se da de alta solo. Al
 * registrarlo se emite un código de un solo uso, con QR, que vence en
 * minutos y se muestra UNA sola vez; el equipo lo canjea y genera su propia
 * clave. Un equipo reinstalado o reemplazado se registra como uno nuevo (los
 * datos vuelven con la primera sincronización) y el viejo se retira. Un
 * equipo retirado se reconecta con su propio código de reconexión: en la
 * misma unidad conserva todo; en otra, borra su base y baja la nueva. Los
 * rechazos (código usado, vencido, firma inválida...) quedan en "Intentos y
 * alertas", con la IP.
 *
 * Retirar y Eliminar piden código de confirmación por correo -- misma "sos
 * vos ahora mismo" que alta/baja de administradores (ver
 * `ConfirmacionSensible`): le cortan el paso al equipo para siempre.
 * Autorizar reconexión no: devuelve un equipo a trabajar.
 */
export default function Dispositivos({ sesion }: { sesion: UsuarioSesion }) {
  const clienteConsultas = useQueryClient();
  const [codigoMostrado, setCodigoMostrado] = useState<CodigoMostrado | null>(null);
  const [modalAbierto, setModalAbierto] = useState(false);
  const [vista, setVista] = useState<Vista>("dispositivos");
  const [creando, setCreando] = useState(false);
  const [errorForm, setErrorForm] = useState<string | null>(null);

  const [sitioId, setSitioId] = useState("");
  const [tipo, setTipo] = useState<TipoDispositivo>("pc");
  const [etiqueta, setEtiqueta] = useState("");

  const [modalSitioAbierto, setModalSitioAbierto] = useState(false);
  const [nuevoSitioNombre, setNuevoSitioNombre] = useState("");
  const [creandoSitio, setCreandoSitio] = useState(false);
  const [errorSitio, setErrorSitio] = useState<string | null>(null);

  // Autorizar la reconexión de un equipo retirado: unidad y etiqueta que va a
  // tener (por defecto, las que ya tenía).
  const [reconectando, setReconectando] = useState<FilaDispositivo | null>(null);
  const [sitioReconexion, setSitioReconexion] = useState("");
  const [etiquetaReconexion, setEtiquetaReconexion] = useState("");
  const [autorizandoReconexion, setAutorizandoReconexion] = useState(false);

  // Retirar/Eliminar piden código de correo además de confirmar -- ver el
  // doc-comment del componente. `accion` maneja su propio error/toast (no
  // tira), para que `ConfirmacionSensible` no se quede con una excepción
  // sin atrapar entre medio de su propio manejo del código.
  const [confirmacionSensible, setConfirmacionSensible] = useState<{
    titulo: string;
    pregunta: string;
    descripcion: string;
    accion: () => Promise<void>;
  } | null>(null);

  // Canal Realtime sobre la tabla `dispositivos` para reflejar altas/bajas/
  // suspensiones/último uso al instante -- el intervalo queda como respaldo
  // ante una reconexión de Realtime que tarde. Misma clave que Usuarios.tsx:
  // las dos pantallas comparten una sola petición.
  const { datos, cargando, recargar } = useLista(["dispositivos-y-sitios"], listarDispositivosYSitios, {
    intervaloMs: 120_000,
    tablas: "dispositivos",
  });
  const sitios = useMemo(() => datos?.sitios ?? [], [datos]);
  const dispositivos = useMemo<Dispositivo[]>(() => datos?.dispositivos ?? [], [datos]);
  const codigosPendientes = useMemo<CodigoPendiente[]>(() => datos?.codigos_pendientes ?? [], [datos]);
  const eventos = useMemo<EventoSeguridad[]>(() => datos?.eventos ?? [], [datos]);

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

  const columnasEventos = useMemo<ColDef<EventoSeguridad>[]>(
    () => [
      {
        colId: "cuando",
        headerName: "Cuándo",
        flex: 1,
        minWidth: 150,
        sort: "desc",
        filter: false,
        valueGetter: ({ data }) => (data ? textoFechaHora(data.ocurrido_en) : ""),
        comparator: (_a, _b, nodoA, nodoB) =>
          (nodoA.data?.ocurrido_en ?? "").localeCompare(nodoB.data?.ocurrido_en ?? ""),
      },
      {
        colId: "que_paso",
        headerName: "Qué pasó",
        flex: 1.2,
        minWidth: 170,
        valueGetter: ({ data }) => (data ? (TEXTO_EVENTO[data.tipo] ?? data.tipo) : ""),
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "detalle",
        headerName: "Detalle",
        flex: 2.2,
        minWidth: 240,
        valueGetter: ({ data }) => (data ? detalleEvento(data.tipo, data.detalle) || "—" : ""),
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "dispositivo",
        headerName: "Dispositivo",
        flex: 1.2,
        minWidth: 160,
        valueGetter: ({ data }) => (data ? etiquetaPorDispositivo(data.dispositivo_id) : ""),
        cellStyle: { textAlign: "left" },
      },
      { colId: "ip", headerName: "IP", flex: 0.9, minWidth: 130, valueGetter: ({ data }) => data?.ip ?? "—" },
    ],
    [etiquetaPorDispositivo],
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
  }

  async function alEnviarFormulario(evento: React.FormEvent) {
    evento.preventDefault();
    const sitio = sitios.find((s) => s.id === sitioId);
    if (!sitio) {
      setErrorForm("Elija una unidad operativa (o cree una con el botón +).");
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
        clase: "vinculacion",
        titulo: `${etiqueta.trim()} (${resultado.sitio_nombre})`,
        codigo: resultado.codigo,
        expira_en: resultado.expira_en,
      });
      void recargar();
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
      // La unidad nueva aparece en la lista sin esperar a otra consulta.
      clienteConsultas.setQueryData<typeof datos>(["dispositivos-y-sitios"], (actual) =>
        actual && !actual.sitios.some((s) => s.id === nuevo.id)
          ? { ...actual, sitios: [...actual.sitios, nuevo] }
          : actual,
      );
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
      pregunta: `¿Borrar "${fila.etiqueta}" de la lista? Queda retirado y deja de aparecer; su historial se conserva.`,
      descripcion: `borrar "${fila.etiqueta}" de la lista -- queda retirado y deja de aparecer`,
      accion: async () => {
        try {
          await eliminarDispositivo(fila.id);
          toast.success(`${fila.etiqueta} retirado y quitado de la lista.`);
          setConfirmacionSensible(null);
          await recargar();
        } catch (error) {
          toast.error(mensajeError(error));
        }
      },
    });
  }, [recargar]);

  const alRetirar = useCallback((fila: FilaDispositivo) => {
    setConfirmacionSensible({
      titulo: "Retirar dispositivo",
      pregunta: `¿Retirar "${fila.etiqueta}"? Ese equipo deja de sincronizar en el acto. Para volver a usarlo hay que autorizar su reconexión.`,
      descripcion: `retirar "${fila.etiqueta}" -- ese equipo deja de sincronizar en el acto`,
      accion: async () => {
        try {
          await revocarDispositivo(fila.id);
          toast.success(`${fila.etiqueta} retirado.`);
          setConfirmacionSensible(null);
          await recargar();
        } catch (error) {
          toast.error(mensajeError(error));
        }
      },
    });
  }, [recargar]);

  const alAutorizarReconexion = useCallback((fila: FilaDispositivo) => {
    setReconectando(fila);
    setSitioReconexion(fila.sitio_id);
    setEtiquetaReconexion(fila.etiqueta);
  }, []);

  // Sin código por correo (pedido del dueño, 2026-10-26): retirar y borrar
  // le cortan el paso a un equipo y siguen con `ConfirmacionSensible`, pero
  // reconectar lo devuelve a trabajar -- el formulario es la confirmación,
  // con el aviso a la vista si cambia de unidad. El servidor igual exige
  // sesión de administrador del panel.
  async function alEnviarReconexion(evento: React.FormEvent) {
    evento.preventDefault();
    const fila = reconectando;
    const sitio = sitios.find((s) => s.id === sitioReconexion);
    const nuevaEtiqueta = etiquetaReconexion.trim();
    if (!fila || !sitio || !nuevaEtiqueta || autorizandoReconexion) return;
    setAutorizandoReconexion(true);
    try {
      const resultado = await autorizarReconexion({
        dispositivo_id: fila.id,
        sitio_id: sitio.id,
        etiqueta: nuevaEtiqueta,
      });
      setReconectando(null);
      setCodigoMostrado({
        clase: "reconexion",
        titulo: `${nuevaEtiqueta} (${resultado.sitio_nombre})`,
        codigo: resultado.codigo,
        expira_en: resultado.expira_en,
      });
      await recargar();
    } catch (error) {
      toast.error(mensajeError(error));
    } finally {
      setAutorizandoReconexion(false);
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
            ? ["Retirado", "var(--error)"]
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
        flex: 1.2,
        minWidth: 190,
        sortable: false,
        filter: false,
        cellRenderer: ({ data }: { data: FilaDispositivo }) => (
          <div className="flex h-full items-center justify-center gap-[0.4rem]">
            {data.revoked_at ? (
              <button
                type="button"
                className="boton boton-celda"
                onClick={() => alAutorizarReconexion(data)}
              >
                Autorizar reconexión
              </button>
            ) : (
              <button
                type="button"
                className="boton boton-celda"
                onClick={() => alRetirar(data)}
              >
                Retirar
              </button>
            )}
            <button
              type="button"
              className="boton boton-celda"
              onClick={() => alEliminar(data)}
            >
              Eliminar
            </button>
          </div>
        ),
      },
    ],
    [alRetirar, alEliminar, alAutorizarReconexion],
  );

  const selectorVista = (
    <SegmentadoOpciones opciones={OPCIONES_VISTA} valor={vista} onCambiar={setVista} etiqueta="Vista" />
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          {/* Cada vista con su `key`: sin ella React reusa la misma grilla y por
              un instante pinta las filas de una con las columnas de la otra. */}
          {vista === "dispositivos" ? (
            <Tabla<FilaDispositivo>
              key="dispositivos"
              id="dispositivos"
              columnas={columnas}
              filas={filas}
              filtrosPorColumna
              controles={
                <>
                  {selectorVista}
                  <button type="button" className="boton" onClick={abrirModal}>
                    + Nuevo
                  </button>
                </>
              }
            />
          ) : (
            <Tabla<EventoSeguridad>
              key="eventos"
              id="dispositivos-eventos"
              columnas={columnasEventos}
              filas={eventos}
              filtrosPorColumna
              idFila={(evento) => String(evento.id)}
              controles={
                <>
                  {selectorVista}
                  <span role="status">
                    {eventos.length === 0
                      ? "Sin intentos rechazados ni alertas recientes"
                      : `${eventos.length.toLocaleString("es-CR")} intentos y alertas`}
                  </span>
                </>
              }
            />
          )}
        </div>
        {cargando && filas.length === 0 && <p className="text-muted">Cargando…</p>}
      </div>

      {modalAbierto && (
        <Modal titulo="Nuevo dispositivo" onCerrar={cerrarModal}>
            <form onSubmit={alEnviarFormulario} className="flex flex-col gap-4">
              <label className="campo">
                Unidad operativa
                <div className="flex gap-2">
                  <select
                    required
                    autoFocus
                    className="flex-1"
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

              <div className="flex justify-end gap-2">
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
        <Modal
          titulo={`${codigoMostrado.clase === "reconexion" ? "Código de reconexión" : "Código de vinculación"} — ${codigoMostrado.titulo}`}
          onCerrar={() => setCodigoMostrado(null)}
        >
          <CodigoVinculacionEmitido
            codigo={codigoMostrado.codigo}
            expiraEn={codigoMostrado.expira_en}
            onListo={() => setCodigoMostrado(null)}
          />
        </Modal>
      )}

      {reconectando && (
        <Modal titulo={`Autorizar reconexión — ${reconectando.etiqueta}`} onCerrar={() => setReconectando(null)}>
          <form onSubmit={alEnviarReconexion} className="flex flex-col gap-4">
            <p className="text-muted" style={{ margin: 0 }}>
              El equipo recibe un código de reconexión de un solo uso, que sólo sirve para él. En la
              misma unidad conserva todos sus datos; en otra, borra los de su unidad anterior y baja
              los de la nueva.
            </p>
            <label className="campo">
              Unidad operativa
              <select
                required
                autoFocus
                value={sitioReconexion}
                onChange={(evento) => setSitioReconexion(evento.target.value)}
              >
                {sitios.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.nombre}
                    {s.id === reconectando.sitio_id ? " (la misma)" : ""}
                  </option>
                ))}
              </select>
            </label>
            <label className="campo">
              Etiqueta
              <input
                required
                value={etiquetaReconexion}
                onChange={(evento) => setEtiquetaReconexion(evento.target.value)}
              />
            </label>
            {sitioReconexion !== reconectando.sitio_id && (
              <p className="login-error" role="alert" style={{ margin: 0 }}>
                Va a otra unidad: al reconectarse, el equipo borra los datos de su unidad anterior.
              </p>
            )}
            <div className="flex justify-end gap-2">
              <button
                type="button"
                className="boton"
                disabled={autorizandoReconexion}
                onClick={() => setReconectando(null)}
              >
                Cancelar
              </button>
              <button
                type="submit"
                className="boton boton-primario"
                disabled={!etiquetaReconexion.trim() || autorizandoReconexion}
              >
                {autorizandoReconexion ? "Generando…" : "Generar código de reconexión"}
              </button>
            </div>
          </form>
        </Modal>
      )}

      {modalSitioAbierto && (
        <Modal titulo="Nueva unidad operativa" onCerrar={cerrarModalSitio}>
          <form onSubmit={alCrearSitio} className="flex flex-col gap-3">
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

            <div className="flex justify-end gap-2">
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
