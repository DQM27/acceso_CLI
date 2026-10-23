import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { ArrowUp, Undo2, X } from "lucide-react";
import { consultarLienzo, obtenerResumenHoy, preguntarAlAsistente, type TurnoChat } from "../api/lienzo";
import { listarUnidadesOperativas } from "../api/historial";
import LienzoVista from "../lienzo/Lienzo";
import ResumenDeHoy from "../lienzo/ResumenHoy";
import { describirFiltro, sinFiltro } from "../lienzo/edicion";
import { validarLienzo, type Lienzo } from "../lienzo/esquema";
import { VISTAS_PREDEFINIDAS } from "../lienzo/vistas";
import { mensajeError } from "../mensajeError";
import "./Asistente.css";

const RESUMEN = "resumen";
/** La vista que arma el asistente cuando se le pide algo desde el resumen. */
const DEL_ASISTENTE = "asistente";
const SUGERENCIAS = [
  "Ver los PRAIND vencidos por empresa",
  "Ingresos de esta semana por sitio",
  "¿Qué PRAIND vencen el mes que viene?",
  "Empresas con más ingresos en octubre",
];
/** Turnos de la conversación que viajan con cada pedido (el resto queda sólo en pantalla). */
const TURNOS_A_ENVIAR = 12;

interface MensajeChat extends TurnoChat {
  id: number;
  /** Un aviso (tope, error) en vez de una respuesta del asistente. */
  aviso?: boolean;
}

function PanelChat({
  mensajes,
  pensando,
  onEnviar,
}: {
  mensajes: MensajeChat[];
  pensando: boolean;
  onEnviar: (texto: string) => void;
}) {
  const [texto, setTexto] = useState("");
  const fin = useRef<HTMLDivElement>(null);
  useEffect(() => {
    fin.current?.scrollIntoView?.({ behavior: "smooth", block: "end" });
  }, [mensajes.length, pensando]);

  const enviar = (e?: FormEvent) => {
    e?.preventDefault();
    const limpio = texto.trim();
    if (!limpio || pensando) return;
    onEnviar(limpio);
    setTexto("");
  };

  return (
    <aside className="asistente-chat" aria-label="Chat del asistente">
      <header>
        <span>Asistente</span>
        <span className="asistente-beta">Beta</span>
      </header>
      <div className="asistente-mensajes" aria-live="polite">
        <div className="asistente-fila">
          <div className="asistente-avatar" aria-hidden="true">
            IA
          </div>
          <div className="asistente-burbuja">
            Pídame cambiar, filtrar o profundizar en cualquier dato del <b>lienzo</b>. También puede tocar una barra o una
            porción para filtrar.
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
              <div className="asistente-avatar" aria-hidden="true">
                IA
              </div>
              <div className={`asistente-burbuja${m.aviso ? " asistente-burbuja-aviso" : ""}`}>{m.texto}</div>
            </div>
          ),
        )}
        {pensando && (
          <div className="asistente-fila" role="status">
            <div className="asistente-avatar" aria-hidden="true">
              IA
            </div>
            <div className="asistente-burbuja asistente-pensando" aria-label="El asistente está pensando">
              <span />
              <span />
              <span />
            </div>
          </div>
        )}
        <div ref={fin} />
      </div>
      <footer>
        <form className="asistente-entrada" onSubmit={enviar}>
          <input
            placeholder="Escriba lo que quiere ver…"
            aria-label="Mensaje para el asistente"
            value={texto}
            maxLength={2000}
            onChange={(e) => setTexto(e.target.value)}
          />
          <button type="submit" className="asistente-enviar" disabled={pensando || !texto.trim()} aria-label="Enviar">
            <ArrowUp size={16} aria-hidden="true" />
          </button>
        </form>
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

