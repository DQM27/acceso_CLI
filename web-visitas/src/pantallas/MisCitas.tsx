import { useEffect, useRef, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import {
  CalendarDays,
  CalendarRange,
  ChevronLeft,
  ChevronRight,
  CirclePlus,
  List,
  MapPin,
  RefreshCw,
  Users,
  X,
} from "lucide-react";
import { cancelarCita, listarCitas, listarCitasCalendario, mensajeError } from "../api";
import type { Cita, FiltroEstado } from "../dominio";
import { estadoCita, fechaLegible } from "../fecha";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, Cargando, Modal } from "../componentes/Comunes";
import CitasCalendario from "../componentes/CitasCalendario";

const FILTROS: { valor: FiltroEstado; nombre: string }[] = [
  { valor: "TODAS", nombre: "Todas" },
  { valor: "VIGENTE", nombre: "Vigentes" },
  { valor: "VENCIDA", nombre: "Vencidas" },
  { valor: "CANCELADA", nombre: "Canceladas" },
];
const etiquetaEstado = {
  VIGENTE: "Vigente",
  CANCELADA: "Cancelada",
  VENCIDA: "Vencida",
};

export default function MisCitas() {
  const { anfitrion, verificado } = useAuth();
  const [filtro, setFiltro] = useState<FiltroEstado>("TODAS");
  const [pagina, setPagina] = useState(0);
  const [citas, setCitas] = useState<Cita[]>([]);
  const [hayMas, setHayMas] = useState(false);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const [detalle, setDetalle] = useState<Cita | null>(null);
  const [cancelacion, setCancelacion] = useState<Cita | null>(null);
  const [cancelando, setCancelando] = useState(false);
  const [errorCancelar, setErrorCancelar] = useState<string | null>(null);
  const [aviso, setAviso] = useState<string | null>(null);
  const [vista, setVista] = useState<"lista" | "calendario">("lista");
  const [citasCalendario, setCitasCalendario] = useState<Cita[]>([]);
  const [cargandoCalendario, setCargandoCalendario] = useState(true);
  const [errorCalendario, setErrorCalendario] = useState<string | null>(null);
  const bloqueo = useRef(false);
  const ruta = useLocation();
  const navegar = useNavigate();

  useEffect(() => {
    if (ruta.state?.creada) {
      setAviso(
        "Cita agendada. Recordales a tus visitantes traer su documento de identidad.",
      );
      navegar("/citas", { replace: true, state: null });
    }
  }, [ruta.state, navegar]);

  useEffect(() => {
    const controlador = new AbortController();
    setCargando(true);
    setError(null);
    listarCitas(anfitrion!.correo, filtro, pagina, controlador.signal)
      .then((resultado) => {
        if (controlador.signal.aborted) return;
        setCitas(resultado.citas);
        setHayMas(resultado.hayMas);
        if (!resultado.citas.length && pagina > 0) setPagina((p) => p - 1);
      })
      .catch((fallo) => {
        if (!controlador.signal.aborted) setError(mensajeError(fallo));
      })
      .finally(() => {
        if (!controlador.signal.aborted) setCargando(false);
      });
    return () => controlador.abort();
  }, [anfitrion!.correo, filtro, pagina, revision]);

  // Sólo se pide cuando la vista Calendario está activa -- evita traer
  // hasta 500 citas en cada carga de la pantalla cuando la mayoría de las
  // veces se usa la lista paginada de a 12.
  useEffect(() => {
    if (vista !== "calendario") return;
    const controlador = new AbortController();
    setCargandoCalendario(true);
    setErrorCalendario(null);
    listarCitasCalendario(anfitrion!.correo, filtro, controlador.signal)
      .then((resultado) => {
        if (controlador.signal.aborted) return;
        setCitasCalendario(resultado);
      })
      .catch((fallo) => {
        if (!controlador.signal.aborted) setErrorCalendario(mensajeError(fallo));
      })
      .finally(() => {
        if (!controlador.signal.aborted) setCargandoCalendario(false);
      });
    return () => controlador.abort();
  }, [anfitrion!.correo, filtro, vista, revision]);

  useEffect(() => {
    const alVolver = () => {
      if (document.visibilityState === "visible") setRevision((v) => v + 1);
    };
    const intervalo = setInterval(alVolver, 60_000);
    document.addEventListener("visibilitychange", alVolver);
    return () => {
      clearInterval(intervalo);
      document.removeEventListener("visibilitychange", alVolver);
    };
  }, []);

  async function cancelar() {
    if (!cancelacion || !verificado || bloqueo.current) return;
    bloqueo.current = true;
    setCancelando(true);
    setErrorCancelar(null);
    try {
      await cancelarCita(cancelacion.id, anfitrion!.correo);
      setCancelacion(null);
      setAviso("La cita se canceló correctamente.");
      setRevision((v) => v + 1);
    } catch (fallo) {
      setErrorCancelar(mensajeError(fallo));
    } finally {
      bloqueo.current = false;
      setCancelando(false);
    }
  }

  return (
    <div className="pagina">
      <div className="encabezado-pagina">
        <div>
          <p className="antetitulo">TU AGENDA</p>
          <h1>Mis citas</h1>
        </div>
        <Link className="boton boton-primario" to="/nueva">
          <CirclePlus aria-hidden="true" />
          Nueva cita
        </Link>
      </div>
      {aviso && (
        <Aviso tipo="exito">
          <div className="aviso-con-accion">
            <span>{aviso}</span>
            <button
              className="boton boton-discreto solo-icono"
              aria-label="Cerrar aviso"
              onClick={() => setAviso(null)}
            >
              <X aria-hidden="true" />
            </button>
          </div>
        </Aviso>
      )}
      <section className={`tarjeta agenda ${vista === "calendario" ? "agenda-llena" : ""}`}>
        <div className="agenda-herramientas">
          <div
            className="filtros"
            role="group"
            aria-label="Filtrar citas por estado"
          >
            {FILTROS.map((item) => (
              <button
                key={item.valor}
                className={filtro === item.valor ? "filtro activo" : "filtro"}
                aria-pressed={filtro === item.valor}
                onClick={() => {
                  setFiltro(item.valor);
                  setPagina(0);
                }}
              >
                {item.nombre}
              </button>
            ))}
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <div
              className="toggle-vista"
              role="group"
              aria-label="Cambiar cómo se muestra la agenda"
            >
              <button
                className={vista === "lista" ? "filtro activo" : "filtro"}
                aria-pressed={vista === "lista"}
                onClick={() => setVista("lista")}
              >
                <List aria-hidden="true" />
                Lista
              </button>
              <button
                className={vista === "calendario" ? "filtro activo" : "filtro"}
                aria-pressed={vista === "calendario"}
                onClick={() => setVista("calendario")}
              >
                <CalendarRange aria-hidden="true" />
                Calendario
              </button>
            </div>
            <button
              className="boton boton-discreto"
              disabled={vista === "lista" ? cargando : cargandoCalendario}
              onClick={() => setRevision((v) => v + 1)}
            >
              <RefreshCw aria-hidden="true" />
              Actualizar
            </button>
          </div>
        </div>
        {vista === "calendario" ? (
          cargandoCalendario ? (
            <Cargando texto="Consultando tu agenda…" />
          ) : errorCalendario ? (
            <div className="estado-agenda">
              <Aviso>{errorCalendario}</Aviso>
              <button className="boton" onClick={() => setRevision((v) => v + 1)}>
                Volver a intentar
              </button>
            </div>
          ) : (
            <div className="agenda-calendario-cuerpo">
              <CitasCalendario citas={citasCalendario} onSeleccionar={setDetalle} />
            </div>
          )
        ) : cargando ? (
          <Cargando texto="Consultando tus citas…" />
        ) : error ? (
          <div className="estado-agenda">
            <Aviso>{error}</Aviso>
            <button className="boton" onClick={() => setRevision((v) => v + 1)}>
              Volver a intentar
            </button>
          </div>
        ) : citas.length === 0 ? (
          <div className="estado-vacio">
            <span className="vacio-icono">
              <CalendarDays aria-hidden="true" />
            </span>
            <h2>
              {filtro === "TODAS"
                ? "Tu próxima visita empieza aquí"
                : "No hay citas en este estado"}
            </h2>
            <p>
              {filtro === "TODAS"
                ? "Creá tu primera cita y dejá todo preparado para recibir a tus visitantes."
                : "Probá con otro filtro para consultar el resto de tu agenda."}
            </p>
            <Link className="boton boton-primario" to="/nueva">
              <CirclePlus aria-hidden="true" />
              Agendar una visita
            </Link>
          </div>
        ) : (
          <div className="lista-citas">
            {citas.map((cita) => {
              const estado = estadoCita(cita);
              const personas = cita.cita_visitantes;
              const titulo =
                cita.motivo ||
                (personas.length === 1
                  ? `Visita de ${personas[0]!.nombre}`
                  : `Visita de ${personas.length} personas`);
              return (
                <article className="cita" key={cita.id}>
                  <div className="cita-fecha" aria-hidden="true">
                    <span>{cita.fecha_desde.slice(8)}</span>
                    <small>
                      {new Intl.DateTimeFormat("es-CR", {
                        month: "short",
                        timeZone: "UTC",
                      }).format(new Date(`${cita.fecha_desde}T12:00:00Z`))}
                    </small>
                  </div>
                  <div className="cita-contenido">
                    <div className="cita-titulo">
                      <h3>{titulo}</h3>
                      <span className={`estado estado-${estado.toLowerCase()}`}>
                        {etiquetaEstado[estado]}
                      </span>
                    </div>
                    <p className="cita-fechas">
                      {fechaLegible(cita.fecha_desde)}
                      {cita.fecha_hasta !== cita.fecha_desde &&
                        ` — ${fechaLegible(cita.fecha_hasta)}`}
                    </p>
                    <div className="cita-meta">
                      <span>
                        <Users aria-hidden="true" />
                        {personas.length}{" "}
                        {personas.length === 1 ? "visitante" : "visitantes"}
                      </span>
                      <span>
                        <MapPin aria-hidden="true" />
                        {cita.cita_sitios
                          .map((s) => s.sitios?.nombre ?? "Sitio no disponible")
                          .join(", ") || "Sin sitios"}
                      </span>
                    </div>
                  </div>
                  <div className="cita-acciones">
                    <button className="boton" onClick={() => setDetalle(cita)}>
                      Ver detalles
                      <ChevronRight aria-hidden="true" />
                    </button>
                    {estado === "VIGENTE" && (
                      <button
                        className="boton boton-discreto boton-peligro"
                        disabled={!verificado}
                        onClick={() => {
                          setErrorCancelar(null);
                          setCancelacion(cita);
                        }}
                      >
                        Cancelar cita
                      </button>
                    )}
                  </div>
                </article>
              );
            })}
          </div>
        )}
        {vista === "lista" && !cargando && !error && (citas.length > 0 || pagina > 0) && (
          <div className="paginacion">
            <span>Página {pagina + 1}</span>
            <div>
              <button
                className="boton"
                disabled={pagina === 0}
                onClick={() => setPagina((v) => v - 1)}
              >
                <ChevronLeft aria-hidden="true" />
                Anterior
              </button>
              <button
                className="boton"
                disabled={!hayMas}
                onClick={() => setPagina((v) => v + 1)}
              >
                Siguiente
                <ChevronRight aria-hidden="true" />
              </button>
            </div>
          </div>
        )}
      </section>
      <p className="nota-agenda">
        <MapPin aria-hidden="true" />
        El ingreso se registra al presentar el documento en el punto de acceso.
      </p>
      {detalle && (
        <Modal titulo="Detalles de la cita" onCerrar={() => setDetalle(null)}>
          <span
            className={`estado estado-${estadoCita(detalle).toLowerCase()}`}
          >
            {etiquetaEstado[estadoCita(detalle)]}
          </span>
          <dl className="datos-revision">
            <div>
              <dt>Fechas</dt>
              <dd>
                {fechaLegible(detalle.fecha_desde)} —{" "}
                {fechaLegible(detalle.fecha_hasta)}
              </dd>
            </div>
            <div>
              <dt>Sitios</dt>
              <dd>
                {detalle.cita_sitios
                  .map((s) => s.sitios?.nombre ?? "Sitio no disponible")
                  .join(", ")}
              </dd>
            </div>
            {detalle.motivo && (
              <div>
                <dt>Motivo</dt>
                <dd>{detalle.motivo}</dd>
              </div>
            )}
          </dl>
          <h3>Visitantes ({detalle.cita_visitantes.length})</h3>
          <ul className="personas-revision">
            {detalle.cita_visitantes.map((v) => (
              <li key={v.id}>
                <span className="avatar" aria-hidden="true">
                  <Users />
                </span>
                <div>
                  <strong>{v.nombre}</strong>
                  <span>
                    {v.cedula}
                    {v.empresa && ` · ${v.empresa}`}
                    {v.placa_vehiculo && ` · Placa: ${v.placa_vehiculo}`}
                  </span>
                </div>
              </li>
            ))}
          </ul>
          <div className="acciones-formulario">
            <button className="boton" onClick={() => setDetalle(null)}>
              Cerrar
            </button>
          </div>
        </Modal>
      )}
      {cancelacion && (
        <Modal
          titulo="Cancelar esta cita"
          ocupado={cancelando}
          onCerrar={() => setCancelacion(null)}
        >
          <p>
            La visita de {fechaLegible(cancelacion.fecha_desde)} quedará
            cancelada para todas las personas y sitios incluidos.
          </p>
          <p className="texto-secundario">
            La cita seguirá en tu historial. Para agendar otra visita tendrás
            que crear una nueva cita.
          </p>
          {errorCancelar && (
            <Aviso>
              {errorCancelar}
              <p>
                Si ya la cancelaste desde otra pestaña, cerrá este diálogo y
                actualizá la lista.
              </p>
            </Aviso>
          )}
          <div className="acciones-formulario">
            <button
              className="boton"
              disabled={cancelando}
              onClick={() => setCancelacion(null)}
            >
              Conservar cita
            </button>
            <button
              className="boton boton-peligro"
              disabled={cancelando || !verificado}
              onClick={() => void cancelar()}
            >
              {cancelando ? "Cancelando…" : "Sí, cancelar cita"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
