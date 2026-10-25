import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowLeft } from "lucide-react";
import { buscarEnManual } from "../../../supabase/functions/_shared/manual/buscar.ts";
import { APPS, type App, type DestinoEnlace } from "../../../supabase/functions/_shared/manual/manual.ts";
import { manual } from "../ayuda/contenido";
import Markdown, { PREFIJO_ANCLA } from "../ayuda/Markdown";
import "../ayuda/ayuda.css";

const PROCESOS = [...manual.procesos.values()];
const NOMBRE_APP = Object.fromEntries(APPS.map((app) => [app.id, app.nombre])) as Record<App, string>;
// El panel primero: es lo que usa quien entra acá.
const ORDEN_APPS: App[] = ["panel", "escritorio", "movil"];

/**
 * Manual de uso de las tres apps, el mismo de `docs/manual/` que se ve en el
 * escritorio (Ayuda, F1). El buscador recorre las tres; el índice, la que
 * esté elegida.
 */
export default function Ayuda() {
  const [app, setApp] = useState<App>("panel");
  const [consulta, setConsulta] = useState("");
  const [actual, setActual] = useState<DestinoEnlace | null>(null);
  const [historial, setHistorial] = useState<DestinoEnlace[]>([]);
  const contenido = useRef<HTMLElement>(null);

  const resultados = useMemo(() => buscarEnManual(PROCESOS, consulta), [consulta]);
  const buscando = consulta.trim().length >= 2;
  const proceso = actual ? manual.procesos.get(actual.ruta) : undefined;

  useEffect(() => {
    if (!actual) return;
    const destino = actual.ancla ? document.getElementById(PREFIJO_ANCLA + actual.ancla) : null;
    if (destino) destino.scrollIntoView({ block: "start" });
    else contenido.current?.scrollTo({ top: 0 });
  }, [actual]);

  function abrir(destino: DestinoEnlace) {
    const objetivo = manual.procesos.get(destino.ruta);
    if (!objetivo) return;
    if (actual) setHistorial((anterior) => [...anterior, actual]);
    setActual(destino);
    setApp(objetivo.app);
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
            type="search"
            placeholder="Buscar, o pegar un mensaje…"
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

        {!buscando && (
          <div className="segmentado ayuda-apps" role="group" aria-label="App">
            {ORDEN_APPS.map((id) => (
              <button key={id} type="button" aria-pressed={app === id} onClick={() => setApp(id)}>
                {NOMBRE_APP[id]}
              </button>
            ))}
          </div>
        )}

        <nav className="ayuda-lista" aria-label={buscando ? "Resultados" : "Temas de ayuda"}>
          {buscando ? (
            resultados.length === 0 ? (
              <p className="ayuda-vacio">
                No hay nada con esas palabras. Pruebe con otras más cortas (por ejemplo, «gafete»
                o «dispositivo»).
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
                  <span className="ayuda-resultado-seccion">
                    {NOMBRE_APP[resultado.proceso.app]}
                    {resultado.seccion && ` · ${resultado.seccion}`}
                  </span>
                  {resultado.fragmento && (
                    <span className="ayuda-resultado-fragmento">{resultado.fragmento}</span>
                  )}
                </button>
              ))
            )
          ) : (
            manual.indice[app].map((grupo) => (
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
            <p>
              El manual de uso del panel, del escritorio del puesto de control y de la app del
              teléfono.
            </p>
            <ul>
              <li>
                Escriba una palabra en el buscador (por ejemplo, <strong>dispositivo</strong> o{" "}
                <strong>contraseña</strong>), o pegue el mensaje que le apareció a alguien.
              </li>
              <li>O elija la app y un tema en la lista de la izquierda.</li>
            </ul>
          </div>
        )}
      </article>
    </div>
  );
}