/** Un documento del lienzo con sus filtros como chips y Deshacer. */
function VistaDocumento({
  lienzo,
  puedeDeshacer,
  rechazo,
  onEditar,
  onDeshacer,
}: {
  lienzo: Lienzo;
  puedeDeshacer: boolean;
  rechazo: string | null;
  onEditar: (nuevo: Lienzo) => void;
  onDeshacer: () => void;
}) {
  const { data, isLoading, isFetching, error } = useQuery({
    queryKey: ["lienzo-vista", lienzo],
    queryFn: () => consultarLienzo(lienzo),
    // Mientras llega lo nuevo se sigue viendo lo anterior: así los gráficos pasan de un estado al otro.
    placeholderData: (previo) => previo,
  });

  return (
    <div className="asistente-documento">
      <div className="asistente-filtros" aria-label="Filtros del lienzo">
        {lienzo.filtros.map((f, i) => (
          <span key={`${f.campo}-${i}`} className="asistente-filtro">
            {describirFiltro(lienzo.fuente, f, lienzo.formato?.fecha)}
            <button
              type="button"
              aria-label={`Quitar filtro ${describirFiltro(lienzo.fuente, f, lienzo.formato?.fecha)}`}
              onClick={() => onEditar(sinFiltro(lienzo, i))}
            >
              <X size={12} aria-hidden="true" />
            </button>
          </span>
        ))}
        <span className="asistente-espacio" />
        <button type="button" className="boton asistente-deshacer" disabled={!puedeDeshacer} onClick={onDeshacer}>
          <Undo2 size={14} aria-hidden="true" /> Deshacer
        </button>
      </div>
      {rechazo && (
        <p className="asistente-error" role="alert">
          No se pudo aplicar ese cambio: {rechazo}
        </p>
      )}
      {data ? (
        <div className={isFetching ? "asistente-actualizando" : undefined}>
          <LienzoVista lienzo={lienzo} filas={data.filas} anteriores={data.anteriores} rango={data.rango} onEditar={onEditar} />
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

  // Historial de ediciones de cada vista (la última es la que se ve).
  const [historiales, setHistoriales] = useState<Record<string, Lienzo[]>>({});
  const [rechazo, setRechazo] = useState<string | null>(null);
  const [mensajes, setMensajes] = useState<MensajeChat[]>([]);
  const siguienteId = useRef(0);

  const predefinida = useMemo(() => {
    const v = VISTAS_PREDEFINIDAS.find((x) => x.id === vista);
    return v ? validarLienzo(v.lienzo) : null;
  }, [vista]);
  const historial = historiales[vista] ?? [];
  const lienzo: Lienzo | null = historial.at(-1) ?? (predefinida?.ok ? predefinida.lienzo : null);

  const aplicar = (destino: string, nuevo: Lienzo) => {
    const r = validarLienzo(nuevo);
    if (!r.ok) {
      setRechazo(r.errores.join("; "));
      return;
    }
    setRechazo(null);
    setHistoriales((h) => ({ ...h, [destino]: [...(h[destino] ?? []), r.lienzo] }));
  };

  const agregarMensaje = (m: Omit<MensajeChat, "id">) =>
    setMensajes((lista) => [...lista, { ...m, id: siguienteId.current++ }]);

  const pregunta = useMutation({
    mutationFn: (pedido: Parameters<typeof preguntarAlAsistente>[0]) => preguntarAlAsistente(pedido),
    onSuccess: (respuesta) => {
      agregarMensaje({ rol: "asistente", texto: respuesta.mensaje });
      if (respuesta.tipo !== "editado") return;
      if (vista === RESUMEN) {
        // Desde el resumen, el asistente arma una vista nueva y se pasa a ella.
        setHistoriales((h) => ({ ...h, [DEL_ASISTENTE]: [] }));
        aplicar(DEL_ASISTENTE, respuesta.lienzo);
        setVista(DEL_ASISTENTE);
      } else {
        aplicar(vista, respuesta.lienzo);
      }
    },
    onError: (error) => agregarMensaje({ rol: "asistente", texto: mensajeError(error), aviso: true }),
  });

  const enviar = (texto: string) => {
    const historialChat = mensajes.filter((m) => !m.aviso).slice(-TURNOS_A_ENVIAR).map(({ rol, texto: t }) => ({ rol, texto: t }));
    agregarMensaje({ rol: "usuario", texto });
    pregunta.mutate({ pregunta: texto, lienzo: vista === RESUMEN ? null : lienzo, historial: historialChat });
  };

  const hayDelAsistente = (historiales[DEL_ASISTENTE] ?? []).length > 0;
  const esResumen = vista === RESUMEN;

  return (
    <div className="asistente">
      <PanelChat mensajes={mensajes} pensando={pregunta.isPending} onEnviar={enviar} />
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
          ) : predefinida && !predefinida.ok ? (
            <p className="asistente-error" role="alert">Vista inválida: {predefinida.errores.join("; ")}</p>
          ) : lienzo ? (
            <VistaDocumento
              lienzo={lienzo}
              puedeDeshacer={historial.length > (vista === DEL_ASISTENTE ? 1 : 0)}
              rechazo={rechazo}
              onEditar={(nuevo) => aplicar(vista, nuevo)}
              onDeshacer={() => {
                setRechazo(null);
                setHistoriales((h) => ({ ...h, [vista]: (h[vista] ?? []).slice(0, -1) }));
              }}
            />
          ) : null}
        </div>
      </main>
    </div>
  );
}
