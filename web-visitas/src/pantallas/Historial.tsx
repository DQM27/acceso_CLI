import { useState } from "react";
import { Link } from "react-router-dom";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { listarHistorial, mensajeError } from "../api";
import { tituloCita } from "../dominio";
import { useAuth } from "../contexto/AuthContexto";
import { estadoCita, rangoLegible } from "../fecha";
import Encabezado from "../componentes/Encabezado";
import { Aviso, Cargando } from "../componentes/Comunes";

/** Visitas pasadas y canceladas, de la más reciente a la más vieja. */
export default function Historial() {
  const { anfitrion } = useAuth();
  const correo = anfitrion?.correo ?? "";
  const [pagina, setPagina] = useState(0);
  const historial = useQuery({
    queryKey: ["historial", correo, pagina],
    queryFn: ({ signal }) => listarHistorial(correo, pagina, signal),
    enabled: !!correo,
    placeholderData: keepPreviousData,
  });

  return (
    <>
      <Encabezado volver="/visitas" titulo="Historial" />
      <main id="contenido" className="mx-auto flex max-w-[720px] flex-col gap-3 px-4 py-5">
        {historial.isPending && <Cargando />}
        {historial.isError && <Aviso>{mensajeError(historial.error)}</Aviso>}
        {historial.data && historial.data.citas.length === 0 && (
          <p className="tarjeta m-0 p-4 text-muted">Todavía no hay visitas pasadas.</p>
        )}
        {historial.data && historial.data.citas.length > 0 && (
          <ul className="tarjeta m-0 list-none overflow-hidden p-0">
            {historial.data.citas.map((cita, i) => {
              const estado = estadoCita(cita);
              return (
                <li key={cita.id} className={i === historial.data.citas.length - 1 ? "" : "border-b border-panel-suave"}>
                  <Link
                    to={`/visitas/${cita.id}`}
                    className="flex items-center justify-between gap-3 px-3.5 py-3 text-texto no-underline hover:bg-campo"
                  >
                    <div className="min-w-0">
                      <div className="truncate font-semibold">{tituloCita(cita)}</div>
                      <div className="text-[13px] text-muted">{rangoLegible(cita.fecha_desde, cita.fecha_hasta)}</div>
                    </div>
                    {estado === "CANCELADA" ? (
                      <span className="estado estado-cancelada">Cancelada</span>
                    ) : (
                      <span className="estado estado-salio">Pasada</span>
                    )}
                  </Link>
                </li>
              );
            })}
          </ul>
        )}
        {historial.data && (pagina > 0 || historial.data.hayMas) && (
          <div className="flex justify-between">
            <button type="button" className="boton" disabled={pagina === 0} onClick={() => setPagina((p) => p - 1)}>
              Más recientes
            </button>
            <button
              type="button"
              className="boton"
              disabled={!historial.data.hayMas}
              onClick={() => setPagina((p) => p + 1)}
            >
              Más antiguas
            </button>
          </div>
        )}
      </main>
    </>
  );
}
