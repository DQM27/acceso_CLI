import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowUp,
  CircleHelp,
  Download,
  FileSpreadsheet,
  FileText,
  MessageSquarePlus,
  Printer,
  Redo2,
  RotateCcw,
  Save,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import {
  TOPE_DIARIO_ASISTENTE,
  borrarVista,
  consultarLienzo,
  contarConsultasDeHoy,
  guardarVista,
  listarAlertas,
  listarVistasGuardadas,
  obtenerResumenHoy,
  preguntarAlAsistente,
  puedeVerAlertas,
  type TurnoChat,
} from "../api/lienzo";
import { listarUnidadesOperativas } from "../api/historial";
import { supabase } from "../lib/supabase";
import { exportarFilas, type FormatoExportacion } from "../lienzo/exportar";
import AlertasAsistente from "./AlertasAsistente";
import GuiaAsistente from "./GuiaAsistente";
import { esPedidoDeAyuda } from "../lienzo/guia";
import isotipo from "../assets/lattis-isotipo.png";
import LienzoVista from "../lienzo/Lienzo";
import ResumenDeHoy from "../lienzo/ResumenHoy";
import { describirFiltro, sinFiltro } from "../lienzo/edicion";
import { origenDe, validarLienzo, type Lienzo } from "../lienzo/esquema";
import { VISTAS_PREDEFINIDAS } from "../lienzo/vistas";
import { mensajeError } from "../mensajeError";
import "./Asistente.css";

const RESUMEN = "resumen";
/** La vista que arma el asistente cuando se le pide algo desde el resumen. */
const DEL_ASISTENTE = "asistente";
/** Intentos de manipular al asistente (sólo para la cuenta que recibe las alertas). */
const ALERTAS = "alertas-ia";
/** Prefijo de las vistas guardadas en el selector ("guardada:<id>"). */
const GUARDADA = "guardada:";
const SUGERENCIAS = [
  "Ver los PRAIND vencidos por empresa",
  "Ingresos de esta semana por sitio",
  "¿Qué PRAIND vencen el mes que viene?",
  "Empresas con más ingresos en octubre",
];
/** Turnos de la conversación que viajan con cada pedido (el resto queda sólo en pantalla). */
const TURNOS_A_ENVIAR = 12;

/** La foto del asistente en el chat: el isotipo de Lattis. */
function Avatar() {
  return <img className="asistente-avatar" src={isotipo} alt="" aria-hidden="true" />;
}

interface MensajeChat extends TurnoChat {
  id: number;
  /** Un aviso (tope, error) en vez de una respuesta del asistente. */
  aviso?: boolean;
  /** En un aviso de error: el pedido que falló, para reintentarlo sin reescribirlo. */
  reintentar?: string;
  /** Próximos pedidos que propone el asistente (botones bajo su respuesta). */
  sugerencias?: string[];
}

/** La conversación se guarda en esta pestaña: sobrevive a cambiar de pantalla, se borra al cerrarla. */
const CLAVE_CONVERSACION = "lienzo-conversacion";

function conversacionGuardada(): MensajeChat[] {
  try {
    const crudo = sessionStorage.getItem(CLAVE_CONVERSACION);
    const lista: unknown = crudo ? JSON.parse(crudo) : [];
    if (!Array.isArray(lista)) return [];
    return lista.filter(
      (m): m is MensajeChat =>
        typeof m === "object" && m !== null && typeof m.id === "number" && typeof m.texto === "string" &&
        (m.rol === "usuario" || m.rol === "asistente"),
    );
  } catch {
    return [];
  }
}

