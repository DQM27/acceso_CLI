import { useRef } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { avisar } from "../avisos";
import { crearCita, mensajeError, obtenerCita } from "../api";
import { formularioDesdeCita, visitanteVacio } from "../dominio";
import type { FormularioCita } from "../dominio";
import { hoyCostaRica, rangoLegible } from "../fecha";
import Encabezado from "../componentes/Encabezado";
import FormularioVisita from "../componentes/FormularioVisita";
import { Aviso, Cargando } from "../componentes/Comunes";

function vacio(): FormularioCita {
  const hoy = hoyCostaRica();
  return {
    fecha_desde: hoy,
    fecha_hasta: hoy,
    hora_estimada: "",
    motivo: "",
    sitios: [],
    visitantes: [visitanteVacio()],
  };
}

/** Agendar una visita nueva. Con `?desde=<id>` arranca con los datos de esa
 * visita (Duplicar), para hoy. */
export default function AgendarVisita() {
  const [parametros] = useSearchParams();
  const desde = parametros.get("desde");
  const origen = useQuery({
    queryKey: ["cita", desde],
    queryFn: ({ signal }) => obtenerCita(desde as string, signal),
    enabled: !!desde,
  });

  return (
    <>
      <Encabezado volver="/visitas" titulo={desde ? "Duplicar visita" : "Agendar visita"} />
      {desde && origen.isPending && <Cargando />}
      {desde && origen.isError && (
        <div className="mx-auto max-w-[720px] p-4">
          <Aviso>{mensajeError(origen.error)}</Aviso>
        </div>
      )}
      {(!desde || origen.isSuccess) && (
        <Agendar
          key={desde ?? "nueva"}
          inicial={origen.data ? { ...formularioDesdeCita(origen.data), fecha_desde: hoyCostaRica(), fecha_hasta: hoyCostaRica() } : vacio()}
        />
      )}
    </>
  );
}

function Agendar({ inicial }: { inicial: FormularioCita }) {
  const navegar = useNavigate();
  const clienteConsultas = useQueryClient();
  // El id se genera una vez: un reintento (respuesta perdida) usa el mismo y
  // la base no duplica la visita. La fecha de validación también se conserva,
  // para que cruzar medianoche no impida recuperar una visita ya guardada.
  const id = useRef(crypto.randomUUID());
  const fechaValidacion = useRef(hoyCostaRica());

  async function guardar(formulario: FormularioCita) {
    // Si falla, el formulario muestra el mensaje (`mensajeError`).
    await crearCita(id.current, formulario, fechaValidacion.current);
    await clienteConsultas.invalidateQueries({ queryKey: ["citas-actuales"] });
    await clienteConsultas.invalidateQueries({ queryKey: ["visitantes-anteriores"] });
    avisar(`Visita agendada: ${rangoLegible(formulario.fecha_desde, formulario.fecha_hasta)}`, {
      texto: "Agendar otra",
      alPulsar: () => navegar("/agendar"),
    });
    navegar("/visitas");
  }

  return <FormularioVisita inicial={inicial} textoBoton="Agendar" onGuardar={guardar} />;
}
