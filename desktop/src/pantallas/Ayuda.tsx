import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft } from "lucide-react";
import { buscarEnManual } from "../../../supabase/functions/_shared/manual/buscar.ts";
import type { DestinoEnlace } from "../../../supabase/functions/_shared/manual/manual.ts";
import { manual } from "../ayuda/contenido";
import Markdown, { PREFIJO_ANCLA } from "../ayuda/Markdown";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import "../ayuda/ayuda.css";

// En el puesto de control sólo se busca en los procesos del escritorio (el
// plan de la ayuda: no explicar acá lo del panel de administración).
const PROCESOS = [...manual.procesos.values()].filter((proceso) => proceso.app === "escritorio");
const INDICE = manual.indice.escritorio;

/**
 * Manual de uso navegable, sin internet y sin IA (etapa 2 de la ayuda, ver
 * docs/features-futuras/plan-asistente-ia.md, Parte B). El contenido es
 * `docs/manual/` tal cual: se edita ahí y viaja con el programa.
 *
 * `senalBuscar` sube con cada F1: lleva el foco al buscador.
 */
export default function Ayuda({ senalBuscar }: { senalBuscar: number }) {
  const [consulta, setConsulta] = useState("");
  const [actual, setActual] = useState<DestinoEnlace | null>(null);
  const [historial, setHistorial] = useState<DestinoEnlace[]>([]);
  const buscador = useRef<HTMLInputElement>(null);
  const contenido = useRef<HTMLElement>(null);

  const resultados = useMemo(() => buscarEnManual(PROCESOS, consulta), [consulta]);
  const buscando = consulta.trim().length >= 2;
  const proceso = actual ? manual.procesos.get(actual.ruta) : undefined;

  useBarraEstado(
    buscando ? `${resultados.length} resultado(s)` : `${PROCESOS.length} temas de ayuda`,
  );

  useEffect(() => {
    buscador.current?.focus();
    buscador.current?.select();
  }, [senalBuscar]);

  // Al abrir un tema: arriba de todo, o directo a la sección pedida.
  useEffect(() => {
    if (!actual) return;
    const destino = actual.ancla ? document.getElementById(PREFIJO_ANCLA + actual.ancla) : null;
    if (destino) destino.scrollIntoView({ block: "start" });
    else contenido.current?.scrollTo({ top: 0 });
  }, [actual]);

  function abrir(destino: DestinoEnlace) {
    if (!manual.procesos.has(destino.ruta)) return;
    if (actual) setHistorial((anterior) => [...anterior, actual]);
    setActual(destino);
  }

  function volver() {
    const anterior = historial[historial.length - 1];
    if (!anterior) return;
    setHistorial(historial.slice(0, -1));
    setActual(anterior);
  }

  return (
    <div className="pantalla-cuerpo ayuda">
      <aside className="tarjeta ayuda-lateral">
        <div className="campo">
          <input
            ref={buscador}
            type="search"
            placeholder="Buscar, o pegar el mensaje que apareció…"
            aria-label="Buscar en la ayuda"
            value={consulta}
            onChange={(evento) => setConsulta(evento.target.value)}
            onKeyDown={(evento) => {
              if (evento.key === "Enter" && resultados[0]) {
                const { proceso: primero, ancla } = resultados[0];
                abrir({ ruta: primero.ruta, ancla });
              } else if (evento.key === "Escape") {
                setConsulta("");
              }
            }}
          />
        </div>

        <nav className="ayuda-lista" aria-label={buscando ? "Resultados" : "Temas de ayuda"}>
          {buscando ? (
            resultados.length === 0 ? (
              <p className="ayuda-vacio">
                No hay nada con esas palabras. Pruebe con otras más cortas (por ejemplo, «gafete»
                o «salida»).
              </p>
            ) : (
              resultados.map((resultado) => (
                <button
                  key={resultado.proceso.ruta}
                  type="button"
                  className={`ayuda-resultado${resultado.proceso.ruta === actual?.ruta ? " activo" : ""}`}
                  onClick={() => abrir({ ruta: resultado.proceso.ruta, ancla: resultado.ancla })}
                >
                  <span className="ayuda-resultado-titulo">{resultado.proceso.titulo}</span>
                  {resultado.seccion && <span className="ayuda-resultado-seccion">{resultado.seccion}</span>}
                  {resultado.fragmento && (
                    <span className="ayuda-resultado-fragmento">{resultado.fragmento}</span>
                  )}
                </button>
              ))
            )
          ) : (
            INDICE.map((grupo) => (
              <div key={grupo.titulo} className="ayuda-grupo">
                {grupo.titulo && <h3>{grupo.titulo}</h3>}
                {grupo.procesos.map((p) => (
                  <button
                    key={p.ruta}
                    type="button"
                    className={`ayuda-tema${p.ruta === actual?.ruta ? " activo" : ""}`}
                    onClick={() => abrir({ ruta: p.ruta, ancla: null })}
                  >
                    {p.titulo}
                  </button>
                ))}
              </div>
            ))
          )}
        </nav>
      </aside>

      <article ref={contenido} className="tarjeta ayuda-proceso">
        {proceso ? (
          <>
            {historial.length > 0 && (
              <button type="button" className="boton ayuda-atras" onClick={volver}>
                <ArrowLeft size={16} aria-hidden="true" /> Atrás
              </button>
            )}
            <Markdown bloques={proceso.bloques} ruta={proceso.ruta} onEnlace={abrir} />
          </>
        ) : (
          <div className="ayuda-contenido">
            <h1>Ayuda</h1>
            <p>Aquí está el manual de uso del programa. Funciona también sin internet.</p>
            <ul>
              <li>
                Escriba una palabra en el buscador (por ejemplo, <strong>gafete</strong> o{" "}
                <strong>PRAIND</strong>), o pegue el mensaje que le apareció en pantalla.
              </li>
              <li>O elija un tema en la lista de la izquierda.</li>
              <li>
                Pulse <strong>F1</strong> desde cualquier pantalla para volver aquí.
              </li>
            </ul>
          </div>
        )}
      </article>
    </div>
  );
}