function PanelChat({
  mensajes,
  pensando,
  consultasHoy,
  hayVistaAbierta,
  onEnviar,
  onNueva,
  onReintentar,
}: {
  mensajes: MensajeChat[];
  pensando: boolean;
  /** Sobre una vista abierta, la guía suma ideas para esa vista. */
  hayVistaAbierta: boolean;
  /** Consultas de hoy de esta cuenta (null mientras no se sabe). */
  consultasHoy: number | null;
  onEnviar: (texto: string) => void;
  onNueva: () => void;
  onReintentar: (mensaje: MensajeChat) => void;
}) {
  const [texto, setTexto] = useState("");
  const [guiaAbierta, setGuiaAbierta] = useState(false);
  const pedirDesdeGuia = (pedido: string) => {
    setGuiaAbierta(false);
    onEnviar(pedido);
  };
  const fin = useRef<HTMLDivElement>(null);
  const caja = useRef<HTMLTextAreaElement>(null);
  // La caja crece con lo escrito (hasta su alto máximo en CSS) y vuelve a una línea al enviar.
  useEffect(() => {
    const el = caja.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [texto]);
  useEffect(() => {
    fin.current?.scrollIntoView?.({ behavior: "smooth", block: "end" });
  }, [mensajes.length, pensando]);

  const enviar = (e?: FormEvent) => {
    e?.preventDefault();
    const limpio = texto.trim();
    if (!limpio || pensando) return;
    setTexto("");
    // "¿Qué podés hacer?", "ayuda"…: la guía responde sola, sin gastar una consulta.
    if (esPedidoDeAyuda(limpio)) {
      setGuiaAbierta(true);
      return;
    }
    onEnviar(limpio);
  };

  return (
    <aside className="asistente-chat" aria-label="Chat del asistente">
      <header>
        <span>Asistente</span>
        {consultasHoy !== null && (
          <span
            className={`asistente-contador${consultasHoy >= TOPE_DIARIO_ASISTENTE * 0.8 ? " asistente-contador-alto" : ""}`}
            title="Consultas al asistente que usó hoy; el tope se renueva cada día"
          >
            {Math.min(consultasHoy, TOPE_DIARIO_ASISTENTE)} de {TOPE_DIARIO_ASISTENTE} hoy
          </span>
        )}
        <button
          type="button"
          className="asistente-nueva"
          onClick={() => setGuiaAbierta((a) => !a)}
          aria-pressed={guiaAbierta}
          title="¿Qué puedo pedir?"
          aria-label="¿Qué puedo pedir?"
        >
          <CircleHelp size={16} aria-hidden="true" />
        </button>
        <button
          type="button"
          className="asistente-nueva"
          onClick={() => {
            setGuiaAbierta(false);
            onNueva();
          }}
          disabled={pensando || mensajes.length === 0}
          title="Nueva conversación"
          aria-label="Nueva conversación"
        >
          <MessageSquarePlus size={16} aria-hidden="true" />
        </button>
      </header>
      {guiaAbierta ? (
        <GuiaAsistente
          hayVistaAbierta={hayVistaAbierta}
          pensando={pensando}
          onPedir={pedirDesdeGuia}
          onCerrar={() => setGuiaAbierta(false)}
        />
      ) : (
        <div className="asistente-mensajes" aria-live="polite">
          <div className="asistente-fila">
            <Avatar />
            <div className="asistente-burbuja">
              Pídame cambiar, filtrar o profundizar en cualquier dato del <b>lienzo</b>. También puede tocar una barra o una
              porción para filtrar. Para ver ideas, toque <b>?</b> arriba o escriba «¿qué podés hacer?».
            </div>
          </div>
          {mensajes.length === 0 && (
            <div className="asistente-sugerencias">
              {SUGERENCIAS.map((s) => (
                <button key={s} type="button" className="chip" onClick={() => onEnviar(s)} disabled={pensando}>
                  {s}
                </button>
              ))}
            </div>
          )}
          {mensajes.map((m) =>
            m.rol === "usuario" ? (
              <div key={m.id} className="asistente-fila asistente-fila-usuario">
                <div className="asistente-burbuja asistente-burbuja-usuario">{m.texto}</div>
              </div>
            ) : (
              <div key={m.id} className="asistente-fila">
                <Avatar />
                <div className={`asistente-burbuja${m.aviso ? " asistente-burbuja-aviso" : ""}`}>
                  {m.texto}
                  {m.reintentar && (
                    <button type="button" className="asistente-reintentar" onClick={() => onReintentar(m)} disabled={pensando}>
                      <RotateCcw size={13} aria-hidden="true" /> Reintentar
                    </button>
                  )}
                </div>
              </div>
            ),
          )}
          {/* Las sugerencias del asistente, sólo bajo su última respuesta (las viejas ya no aplican). */}
          {!pensando && mensajes.at(-1)?.sugerencias?.length ? (
            <div className="asistente-sugerencias asistente-sugerencias-respuesta" aria-label="Sugerencias del asistente">
              {mensajes.at(-1)?.sugerencias?.map((s) => (
                <button key={s} type="button" className="chip" onClick={() => onEnviar(s)}>
                  {s}
                </button>
              ))}
            </div>
          ) : null}
          {pensando && (
            <div className="asistente-fila" role="status">
              <Avatar />
              <div className="asistente-burbuja asistente-pensando" aria-label="El asistente está pensando">
                <span />
                <span />
                <span />
              </div>
            </div>
          )}
          <div ref={fin} />
        </div>
      )}
      <footer>
        <form className="asistente-entrada" onSubmit={enviar}>
          <textarea
            ref={caja}
            rows={1}
            placeholder="Escriba lo que quiere ver…"
            aria-label="Mensaje para el asistente"
            value={texto}
            maxLength={2000}
            onChange={(e) => setTexto(e.target.value)}
            onKeyDown={(e) => {
              // Enter envía; Shift+Enter hace un salto de línea (y no se envía mientras se compone un acento).
              if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault();
                enviar();
              }
            }}
          />
          <button type="submit" className="asistente-enviar" disabled={pensando || !texto.trim()} aria-label="Enviar">
            <ArrowUp size={17} strokeWidth={2.5} aria-hidden="true" />
          </button>
        </form>
        <p className="asistente-entrada-ayuda">Enter para enviar · Shift+Enter para otra línea</p>
      </footer>
    </aside>
  );
}

