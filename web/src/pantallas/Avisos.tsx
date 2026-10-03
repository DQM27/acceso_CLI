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
  listarEquiposConNotificaciones,
} from "../api/avisos";
import type { TipoAviso } from "../api/avisos";
import { contarDestinatarios, destinoDeSeleccion, textoAlcance, textoResultado } from "./Avisos.logica";
import type { SeleccionAviso } from "./Avisos.logica";
import { mensajeError } from "../mensajeError";

const TIPO_EQUIPO: Record<string, string> = { pc: "PC", mobile: "Celular", visor: "Visor" };

interface DatosAvisos {
  sitios: Sitio[];
  equipos: Dispositivo[];
  conNotificaciones: Set<string>;
}

/** Unidades y equipos vigentes (sin los retirados ni los ocultos del panel),
 * más quiénes ya registraron su token de notificaciones. */
async function cargarDatos(): Promise<DatosAvisos> {
  const [estado, conNotificaciones] = await Promise.all([
    listarDispositivosYSitios(),
    listarEquiposConNotificaciones(),
  ]);
  return {
    sitios: [...estado.sitios].sort((a, b) => a.nombre.localeCompare(b.nombre, "es")),
    equipos: estado.dispositivos
      .filter((equipo) => !equipo.revoked_at && !equipo.oculto_en_panel)
      .sort((a, b) => a.etiqueta.localeCompare(b.etiqueta, "es")),
    conNotificaciones,
  };
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
      if (resultado.fallidos > 0 || resultado.enviados === 0) toast.warning(textoResultado(resultado));
      else toast.success(textoResultado(resultado));
      setConfirmando(false);
      limpiar();
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
                      <p className="ml-6 text-sm text-[var(--muted)]">Sin equipos vigentes.</p>
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
                          <span className="text-[var(--muted)]">({TIPO_EQUIPO[equipo.tipo] ?? equipo.tipo})</span>
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
