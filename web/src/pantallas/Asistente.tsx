import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ArrowUp, Undo2, X } from "lucide-react";
import { consultarLienzo, obtenerResumenHoy } from "../api/lienzo";
import { listarUnidadesOperativas } from "../api/historial";
import LienzoVista from "../lienzo/Lienzo";
import ResumenDeHoy from "../lienzo/ResumenHoy";
import { describirFiltro, sinFiltro } from "../lienzo/edicion";
import { validarLienzo, type Lienzo } from "../lienzo/esquema";
import { VISTAS_PREDEFINIDAS } from "../lienzo/vistas";
import { mensajeError } from "../mensajeError";
import "./Asistente.css";

const RESUMEN = "resumen";
const SUGERENCIAS = [
  "Ver los PRAIND vencidos por empresa",
  "¿Quiénes están adentro ahora?",
  "Comparar dos sitios",
  "Horas por semana de un contratista",
];

/** Panel de chat de la propuesta B: maquetado pero apagado hasta la etapa 2 (modelo de IA). */
function PanelChat() {
  return (
    <aside className="asistente-chat" aria-label="Chat del asistente">
      <header>
        <span>Asistente</span>
        <span className="asistente-proximamente">Próximamente</span>
      </header>
      <div className="asistente-mensajes">
        <div className="asistente-fila">
          <div className="asistente-avatar" aria-hidden="true">
            IA
          </div>
          <div className="asistente-burbuja">
            Aquí podrá pedirme cambiar, filtrar o profundizar en cualquier dato del <b>lienzo</b>. Todavía no
            está activo: mientras tanto, elija una vista arriba y toque una barra o una porción para filtrar.
          </div>
        </div>
        <div className="asistente-sugerencias">
          {SUGERENCIAS.map((texto) => (
            <button key={texto} type="button" className="chip" disabled>
              {texto}
            </button>
          ))}
        </div>
      </div>
      <footer>
        <div className="asistente-entrada">
          <input placeholder="Escriba lo que quiere ver…" disabled aria-label="Mensaje para el asistente" />
          <button type="button" className="asistente-enviar" disabled aria-label="Enviar">
            <ArrowUp size={16} aria-hidden="true" />
          </button>
        </div>
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

/**
 * Una vista del catálogo y sus ediciones. Cada edición (hoy, el clic para
 * profundizar o quitar un filtro; en la etapa 2, lo que pida el chat) pasa por
 * el validador antes de aplicarse, y se puede deshacer.
 */
function VistaDocumento({ id }: { id: string }) {
  const vista = VISTAS_PREDEFINIDAS.find((v) => v.id === id);
  const [historial, setHistorial] = useState<Lienzo[]>([]);
  const [rechazo, setRechazo] = useState<string | null>(null);
  // Siempre por el validador, aunque el documento sea nuestro: es el mismo camino que seguirá lo que devuelva el modelo.
  const validado = useMemo(() => (vista ? validarLienzo(vista.lienzo) : null), [vista]);
  const original = validado?.ok ? validado.lienzo : null;
  const lienzo = historial.at(-1) ?? original;

  const { data, isLoading, isFetching, error } = useQuery({
    queryKey: ["lienzo-vista", lienzo],
    queryFn: () => (lienzo ? consultarLienzo(lienzo) : Promise.resolve({ filas: [] })),
    enabled: lienzo !== null,
    // Mientras llega lo nuevo se sigue viendo lo anterior: así los gráficos pasan de un estado al otro.
    placeholderData: (previo) => previo,
  });

  const editar = (nuevo: Lienzo) => {
    const r = validarLienzo(nuevo);
    if (!r.ok) {
      setRechazo(r.errores.join("; "));
      return;
    }
    setRechazo(null);
    setHistorial((h) => [...h, r.lienzo]);
  };

  if (validado && !validado.ok) {
    return <p className="asistente-error" role="alert">Vista inválida: {validado.errores.join("; ")}</p>;
  }
  if (!lienzo) return null;

  return (
    <div className="asistente-documento">
      <div className="asistente-filtros" aria-label="Filtros del lienzo">
        {lienzo.filtros.map((f, i) => (
          <span key={`${f.campo}-${i}`} className="asistente-filtro">
            {describirFiltro(lienzo.fuente, f, lienzo.formato?.fecha)}
            <button
              type="button"
              aria-label={`Quitar filtro ${describirFiltro(lienzo.fuente, f, lienzo.formato?.fecha)}`}
              onClick={() => editar(sinFiltro(lienzo, i))}
            >
              <X size={12} aria-hidden="true" />
            </button>
          </span>
        ))}
        <span className="asistente-espacio" />
        <button
          type="button"
          className="boton asistente-deshacer"
          disabled={historial.length === 0}
          onClick={() => {
            setRechazo(null);
            setHistorial((h) => h.slice(0, -1));
          }}
        >
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
          <LienzoVista lienzo={lienzo} filas={data.filas} anteriores={data.anteriores} rango={data.rango} onEditar={editar} />
        </div>
      ) : (
        <Estado cargando={isLoading} error={error} />
      )}
      {data && error && <Estado cargando={false} error={error} />}
    </div>
  );
}

/**
 * Asistente — Parte A, etapa 1 ("Lienzo sin IA"): el resumen de hoy y vistas
 * del catálogo dibujadas desde un documento JSON, que se pueden filtrar
 * tocando los gráficos. El chat está maquetado y apagado: todavía no hay modelo.
 */
export default function Asistente() {
  const [vista, setVista] = useState<string>(RESUMEN);
  const [sitioId, setSitioId] = useState("");
  const { data: sitios } = useQuery({ queryKey: ["lienzo-sitios"], queryFn: listarUnidadesOperativas });
  const esResumen = vista === RESUMEN;

  return (
    <div className="asistente">
      <PanelChat />
      <main className="asistente-lienzo">
        <div className="asistente-barra">
          <div className="campo asistente-selector">
            <select value={vista} onChange={(e) => setVista(e.target.value)} aria-label="Vista del lienzo">
              <option value={RESUMEN}>Resumen de hoy</option>
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
          {esResumen ? <VistaResumen sitioId={sitioId} /> : <VistaDocumento id={vista} />}
        </div>
      </main>
    </div>
  );
}