function Estado({ cargando, error }: { cargando: boolean; error: unknown }) {
  if (error) return <p className="asistente-error" role="alert">{mensajeError(error)}</p>;
  return cargando ? <p role="status">Cargando…</p> : null;
}

function VistaResumen({ sitioId }: { sitioId: string }) {
  const { data, isLoading, error } = useQuery({
    queryKey: ["lienzo-resumen", sitioId],
    queryFn: () => obtenerResumenHoy(sitioId || undefined),
    refetchInterval: 60_000,
    // Al cambiar de sitio se sigue viendo el anterior mientras llega el nuevo: los números se mueven, no parpadean.
    placeholderData: (previo) => previo,
  });
  return data ? <ResumenDeHoy resumen={data} /> : <Estado cargando={isLoading} error={error} />;
}

/** Menú de exportación: los datos de la vista (Excel, CSV, PDF) o la vista entera para imprimir. */
function MenuExportar({ onExportar, deshabilitado }: { onExportar: (f: FormatoExportacion | "imprimir") => void; deshabilitado: boolean }) {
  const menu = useRef<HTMLDetailsElement>(null);
  const elegir = (f: FormatoExportacion | "imprimir") => {
    if (menu.current) menu.current.open = false;
    onExportar(f);
  };
  return (
    <details ref={menu} className="asistente-menu">
      <summary className={`boton${deshabilitado ? " asistente-menu-apagado" : ""}`} aria-disabled={deshabilitado}>
        <Download size={14} aria-hidden="true" /> Exportar
      </summary>
      <div className="asistente-menu-opciones" role="menu">
        <span className="asistente-menu-grupo">Datos de la vista</span>
        <button type="button" role="menuitem" onClick={() => elegir("excel")}>
          <FileSpreadsheet size={14} aria-hidden="true" /> Excel
        </button>
        <button type="button" role="menuitem" onClick={() => elegir("csv")}>
          <FileText size={14} aria-hidden="true" /> CSV
        </button>
        <button type="button" role="menuitem" onClick={() => elegir("pdf")}>
          <FileText size={14} aria-hidden="true" /> PDF (tabla)
        </button>
        <span className="asistente-menu-grupo">La vista con sus gráficos</span>
        <button type="button" role="menuitem" onClick={() => elegir("imprimir")}>
          <Printer size={14} aria-hidden="true" /> Imprimir o guardar PDF
        </button>
      </div>
    </details>
  );
}

