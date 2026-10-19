import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import { BellOff, Send, Siren } from "lucide-react";
import Modal from "../componentes/Modal";
import { useLista } from "../componentes/useLista";
import { listarDispositivosYSitios } from "../api/dispositivos";
import type { Dispositivo, Sitio } from "../api/dispositivos";
import {
  LARGO_MAXIMO_CUERPO,
  LARGO_MAXIMO_TITULO,
  enviarAviso,
  listarAvisosEnviados,
  listarEquiposConNotificaciones,
} from "../api/avisos";
import type { AvisoEnviado, TipoAviso } from "../api/avisos";
import {
  contarDestinatarios,
  destinoDeSeleccion,
  textoAccion,
  textoAlcance,
  textoDestino,
  textoEntrega,
  textoResultado,
} from "./Avisos.logica";
import type { SeleccionAviso } from "./Avisos.logica";
import { mensajeError } from "../mensajeError";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

interface DatosAvisos {
  sitios: Sitio[];
  equipos: Dispositivo[];
  conNotificaciones: Set<string>;
  /** Nombres de TODAS las unidades y equipos, retirados incluidos: el
   * historial nombra a quién se mandó cada aviso aunque ya no esté. */
  nombresSitios: Map<string, string>;
  nombresEquipos: Map<string, string>;
}

/** Unidades y celulares vigentes (sin los retirados ni los ocultos del
 * panel), más quiénes ya registraron su token de notificaciones. Las PC y los
 * visores no reciben notificaciones push: no se listan. */
async function cargarDatos(): Promise<DatosAvisos> {
  const [estado, conNotificaciones] = await Promise.all([
    listarDispositivosYSitios(),
    listarEquiposConNotificaciones(),
  ]);
  return {
    sitios: [...estado.sitios].sort((a, b) => a.nombre.localeCompare(b.nombre, "es")),
    equipos: estado.dispositivos
      .filter((equipo) => equipo.tipo === "mobile" && !equipo.revoked_at && !equipo.oculto_en_panel)
      .sort((a, b) => a.etiqueta.localeCompare(b.etiqueta, "es")),
    conNotificaciones,
    nombresSitios: new Map(estado.sitios.map((sitio) => [sitio.id, sitio.nombre])),
    nombresEquipos: new Map(estado.dispositivos.map((equipo) => [equipo.id, equipo.etiqueta])),
  };
}

