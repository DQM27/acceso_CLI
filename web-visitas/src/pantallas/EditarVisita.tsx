import { useRef } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { avisar } from "../avisos";
import { editarCita, mensajeError, obtenerCita } from "../api";
import { formularioDesdeCita } from "../dominio";
import type { FormularioCita } from "../dominio";
import { hoyCostaRica } from "../fecha";
import Encabezado from "../componentes/Encabezado";
import FormularioVisita from "../componentes/FormularioVisita";
import { Aviso, Cargando } from "../componentes/Comunes";

/** Editar una visita. La base la cancela y crea la nueva en un solo paso
 * (así el cambio llega a todas las porterías), y sólo si nadie entró. */
export default function EditarVisita() {
  const { id = "" } = useParams();
  const navegar = useNavigate();
  const clienteConsultas = useQueryClient();
  const nuevoId = useRef(crypto.randomUUID());
  const fechaValidacion = useRef(hoyCostaRica());
  const cita = useQuery({ queryKey: ["cita", id], queryFn: ({ signal }) => obtenerCita(id, signal) });

  async function guardar(formulario: FormularioCita) {
    const nueva = await editarCita(id, nuevoId.current, formulario, fechaValidacion.current);
    await clienteConsultas.invalidateQueries({ queryKey: ["citas-actuales"] });
    await clienteConsultas.invalidateQueries({ queryKey: ["visitantes-anteriores"] });
    avisar("Visita actualizada.");
    navegar(`/visitas/${nueva}`, { replace: true });
  }

  return (
    <>
      <Encabezado volver={`/visitas/${id}`} titulo="Editar visita" />
      {cita.isPending && <Cargando />}
      {(cita.isError || (cita.isSuccess && !cita.data)) && (
        <div className="mx-auto max-w-[720px] p-4">
          <Aviso>{cita.isError ? mensajeError(cita.error) : "Esta visita no existe o no es suya."}</Aviso>
        </div>
      )}
      {cita.data && cita.data.estado === "CANCELADA" && (
        <div className="mx-auto max-w-[720px] p-4">
          <Aviso>Esta visita está cancelada: no se puede editar.</Aviso>
        </div>
      )}
      {cita.data && cita.data.estado === "VIGENTE" && (
        <FormularioVisita inicial={formularioDesdeCita(cita.data)} textoBoton="Guardar cambios" onGuardar={guardar} />
      )}
    </>
  );
}
