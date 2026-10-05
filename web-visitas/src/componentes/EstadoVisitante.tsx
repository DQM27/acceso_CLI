import type { EstadoVisitante as Estado } from "../dominio";
import { horaDeInstante } from "../fecha";

/** "Llegó 9:12 · gafete 7" / "Salió 11:40" / "Sin llegar". `conSitio` agrega
 * la unidad cuando la cita es en varias. */
export default function EstadoVisitante({ estado, conSitio = false }: { estado: Estado; conSitio?: boolean }) {
  if (estado.tipo === "adentro") {
    const gafete = estado.gafete !== null ? ` · gafete ${estado.gafete}` : "";
    const sitio = conSitio ? ` · ${estado.sitio}` : "";
    return <span className="estado estado-adentro">{`Llegó ${horaDeInstante(estado.desde)}${gafete}${sitio}`}</span>;
  }
  if (estado.tipo === "salio") {
    return <span className="estado estado-salio">{`Salió ${horaDeInstante(estado.hora)}`}</span>;
  }
  return <span className="estado estado-espera">Sin llegar</span>;
}