/** "18/10/2026 14:32", en la hora del navegador. */
function textoFechaAviso(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

/** Historial: lo último que se mandó, con quién lo mandó, a quién y a
 * cuántos llegó. Los equipos a los que no llegó se nombran en rojo. */
function HistorialAvisos({
  avisos,
  cargando,
  nombresSitios,
  nombresEquipos,
}: {
  avisos: AvisoEnviado[] | undefined;
  cargando: boolean;
  nombresSitios: ReadonlyMap<string, string>;
  nombresEquipos: ReadonlyMap<string, string>;
}) {
  return (
    <section className="tarjeta flex flex-col gap-3 p-4">
      <h2 className="text-base font-semibold">Enviados</h2>
      {cargando && <p className="text-sm text-[var(--muted)]">Cargando historial…</p>}
      {avisos?.length === 0 && <p className="text-sm text-[var(--muted)]">Todavía no se envió ningún aviso.</p>}
      {avisos && avisos.length > 0 && (
        <ul className="flex flex-col gap-2">
          {avisos.map((aviso) => (
            <li key={aviso.id} className="flex flex-col gap-1 rounded border border-[var(--borde)] p-3">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <span className="flex items-center gap-2 font-medium">
                  {aviso.tipo === "emergente" && <Siren size={16} className="text-error" aria-label="Emergente" />}
                  {aviso.titulo}
                </span>
                <span className="text-sm text-[var(--muted)]">{textoFechaAviso(aviso.enviado_en)}</span>
              </div>
              <p className="line-clamp-2 whitespace-pre-wrap text-sm">{aviso.cuerpo}</p>
              <p className="text-sm text-[var(--muted)]">
                {textoDestino(aviso.destino, nombresSitios, nombresEquipos)} · {textoEntrega(aviso)} ·{" "}
                {aviso.enviado_por}
              </p>
              {aviso.no_recibieron.length > 0 && (
                <p className="text-sm text-error">No llegó a: {aviso.no_recibieron.join(", ")}</p>
              )}
              {aviso.avisos_acciones.map((accion) => (
                <p key={`${accion.accion}-${accion.dispositivo_etiqueta}-${accion.ocurrio_en}`} className="text-sm text-[var(--muted)]">
                  {textoAccion(accion, textoFechaAviso(accion.ocurrio_en))}
                </p>
              ))}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function alternar(conjunto: ReadonlySet<string>, id: string): Set<string> {
  const siguiente = new Set(conjunto);
  if (siguiente.has(id)) siguiente.delete(id);
  else siguiente.add(id);
  return siguiente;
}

/**
 * Notificaciones push a los teléfonos de las unidades: llegan aunque la app
 * esté cerrada (Edge Function `admin-enviar-push`). El administrador elige
 * a quién: todos, unidades completas y/o equipos sueltos. Un aviso
 * emergente suena como alarma en el teléfono (canal "Emergencias").
 *
 * Sólo reciben los equipos que ya iniciaron sesión con una versión de la
 * app que registra notificaciones; los demás se listan, deshabilitados,
 * para que se vea por qué no cuentan.
 */
export default function Avisos() {
  const { datos, cargando, recargar } = useLista(["avisos-destinatarios"], cargarDatos, {
    intervaloMs: 60_000,
    tablas: "dispositivos",
  });
  // Sin Realtime: sólo lo escribe este panel, que lo recarga al enviar.
  const historial = useLista(["avisos-enviados"], listarAvisosEnviados, { intervaloMs: 60_000, tablas: "" });
  const [titulo, setTitulo] = useState("");
  const [cuerpo, setCuerpo] = useState("");
  const [tipo, setTipo] = useState<TipoAviso>("normal");
  const [seleccion, setSeleccion] = useState<SeleccionAviso>({
    todos: true,
    sitios: new Set(),
    equipos: new Set(),
  });
  const [confirmando, setConfirmando] = useState(false);
  const [enviando, setEnviando] = useState(false);

  const equipos = useMemo(() => datos?.equipos ?? [], [datos]);
  const conNotificaciones = useMemo(() => datos?.conNotificaciones ?? new Set<string>(), [datos]);
  const destino = useMemo(() => destinoDeSeleccion(seleccion, equipos), [seleccion, equipos]);
  const alcance = useMemo(
    () => contarDestinatarios(seleccion, equipos, conNotificaciones),
    [seleccion, equipos, conNotificaciones],
  );
  const listo = titulo.trim() !== "" && cuerpo.trim() !== "" && destino !== null && alcance > 0;

  const limpiar = useCallback(() => {
    setTitulo("");
    setCuerpo("");
    setTipo("normal");
  }, []);

  async function alConfirmar() {
    if (!destino) return;
    setEnviando(true);
    try {
      const resultado = await enviarAviso({ titulo: titulo.trim(), cuerpo: cuerpo.trim(), tipo, destino });
      // Si no llegó a alguien, el aviso queda hasta que lo cierren: hay que
      // leer a quién avisarle por otro medio.
      if (resultado.fallidos > 0 || resultado.enviados === 0) {
        toast.warning(textoResultado(resultado), { duration: Infinity, closeButton: true });
      } else toast.success(textoResultado(resultado));
      setConfirmando(false);
      limpiar();
      void historial.recargar();
      // Los tokens que FCM dio por muertos se borraron: refresca el conteo.
      if (resultado.tokens_eliminados > 0) void recargar();
    } catch (fallo) {
      toast.error(mensajeError(fallo));
    } finally {
      setEnviando(false);
    }
  }

  const emergente = tipo === "emergente";

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 p-4">
        <section className="tarjeta flex flex-col gap-3 p-4">
          <h2 className="text-base font-semibold">Mensaje</h2>

          <label className="campo">
            Título
            <input
              value={titulo}
              maxLength={LARGO_MAXIMO_TITULO}
              disabled={enviando}
              placeholder="Ej.: Cierre del portón norte"
              onChange={(evento) => setTitulo(evento.target.value)}
            />
          </label>

          <label className="campo">
            <span className="flex justify-between">
              <span>Texto</span>
              <span aria-live="polite">
                {cuerpo.length}/{LARGO_MAXIMO_CUERPO}
              </span>
            </span>
            <textarea
              rows={4}
              value={cuerpo}
              maxLength={LARGO_MAXIMO_CUERPO}
              disabled={enviando}
              placeholder="Lo que verán en el teléfono"
              onChange={(evento) => setCuerpo(evento.target.value)}
            />
          </label>

          <fieldset className="flex flex-col gap-2">
            <legend className="mb-1 text-sm text-[var(--muted)]">Tipo</legend>
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="tipo-aviso"
                checked={!emergente}
                disabled={enviando}
                onChange={() => setTipo("normal")}
              />
              Normal: notificación con sonido de siempre
            </label>
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="tipo-aviso"
                checked={emergente}
                disabled={enviando}
                onChange={() => setTipo("emergente")}
              />
              <Siren size={16} className="text-error" aria-hidden="true" />
              Emergente: suena como alarma y vibra largo
            </label>
          </fieldset>
        </section>

        <section className="tarjeta flex flex-col gap-3 p-4">
          <h2 className="text-base font-semibold">Destinatarios</h2>

          <div className="flex flex-wrap gap-4">
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="destino-aviso"
                checked={seleccion.todos}
                disabled={enviando}
                onChange={() => setSeleccion((actual) => ({ ...actual, todos: true }))}
              />
              Todos los equipos
            </label>
            <label className="flex items-center gap-2">
              <input
                type="radio"
                name="destino-aviso"
                checked={!seleccion.todos}
                disabled={enviando}
                onChange={() => setSeleccion((actual) => ({ ...actual, todos: false }))}
              />
              Elegir unidades o equipos
            </label>
          </div>

          {cargando && <p className="text-sm text-[var(--muted)]">Cargando unidades…</p>}

          {!seleccion.todos && datos && (
            <div className="flex flex-col gap-3">
              {datos.sitios.map((sitio) => {
                const delSitio = equipos.filter((equipo) => equipo.sitio_id === sitio.id);
                const conPush = delSitio.filter((equipo) => conNotificaciones.has(equipo.id)).length;
                const unidadCompleta = seleccion.sitios.has(sitio.id);
                return (
                  <div key={sitio.id} className="flex flex-col gap-1 rounded border border-[var(--borde)] p-2">
                    <label className="flex items-center gap-2 font-medium">
                      <input
                        type="checkbox"
                        checked={unidadCompleta}
                        disabled={enviando}
                        onChange={() => setSeleccion((actual) => ({ ...actual, sitios: alternar(actual.sitios, sitio.id) }))}
                      />
                      {sitio.nombre}
                      <span className="text-sm font-normal text-[var(--muted)]">
                        (toda la unidad · {conPush} con notificaciones)
                      </span>
                    </label>
                    {delSitio.length === 0 && (
                      <p className="ml-6 text-sm text-[var(--muted)]">Sin celulares vigentes.</p>
                    )}
                    {delSitio.map((equipo) => {
                      const tienePush = conNotificaciones.has(equipo.id);
                      return (
                        <label
                          key={equipo.id}
                          className={`ml-6 flex items-center gap-2 text-sm ${tienePush ? "" : "text-[var(--muted)]"}`}
                          title={tienePush ? undefined : "Todavía no registró notificaciones: debe iniciar sesión con la app actualizada"}
                        >
                          <input
                            type="checkbox"
                            checked={unidadCompleta || seleccion.equipos.has(equipo.id)}
                            disabled={enviando || unidadCompleta || !tienePush}
                            onChange={() =>
                              setSeleccion((actual) => ({ ...actual, equipos: alternar(actual.equipos, equipo.id) }))
                            }
                          />
                          {equipo.etiqueta}
                          {!tienePush && <BellOff size={14} aria-label="Sin notificaciones" />}
                        </label>
                      );
                    })}
                  </div>
                );
              })}
            </div>
          )}
        </section>

        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-sm text-[var(--muted)]" aria-live="polite">
            {textoAlcance(alcance)}
          </span>
          <button
            type="button"
            className="boton boton-primario boton-icono"
            disabled={!listo || enviando}
            onClick={() => setConfirmando(true)}
          >
            <Send size={16} aria-hidden="true" />
            Enviar aviso
          </button>
        </div>

        <HistorialAvisos
          avisos={historial.datos}
          cargando={historial.cargando}
          nombresSitios={datos?.nombresSitios ?? new Map()}
          nombresEquipos={datos?.nombresEquipos ?? new Map()}
        />
      </div>

      {confirmando && (
        <Modal titulo={emergente ? "Enviar aviso emergente" : "Enviar aviso"} onCerrar={() => !enviando && setConfirmando(false)}>
          <div className="flex flex-col gap-3">
            <p>
              {textoAlcance(alcance)}
              {seleccion.todos ? " (todas las unidades)" : ""}.
            </p>
            <div className="tarjeta p-3">
              <p className="font-semibold">{titulo.trim()}</p>
              <p className="whitespace-pre-wrap text-sm">{cuerpo.trim()}</p>
            </div>
            {emergente && (
              <p className="flex items-center gap-2 text-error">
                <Siren size={16} aria-hidden="true" />
                Sonará como alarma (con el volumen de alarma) y vibrará largo en esos teléfonos.
              </p>
            )}
            <div className="flex justify-end gap-2">
              <button type="button" className="boton" disabled={enviando} onClick={() => setConfirmando(false)}>
                Cancelar
              </button>
              <button type="button" className="boton boton-primario" disabled={enviando} onClick={alConfirmar}>
                {enviando ? "Enviando…" : "Enviar"}
              </button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
