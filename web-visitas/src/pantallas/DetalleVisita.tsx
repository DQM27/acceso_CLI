import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { avisar } from "../avisos";
import { Copy, Pencil } from "lucide-react";
import { cancelarCita, llegadasDe, mensajeError, obtenerCita } from "../api";
import { estadoVisitante, tituloCita } from "../dominio";
import { estadoCita, fechaLegible, horaLegible, rangoLegible } from "../fecha";
import Encabezado from "../componentes/Encabezado";
import EstadoVisitante from "../componentes/EstadoVisitante";
import { Aviso, Cargando, Dialogo } from "../componentes/Comunes";

/** Una visita: quién viene y si ya llegó, cuándo, dónde; editar, duplicar y
 * cancelar. */
export default function DetalleVisita() {
  const { id = "" } = useParams();
  const navegar = useNavigate();
  const clienteConsultas = useQueryClient();
  const [confirmando, setConfirmando] = useState(false);
  const [cancelando, setCancelando] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cita = useQuery({ queryKey: ["cita", id], queryFn: ({ signal }) => obtenerCita(id, signal) });
  const llegadas = useQuery({
    queryKey: ["llegadas", [id]],
    queryFn: () => llegadasDe([id]),
    enabled: !!cita.data,
    refetchInterval: 30_000,
  });

  async function cancelar() {
    setCancelando(true);
    setError(null);
    try {
      await cancelarCita(id);
      await clienteConsultas.invalidateQueries({ queryKey: ["citas-actuales"] });
      await clienteConsultas.invalidateQueries({ queryKey: ["cita", id] });
      avisar("Visita cancelada. La portería ya no la va a dejar entrar.");
      setConfirmando(false);
      navegar("/visitas");
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      setCancelando(false);
    }
  }

  if (cita.isPending)
    return (
      <>
        <Encabezado volver="/visitas" titulo="Visita" />
        <Cargando />
      </>
    );
  if (cita.isError || !cita.data)
    return (
      <>
        <Encabezado volver="/visitas" titulo="Visita" />
        <main className="mx-auto max-w-[720px] p-4">
          <Aviso>{cita.isError ? mensajeError(cita.error) : "Esta visita no existe o no es suya."}</Aviso>
        </main>
      </>
    );

  const datos = cita.data;
  const estado = estadoCita(datos);
  const porVisitante = new Map((llegadas.data ?? []).map((l) => [l.cita_visitante_id, l]));
  const alguienEntro = (llegadas.data ?? []).length > 0;
  const varios = datos.cita_sitios.length > 1;
  const hora = horaLegible(datos.hora_estimada);

  return (
    <>
      <Encabezado volver="/visitas" titulo="Visita" />
      <main id="contenido" className="mx-auto flex max-w-[720px] flex-col gap-3.5 px-4 py-5">
        <div className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            {estado === "CANCELADA" && <span className="estado estado-cancelada">Cancelada</span>}
            {estado === "VENCIDA" && <span className="estado estado-salio">Pasada</span>}
          </div>
          <h1 className="m-0 text-xl">{tituloCita(datos)}</h1>
          <div className="text-muted">
            {rangoLegible(datos.fecha_desde, datos.fecha_hasta)}
            {hora ? ` · ${hora}` : ""}
          </div>
        </div>

        {error && <Aviso>{error}</Aviso>}

        <section className="tarjeta flex flex-col gap-2 p-3.5" aria-labelledby="titulo-personas">
          <h2 id="titulo-personas" className="rotulo">
            {datos.cita_visitantes.length === 1 ? "Persona" : `${datos.cita_visitantes.length} personas`}
          </h2>
          <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
            {datos.cita_visitantes.map((v) => (
              <li key={v.id} className="flex items-center justify-between gap-2 rounded-chico bg-campo px-2.5 py-2">
                <div className="min-w-0">
                  <div className="truncate font-semibold">{v.nombre}</div>
                  <div className="truncate text-[13px] text-muted">
                    {[v.cedula, v.empresa, v.placa_vehiculo && `placa ${v.placa_vehiculo}`].filter(Boolean).join(" · ")}
                  </div>
                </div>
                {estado !== "CANCELADA" && (
                  <EstadoVisitante estado={estadoVisitante(porVisitante.get(v.id))} conSitio={varios} />
                )}
              </li>
            ))}
          </ul>
        </section>

        <section className="tarjeta flex flex-col gap-1 p-3.5" aria-labelledby="titulo-lugar">
          <h2 id="titulo-lugar" className="rotulo">
            {varios ? "Lugares" : "Lugar"}
          </h2>
          <div>{datos.cita_sitios.map((s) => s.sitios?.nombre ?? "Lugar").join(", ")}</div>
          {/* Si hay motivo ya es el título; aquí sólo si es largo (varias líneas). */}
          {datos.motivo && datos.motivo.includes("\n") && (
            <>
              <h2 className="rotulo mt-3">Motivo</h2>
              <p className="m-0 whitespace-pre-line">{datos.motivo}</p>
            </>
          )}
          <p className="m-0 mt-3 text-[13px] text-muted">Agendada el {fechaLegible(datos.created_at.slice(0, 10))}</p>
        </section>

        <div className="flex flex-wrap gap-2">
          {estado === "VIGENTE" && !alguienEntro && (
            <Link to={`/visitas/${datos.id}/editar`} className="boton no-underline">
              <Pencil aria-hidden="true" />
              Editar
            </Link>
          )}
          <Link to={`/agendar?desde=${datos.id}`} className="boton no-underline">
            <Copy aria-hidden="true" />
            Duplicar
          </Link>
          {estado === "VIGENTE" && (
            <button type="button" className="boton boton-peligro ml-auto" onClick={() => setConfirmando(true)}>
              Cancelar visita
            </button>
          )}
        </div>
        {estado === "VIGENTE" && alguienEntro && (
          <p className="m-0 text-[13px] text-muted">
            Alguien de esta visita ya entró: ya no se puede editar. Si hace falta, duplíquela o agende otra.
          </p>
        )}
      </main>

      {confirmando && (
        <Dialogo titulo="¿Cancelar esta visita?" onCerrar={() => setConfirmando(false)} ocupado={cancelando}>
          <p className="m-0">
            La portería ya no la va a dejar entrar. Quien ya esté adentro sigue adentro.
          </p>
          <div className="flex justify-end gap-2">
            <button type="button" className="boton" disabled={cancelando} onClick={() => setConfirmando(false)}>
              Volver
            </button>
            <button type="button" className="boton boton-peligro" disabled={cancelando} onClick={cancelar}>
              {cancelando ? "Cancelando…" : "Sí, cancelar"}
            </button>
          </div>
        </Dialogo>
      )}
    </>
  );
}