/** Un documento del lienzo con sus filtros como chips, Deshacer/Rehacer, Guardar y Exportar. */
function VistaDocumento({
  lienzo,
  nombre,
  puedeDeshacer,
  puedeRehacer,
  rechazo,
  verSql,
  guardada,
  onEditar,
  onDeshacer,
  onRehacer,
  onGuardar,
  onBorrar,
}: {
  lienzo: Lienzo;
  /** Cómo se llama la vista (para guardar y para los archivos exportados). */
  nombre: string;
  puedeDeshacer: boolean;
  puedeRehacer: boolean;
  rechazo: string | null;
  /** El SQL de una consulta sólo lo ve la cuenta que audita el asistente (alcance "alertas"). */
  verSql: boolean;
  /** Es una vista guardada (se puede borrar). */
  guardada: boolean;
  onEditar: (nuevo: Lienzo) => void;
  onDeshacer: () => void;
  onRehacer: () => void;
  onGuardar: (nombre: string) => Promise<void>;
  onBorrar: () => Promise<void>;
}) {
  const { data, isLoading, isFetching, error } = useQuery({
    queryKey: ["lienzo-vista", lienzo],
    queryFn: () => consultarLienzo(lienzo),
    // Mientras llega lo nuevo se sigue viendo lo anterior: así los gráficos pasan de un estado al otro.
    placeholderData: (previo) => previo,
  });
  const [guardando, setGuardando] = useState<string | null>(null);
  const [confirmarBorrado, setConfirmarBorrado] = useState(false);
  const [aviso, setAviso] = useState<{ texto: string; error?: boolean } | null>(null);
  const origen = origenDe(lienzo);
  const textoFiltros = lienzo.filtros.map((f) => describirFiltro(origen, f, lienzo.formato?.fecha)).join(" · ");

  const exportar = async (formato: FormatoExportacion | "imprimir") => {
    setAviso(null);
    if (formato === "imprimir") {
      window.print();
      return;
    }
    if (!data) return;
    try {
      const { data: sesion } = await supabase.auth.getUser();
      await exportarFilas(formato, {
        titulo: lienzo.titulo ?? nombre,
        origen,
        filas: data.filas,
        formatoFecha: lienzo.formato?.fecha,
        generadoPor: sesion.user?.email ?? "Panel",
        filtros: textoFiltros,
      });
    } catch (e) {
      setAviso({ texto: mensajeError(e), error: true });
    }
  };

  const guardar = async (e: FormEvent) => {
    e.preventDefault();
    if (!guardando?.trim()) return;
    try {
      await onGuardar(guardando.trim());
      setAviso({ texto: `Guardada como "${guardando.trim()}".` });
      setGuardando(null);
    } catch (err) {
      setAviso({ texto: mensajeError(err), error: true });
    }
  };

  return (
    <div className="asistente-documento">
      <div className="asistente-filtros" aria-label="Filtros del lienzo">
        {lienzo.filtros.map((f, i) => (
          <span key={`${f.campo}-${i}`} className="asistente-filtro">
            {describirFiltro(origen, f, lienzo.formato?.fecha)}
            <button
              type="button"
              aria-label={`Quitar filtro ${describirFiltro(origen, f, lienzo.formato?.fecha)}`}
              onClick={() => onEditar(sinFiltro(lienzo, i))}
            >
              <X size={12} aria-hidden="true" />
            </button>
          </span>
        ))}
        <span className="asistente-espacio" />
        <div className="asistente-acciones">
          <button type="button" className="boton asistente-deshacer" disabled={!puedeDeshacer} onClick={onDeshacer}>
            <Undo2 size={14} aria-hidden="true" /> Deshacer
          </button>
          <button type="button" className="boton" disabled={!puedeRehacer} onClick={onRehacer}>
            <Redo2 size={14} aria-hidden="true" /> Rehacer
          </button>
          <button type="button" className="boton" onClick={() => setGuardando(lienzo.titulo ?? nombre)}>
            <Save size={14} aria-hidden="true" /> Guardar
          </button>
          <MenuExportar onExportar={(f) => void exportar(f)} deshabilitado={!data} />
          {guardada &&
            (confirmarBorrado ? (
              <button
                type="button"
                className="boton boton-peligro"
                onClick={() => void onBorrar().catch((err) => setAviso({ texto: mensajeError(err), error: true }))}
                onBlur={() => setConfirmarBorrado(false)}
              >
                <Trash2 size={14} aria-hidden="true" /> ¿Borrar esta vista?
              </button>
            ) : (
              <button type="button" className="boton" onClick={() => setConfirmarBorrado(true)} aria-label="Borrar vista guardada">
                <Trash2 size={14} aria-hidden="true" />
              </button>
            ))}
        </div>
      </div>
      {guardando !== null && (
        <form className="asistente-guardar" onSubmit={(e) => void guardar(e)}>
          <label htmlFor="nombre-vista">Nombre de la vista</label>
          <input
            id="nombre-vista"
            value={guardando}
            maxLength={80}
            autoFocus
            onChange={(e) => setGuardando(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setGuardando(null)}
          />
          <button type="submit" className="boton boton-primario" disabled={!guardando.trim()}>
            Guardar
          </button>
          <button type="button" className="boton" onClick={() => setGuardando(null)}>
            Cancelar
          </button>
          <span className="asistente-guardar-ayuda">Si ya tiene una vista con ese nombre, se reemplaza.</span>
        </form>
      )}
      {aviso && (
        <p className={aviso.error ? "asistente-error" : "asistente-ok"} role={aviso.error ? "alert" : "status"}>
          {aviso.texto}
        </p>
      )}
      {rechazo && (
        <p className="asistente-error" role="alert">
          No se pudo aplicar ese cambio: {rechazo}
        </p>
      )}
      {data ? (
        <div className={isFetching ? "asistente-actualizando" : undefined}>
          {data.truncado && (
            <p className="asistente-aviso" role="status">
              La consulta tiene más filas de las que se muestran: se ven sólo las primeras.
            </p>
          )}
          <LienzoVista lienzo={lienzo} filas={data.filas} anteriores={data.anteriores} rango={data.rango} onEditar={onEditar} />
          {verSql && lienzo.consulta && (
            <details className="asistente-sql">
              <summary>Ver la consulta</summary>
              <pre>{lienzo.consulta.sql}</pre>
            </details>
          )}
        </div>
      ) : (
        <Estado cargando={isLoading} error={error} />
      )}
      {data && error && <Estado cargando={false} error={error} />}
    </div>
  );
}

/**
 * Asistente — Parte A del plan: el resumen de hoy y vistas del catálogo
 * dibujadas desde un documento JSON, que se editan tocando los gráficos o
 * pidiéndoselo al asistente (Edge Function `lienzo-ia`). Toda edición, venga
 * de donde venga, pasa por el validador y se puede deshacer.
 */
export default function Asistente() {
  const [vista, setVista] = useState<string>(RESUMEN);
  const [sitioId, setSitioId] = useState("");
  const { data: sitios } = useQuery({ queryKey: ["lienzo-sitios"], queryFn: listarUnidadesOperativas });

  // Historial de ediciones de cada vista (la última es la que se ve) y lo deshecho, para Rehacer.
  const [historiales, setHistoriales] = useState<Record<string, Lienzo[]>>({});
  const [rehechos, setRehechos] = useState<Record<string, Lienzo[]>>({});
  const [rechazo, setRechazo] = useState<string | null>(null);
  const [mensajes, setMensajes] = useState<MensajeChat[]>(conversacionGuardada);
  const siguienteId = useRef(mensajes.reduce((max, m) => Math.max(max, m.id + 1), 0));
  const cliente = useQueryClient();

  const { data: guardadas } = useQuery({ queryKey: ["lienzo-guardadas"], queryFn: listarVistasGuardadas });
  const vistaGuardada = vista.startsWith(GUARDADA) ? guardadas?.find((g) => GUARDADA + g.id === vista) : undefined;

  // La base de la vista: una predefinida o una guardada (validada al abrirla).
  const base = useMemo(() => {
    const v = VISTAS_PREDEFINIDAS.find((x) => x.id === vista);
    if (v) return validarLienzo(v.lienzo);
    return vistaGuardada ? validarLienzo(vistaGuardada.lienzo) : null;
  }, [vista, vistaGuardada]);
  const historial = historiales[vista] ?? [];
  const lienzo: Lienzo | null = historial.at(-1) ?? (base?.ok ? base.lienzo : null);
  const nombreVista =
    vistaGuardada?.nombre ?? VISTAS_PREDEFINIDAS.find((x) => x.id === vista)?.titulo ?? lienzo?.titulo ?? "Vista del asistente";

  const aplicar = (destino: string, nuevo: Lienzo) => {
    const r = validarLienzo(nuevo);
    if (!r.ok) {
      setRechazo(r.errores.join("; "));
      return;
    }
    setRechazo(null);
    setHistoriales((h) => ({ ...h, [destino]: [...(h[destino] ?? []), r.lienzo] }));
    // Una edición nueva descarta lo que se podía rehacer.
    setRehechos((r0) => ({ ...r0, [destino]: [] }));
  };

  const deshacer = () => {
    setRechazo(null);
    const ultimo = historial.at(-1);
    if (!ultimo) return;
    setHistoriales((h) => ({ ...h, [vista]: (h[vista] ?? []).slice(0, -1) }));
    setRehechos((r) => ({ ...r, [vista]: [...(r[vista] ?? []), ultimo] }));
  };

  const rehacer = () => {
    const siguiente = (rehechos[vista] ?? []).at(-1);
    if (!siguiente) return;
    setRehechos((r) => ({ ...r, [vista]: (r[vista] ?? []).slice(0, -1) }));
    setHistoriales((h) => ({ ...h, [vista]: [...(h[vista] ?? []), siguiente] }));
  };

  const guardar = async (nombre: string) => {
    if (!lienzo) return;
    const id = await guardarVista(nombre, lienzo);
    await cliente.invalidateQueries({ queryKey: ["lienzo-guardadas"] });
    // Queda abierta la vista guardada, sin historial: lo guardado es su base.
    setHistoriales((h) => ({ ...h, [GUARDADA + id]: [] }));
    setRehechos((r) => ({ ...r, [GUARDADA + id]: [] }));
    setVista(GUARDADA + id);
  };

  const borrar = async () => {
    if (!vistaGuardada) return;
    await borrarVista(vistaGuardada.id);
    await cliente.invalidateQueries({ queryKey: ["lienzo-guardadas"] });
    setVista(RESUMEN);
  };

  const agregarMensaje = (m: Omit<MensajeChat, "id">) =>
    setMensajes((lista) => [...lista, { ...m, id: siguienteId.current++ }]);

  const pregunta = useMutation({
    mutationFn: (pedido: Parameters<typeof preguntarAlAsistente>[0]) => preguntarAlAsistente(pedido),
    onSuccess: (respuesta) => {
      agregarMensaje({
        rol: "asistente",
        texto: respuesta.mensaje,
        sugerencias: respuesta.tipo === "editado" ? respuesta.sugerencias : undefined,
      });
      if (respuesta.tipo !== "editado") return;
      if (vista === RESUMEN || vista === ALERTAS) {
        // Desde el resumen (o las alertas), el asistente arma una vista nueva y se pasa a ella.
        setHistoriales((h) => ({ ...h, [DEL_ASISTENTE]: [] }));
        aplicar(DEL_ASISTENTE, respuesta.lienzo);
        setVista(DEL_ASISTENTE);
      } else {
        aplicar(vista, respuesta.lienzo);
      }
    },
    onError: (error, pedido) => {
      const texto = mensajeError(error);
      // Reintentar sirve si falló la red o el modelo; no si se llegó al tope del día.
      const reintentar = /tope/i.test(texto) ? undefined : pedido.pregunta;
      agregarMensaje({ rol: "asistente", texto, aviso: true, reintentar });
    },
    onSettled: () => void cliente.invalidateQueries({ queryKey: ["lienzo-uso-hoy"] }),
  });

  const pedir = (texto: string, previos: MensajeChat[]) => {
    const historialChat = previos.filter((m) => !m.aviso).slice(-TURNOS_A_ENVIAR).map(({ rol, texto: t }) => ({ rol, texto: t }));
    pregunta.mutate({ pregunta: texto, lienzo: vista === RESUMEN || vista === ALERTAS ? null : lienzo, historial: historialChat });
  };

  const enviar = (texto: string) => {
    pedir(texto, mensajes);
    agregarMensaje({ rol: "usuario", texto });
  };

  // El aviso de error se va y se vuelve a pedir lo mismo (el pedido ya está en pantalla).
  const reintentar = (aviso: MensajeChat) => {
    if (!aviso.reintentar) return;
    const previos = mensajes.filter((m) => m.id !== aviso.id);
    setMensajes(previos);
    // El historial no incluye el propio pedido que se reintenta (va como pregunta).
    const hastaElPedido = previos.slice(0, previos.findLastIndex((m) => m.rol === "usuario" && m.texto === aviso.reintentar));
    pedir(aviso.reintentar, hastaElPedido);
  };

  const nuevaConversacion = () => {
    setMensajes([]);
    siguienteId.current = 0;
  };

  // La conversación se guarda en la pestaña (se borra al cerrarla).
  useEffect(() => {
    try {
      if (mensajes.length) sessionStorage.setItem(CLAVE_CONVERSACION, JSON.stringify(mensajes));
      else sessionStorage.removeItem(CLAVE_CONVERSACION);
    } catch {
      // Sin almacenamiento (modo privado estricto): la conversación vive sólo en pantalla.
    }
  }, [mensajes]);

  const { data: consultasHoy } = useQuery({ queryKey: ["lienzo-uso-hoy"], queryFn: () => contarConsultasDeHoy(), staleTime: 30_000 });

  const hayDelAsistente = (historiales[DEL_ASISTENTE] ?? []).length > 0;
  const esResumen = vista === RESUMEN;
  const esAlertas = vista === ALERTAS;
  const { data: recibeAlertas } = useQuery({ queryKey: ["lienzo-puede-alertas"], queryFn: puedeVerAlertas, staleTime: Infinity });
  // Misma consulta que la lista de alertas: el contador y la lista se actualizan juntos.
  const { data: alertas } = useQuery({
    queryKey: ["lienzo-alertas"],
    queryFn: listarAlertas,
    enabled: recibeAlertas === true,
    refetchInterval: 60_000,
  });
  const sinRevisar = alertas?.filter((a) => !a.revisada_en).length ?? 0;

  return (
    <div className="asistente">
      <PanelChat
        mensajes={mensajes}
        pensando={pregunta.isPending}
        consultasHoy={consultasHoy ?? null}
        hayVistaAbierta={!esResumen && !esAlertas && lienzo !== null}
        onEnviar={enviar}
        onNueva={nuevaConversacion}
        onReintentar={reintentar}
      />
      <main className="asistente-lienzo">
        <div className="asistente-barra">
          <div className="campo asistente-selector">
            <select
              value={vista}
              onChange={(e) => {
                setRechazo(null);
                setVista(e.target.value);
              }}
              aria-label="Vista del lienzo"
            >
              <option value={RESUMEN}>Resumen de hoy</option>
              {hayDelAsistente && <option value={DEL_ASISTENTE}>Armada por el asistente</option>}
              {VISTAS_PREDEFINIDAS.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.titulo}
                </option>
              ))}
              {guardadas && guardadas.length > 0 && (
                <optgroup label="Mis vistas guardadas">
                  {guardadas.map((g) => (
                    <option key={g.id} value={GUARDADA + g.id}>
                      {g.nombre}
                    </option>
                  ))}
                </optgroup>
              )}
              {recibeAlertas && (
                <option value={ALERTAS}>{sinRevisar ? `Alertas del asistente (${sinRevisar})` : "Alertas del asistente"}</option>
              )}
            </select>
          </div>
          {esResumen && <span className="asistente-vivo">En vivo</span>}
          <span className="asistente-espacio" />
          {esResumen && (
            <div className="campo asistente-selector">
              <select value={sitioId} onChange={(e) => setSitioId(e.target.value)} aria-label="Sitio">
                <option value="">Todos los sitios</option>
                {(sitios ?? []).map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.nombre}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>
        {/* La `key` vuelve a montar el contenido al cambiar de vista: así los paneles entran animados. */}
        <div className="asistente-contenido" key={vista}>
          {esResumen ? (
            <VistaResumen sitioId={sitioId} />
          ) : esAlertas ? (
            <AlertasAsistente />
          ) : base && !base.ok ? (
            <p className="asistente-error" role="alert">Vista inválida: {base.errores.join("; ")}</p>
          ) : lienzo ? (
            <VistaDocumento
              lienzo={lienzo}
              nombre={nombreVista}
              verSql={recibeAlertas === true}
              guardada={Boolean(vistaGuardada)}
              puedeDeshacer={historial.length > (vista === DEL_ASISTENTE ? 1 : 0)}
              puedeRehacer={(rehechos[vista] ?? []).length > 0}
              rechazo={rechazo}
              onEditar={(nuevo) => aplicar(vista, nuevo)}
              onDeshacer={deshacer}
              onRehacer={rehacer}
              onGuardar={guardar}
              onBorrar={borrar}
            />
          ) : vista.startsWith(GUARDADA) ? (
            <p role="status">Cargando…</p>
          ) : null}
        </div>
      </main>
    </div>
  );
}
