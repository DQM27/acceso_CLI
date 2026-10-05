import { useMemo } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { listarCitasActuales, llegadasDe, mensajeError } from "../api";
import { agruparCitas, estadoVisitante, tituloCita } from "../dominio";
import type { Cita, Llegada } from "../dominio";
import { useAuth } from "../contexto/AuthContexto";
import { diaYMes, fechaLarga, horaLegible, hoyCostaRica, rangoLegible } from "../fecha";
import Encabezado from "../componentes/Encabezado";
import EstadoVisitante from "../componentes/EstadoVisitante";
import { Aviso, Cargando } from "../componentes/Comunes";

/** Pantalla principal: lo de hoy con quién ya llegó, y lo que viene. */
export default function MisVisitas() {
  const { anfitrion } = useAuth();
  const correo = anfitrion?.correo ?? "";
  const hoy = hoyCostaRica();

  const citas = useQuery({
    queryKey: ["citas-actuales", correo],
    queryFn: ({ signal }) => listarCitasActuales(correo, signal),
    enabled: !!correo,
    refetchOnWindowFocus: true,
  });
  const grupos = useMemo(() => agruparCitas(citas.data ?? [], hoy), [citas.data, hoy]);
  const idsHoy = grupos.hoy.map((c) => c.id);
  // Quién llegó: se refresca cada 30 s mientras haya visitas hoy.
  const llegadas = useQuery({
    queryKey: ["llegadas", idsHoy],
    queryFn: () => llegadasDe(idsHoy),
    enabled: idsHoy.length > 0,
    refetchInterval: 30_000,
    refetchOnWindowFocus: true,
  });
  const llegadaPorVisitante = useMemo(
    () => new Map((llegadas.data ?? []).map((l) => [l.cita_visitante_id, l])),
    [llegadas.data],
  );

  return (
    <>
      <Encabezado />
      <main id="contenido" className="mx-auto flex max-w-[720px] flex-col gap-4 px-4 py-5">
        <div className="flex items-end justify-between gap-3">
          <div>
            <div className="text-[13px] text-muted first-letter:uppercase">{fechaLarga(hoy)}</div>
            <h1 className="m-0 mt-0.5 text-xl">Mis visitas</h1>
          </div>
          <Link to="/agendar" className="boton boton-primario no-underline">
            <Plus aria-hidden="true" />
            Agendar
          </Link>
        </div>

        {citas.isPending && <Cargando />}
        {citas.isError && <Aviso>{mensajeError(citas.error)}</Aviso>}

        {citas.data && (
          <>
            <section className="flex flex-col gap-2" aria-labelledby="titulo-hoy">
              <h2 id="titulo-hoy" className="rotulo">
                Hoy
              </h2>
              {grupos.hoy.length === 0 ? (
                <p className="tarjeta m-0 p-4 text-muted">No tiene visitas para hoy.</p>
              ) : (
                grupos.hoy.map((cita) => <TarjetaHoy key={cita.id} cita={cita} llegadas={llegadaPorVisitante} />)
              )}
            </section>

            <section className="flex flex-col gap-2" aria-labelledby="titulo-proximas">
              <h2 id="titulo-proximas" className="rotulo">
                Próximas
              </h2>
              {grupos.proximas.length === 0 ? (
                <p className="tarjeta m-0 p-4 text-muted">No hay visitas agendadas para los próximos días.</p>
              ) : (
                <ul className="tarjeta m-0 list-none overflow-hidden p-0">
                  {grupos.proximas.map((cita, i) => (
                    <FilaProxima key={cita.id} cita={cita} ultima={i === grupos.proximas.length - 1} />
                  ))}
                </ul>
              )}
              <Link to="/historial" className="self-start text-[13px] font-semibold text-acento">
                Ver historial
              </Link>
            </section>
          </>
        )}
      </main>
    </>
  );
}

function sitiosDe(cita: Cita) {
  return cita.cita_sitios.map((s) => s.sitios?.nombre ?? "Lugar").join(", ");
}

function TarjetaHoy({ cita, llegadas }: { cita: Cita; llegadas: Map<string, Llegada> }) {
  const hora = horaLegible(cita.hora_estimada);
  const varios = cita.cita_sitios.length > 1;
  return (
    <article className="tarjeta relative flex flex-col gap-2.5 p-3.5">
      <div className="flex items-baseline justify-between gap-3">
        <h3 className="m-0 text-[15px]">
          {/* Toda la tarjeta abre el detalle; el enlace es el título. */}
          <Link to={`/visitas/${cita.id}`} className="text-texto no-underline after:absolute after:inset-0">
            {tituloCita(cita)}
          </Link>
        </h3>
        <span className="shrink-0 text-[13px] text-muted">
          {hora ?? (cita.fecha_desde !== cita.fecha_hasta ? rangoLegible(cita.fecha_desde, cita.fecha_hasta) : "")}
        </span>
      </div>
      <div className="text-[13px] text-muted">{sitiosDe(cita)}</div>
      <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
        {cita.cita_visitantes.map((visitante) => (
          <li key={visitante.id} className="flex items-center justify-between gap-2 rounded-chico bg-campo px-2.5 py-2">
            <span className="min-w-0 truncate">{visitante.nombre}</span>
            <EstadoVisitante estado={estadoVisitante(llegadas.get(visitante.id))} conSitio={varios} />
          </li>
        ))}
      </ul>
    </article>
  );
}

function FilaProxima({ cita, ultima }: { cita: Cita; ultima: boolean }) {
  const { dia, mes } = diaYMes(cita.fecha_desde);
  const personas = cita.cita_visitantes.length;
  const detalle =
    cita.fecha_desde === cita.fecha_hasta ? sitiosDe(cita) : rangoLegible(cita.fecha_desde, cita.fecha_hasta);
  return (
    <li className={ultima ? "" : "border-b border-panel-suave"}>
      <Link to={`/visitas/${cita.id}`} className="flex items-center gap-3 px-3.5 py-3 text-texto no-underline hover:bg-campo">
        <div className="w-11 text-center">
          <div className="text-lg font-bold leading-tight">{dia}</div>
          <div className="text-[11px] text-muted">{mes}</div>
        </div>
        <div className="min-w-0 flex-1">
          <div className="truncate font-semibold">{tituloCita(cita)}</div>
          <div className="truncate text-[13px] text-muted">
            {personas} {personas === 1 ? "persona" : "personas"} · {detalle}
          </div>
        </div>
      </Link>
    </li>
  );
}
