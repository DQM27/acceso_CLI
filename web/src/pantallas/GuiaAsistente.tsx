import { X } from "lucide-react";
import { BOTONES_DE_LA_VISTA, temasDeLaGuia } from "../lienzo/guia";

/**
 * "¿Qué puedo pedir?": lo que sabe hacer el asistente, por tema, con ejemplos
 * que se envían con un toque. Fija, sin llamar al modelo; se abre también al
 * escribir "¿qué podés hacer?" o "ayuda".
 */
export default function GuiaAsistente({
  hayVistaAbierta,
  pensando,
  onPedir,
  onCerrar,
}: {
  hayVistaAbierta: boolean;
  pensando: boolean;
  onPedir: (pedido: string) => void;
  onCerrar: () => void;
}) {
  return (
    <section className="asistente-guia" aria-label="Qué le puedo pedir al asistente">
      <header>
        <h2>¿Qué puedo pedir?</h2>
        <button type="button" className="asistente-nueva" onClick={onCerrar} aria-label="Volver al chat" title="Volver al chat">
          <X size={16} aria-hidden="true" />
        </button>
      </header>
      <p className="asistente-guia-intro">Escriba con sus palabras; estos son ejemplos. Toque uno para pedirlo.</p>
      {temasDeLaGuia(hayVistaAbierta).map((tema) => (
        <div key={tema.titulo} className="asistente-guia-tema">
          <h3>{tema.titulo}</h3>
          <p>{tema.explicacion}</p>
          <div className="asistente-sugerencias">
            {tema.ejemplos.map((e) => (
              <button key={e} type="button" className="chip" onClick={() => onPedir(e)} disabled={pensando}>
                {e}
              </button>
            ))}
          </div>
        </div>
      ))}
      <div className="asistente-guia-tema">
        <h3>Con los botones de la vista</h3>
        <ul>
          {BOTONES_DE_LA_VISTA.map((b) => (
            <li key={b}>{b}</li>
          ))}
        </ul>
      </div>
    </section>
  );
}
