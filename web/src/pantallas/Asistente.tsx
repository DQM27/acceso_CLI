import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ArrowUp } from "lucide-react";
import { consultarFuente, obtenerResumenHoy } from "../api/lienzo";
import { listarUnidadesOperativas } from "../api/historial";
import LienzoVista from "../lienzo/Lienzo";
import ResumenDeHoy from "../lienzo/ResumenHoy";
import { VISTAS_PREDEFINIDAS } from "../lienzo/vistas";
import { validarLienzo } from "../lienzo/esquema";
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
            está activo: mientras tanto, elija una vista arriba.
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
  });
  return data ? <ResumenDeHoy resumen={data} /> : <Estado cargando={isLoading} error={error} />;
}

function VistaDocumento({ id }: { id: string }) {
  const vista = VISTAS_PREDEFINIDAS.find((v) => v.id === id);
  // Siempre por el validador, aunque el documento sea nuestro: es el mismo camino que seguirá lo que devuelva el modelo.
  const validado = useMemo(() => (vista ? validarLienzo(vista.lienzo) : null), [vista]);
  const lienzo = validado?.ok ? validado.lienzo : null;
  const { data, isLoading, error } = useQuery({
    queryKey: ["lienzo-vista", id],
    queryFn: () => (lienzo ? consultarFuente(lienzo) : Promise.resolve([])),
    enabled: lienzo !== null,
  });
  if (validado && !validado.ok) {
    return <p className="asistente-error" role="alert">Vista inválida: {validado.errores.join("; ")}</p>;
  }
  return lienzo && data ? <LienzoVista lienzo={lienzo} filas={data} /> : <Estado cargando={isLoading} error={error} />;
}

/**
 * Asistente — Parte A, etapa 1 ("Lienzo sin IA"): el resumen de hoy y vistas
 * del catálogo dibujadas desde un documento JSON. El chat está maquetado y
 * apagado: todavía no hay modelo.
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
