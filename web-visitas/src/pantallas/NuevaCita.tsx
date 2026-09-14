import {
  Activity,
  addTransitionType,
  startTransition,
  useEffect,
  useRef,
  useState,
  ViewTransition,
} from "react";
import { Link, useBlocker, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  CalendarDays,
  Check,
  MapPin,
  Plus,
} from "lucide-react";
import { crearCita, listarSitios, mensajeError } from "../api";
import {
  esquemaNuevaCita,
  MAX_VISITANTES,
  validarRangoFechas,
  visitanteVacio,
} from "../dominio";
import type { FormularioCita, Sitio } from "../dominio";
import { fechaLegible, horaLegible, hoyCostaRica } from "../fecha";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, Cargando, Modal } from "../componentes/Comunes";
import CampoFechas from "../componentes/CampoFechas";
import PasoWizard from "../componentes/PasoWizard";
import VisitanteFormulario from "../componentes/VisitanteFormulario";
import { useFocoAlCambiar } from "../lib/useFocoAlCambiar";

type Paso = "cuando-donde" | "visitantes" | "revision";
const PASOS: Paso[] = ["cuando-donde", "visitantes", "revision"];
const ETIQUETAS_PASO = ["¿Cuándo y dónde?", "Visitantes", "Confirmar"];
// A qué paso pertenece cada clave de `errores` -- para bloquear el avance
// sólo por errores del paso actual, sin duplicar reglas de validación (el
// `safeParse` sigue siendo el único lugar que valida de verdad; esto sólo
// decide qué mostrar/bloquear en cada pantalla).
const CAMPOS_CUANDO_DONDE = [
  "sitios",
  "fecha_desde",
  "fecha_hasta",
  "hora_estimada",
  "motivo",
];
function perteneceAlPaso(clave: string, paso: Paso): boolean {
  if (paso === "revision") return false;
  const prefijos = paso === "cuando-donde" ? CAMPOS_CUANDO_DONDE : ["visitantes"];
  return prefijos.some((p) => clave === p || clave.startsWith(`${p}.`));
}

export default function NuevaCita() {
  const { verificado } = useAuth();
  const navegar = useNavigate();
  const [formulario, setFormulario] = useState<FormularioCita>(() => ({
    fecha_desde: hoyCostaRica(),
    fecha_hasta: hoyCostaRica(),
    hora_estimada: "",
    motivo: "",
    sitios: [],
    visitantes: [visitanteVacio()],
  }));
  const [sitios, setSitios] = useState<Sitio[]>([]);
  const [cargando, setCargando] = useState(true);
  const [errorSitios, setErrorSitios] = useState<string | null>(null);
  const [intentoSitios, setIntentoSitios] = useState(0);
  const [errores, setErrores] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [paso, setPaso] = useState<Paso>("cuando-donde");
  const [guardando, setGuardando] = useState(false);
  const [enviado, setEnviado] = useState(false);
  const [modificado, setModificado] = useState(false);
  const solicitud = useRef(crypto.randomUUID());
  const fechaValidacion = useRef<string | null>(null);
  const bloqueo = useRef(false);
  const guardado = useRef(false);
  const salida = useBlocker(
    ({ currentLocation, nextLocation }) =>
      modificado &&
      !guardado.current &&
      currentLocation.pathname !== nextLocation.pathname,
  );
  const resumenErrores = useRef<HTMLDivElement>(null);
  // Estado, no ref -- se lee durante el render (como `key` de cada fila),
  // y leer un ref durante el render no está garantizado por React
  // (`react-hooks/refs`). Se mantiene en paralelo a `formulario.visitantes`
  // (mismo índice) en los dos puntos donde ese array cambia: alta y baja.
  const [claves, setClaves] = useState([crypto.randomUUID()]);
  const titulo = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    const controlador = new AbortController();
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- evita que `react-hooks/set-state-in-effect` marque esta
    // actualización como síncrona dentro del efecto.
    Promise.resolve()
      .then(() => {
        setCargando(true);
        setErrorSitios(null);
      })
      .then(() => listarSitios(controlador.signal))
      .then((datos) => {
        if (!controlador.signal.aborted) setSitios(datos);
      })
      .catch((fallo) => {
        if (!controlador.signal.aborted) setErrorSitios(mensajeError(fallo));
      })
      .finally(() => {
        if (!controlador.signal.aborted) setCargando(false);
      });
    return () => controlador.abort();
  }, [intentoSitios]);

  useEffect(() => {
    if (!modificado) return;
    const antesDeSalir = (evento: BeforeUnloadEvent) => {
      evento.preventDefault();
    };
    window.addEventListener("beforeunload", antesDeSalir);
    return () => window.removeEventListener("beforeunload", antesDeSalir);
  }, [modificado]);
  useFocoAlCambiar(titulo, paso);
  const hayErroresDelPaso = Object.keys(errores).some((clave) =>
    perteneceAlPaso(clave, paso),
  );
  useEffect(() => {
    if (hayErroresDelPaso) resumenErrores.current?.focus();
    // Sólo debe re-disparar cuando cambian errores o de paso -- no en cada
    // render donde `hayErroresDelPaso` da el mismo resultado.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [errores, paso]);

  function actualizar(cambios: Partial<FormularioCita>) {
    setFormulario((actual) => ({ ...actual, ...cambios }));
    setModificado(true);
  }
  function visitante(
    indice: number,
    campo: keyof FormularioCita["visitantes"][number],
    valor: string,
  ) {
    actualizar({
      visitantes: formulario.visitantes.map((v, i) =>
        i === indice ? { ...v, [campo]: valor } : v,
      ),
    });
  }
  /** Corre SIEMPRE la validación completa (no hay sub-esquemas por paso --
   * evita duplicar reglas), pero sólo bloquea el avance si el paso ACTUAL
   * tiene errores propios; errores de un paso que todavía no se visitó
   * quedan guardados en `errores` para cuando se llegue ahí, sin bloquear
   * antes de tiempo. */
  function validarFormulario() {
    const resultado = esquemaNuevaCita().safeParse(formulario);
    const porCampo: Record<string, string> = {};
    if (!resultado.success)
      for (const problema of resultado.error.issues)
        porCampo[problema.path.join(".")] ??= problema.message;
    setErrores(porCampo);
    return porCampo;
  }
  /** Cambia de paso dentro de un `startTransition` -- `<ViewTransition>`
   * (más abajo, en el render) sólo anima actualizaciones marcadas como
   * Transition; `addTransitionType` deja elegir la animación (desde-derecha
   * vs. desde-izquierda) según la causa, no sólo el destino. Degrada solo:
   * en un navegador sin View Transitions, React aplica el cambio de estado
   * igual, sin animación. */
  function cambiarPaso(siguiente: Paso, tipo: "adelante" | "atras") {
    startTransition(() => {
      addTransitionType(tipo);
      setPaso(siguiente);
    });
  }
  function continuar(desde: Paso, hacia: Paso) {
    const erroresActuales = validarFormulario();
    const bloqueado = Object.keys(erroresActuales).some((clave) =>
      perteneceAlPaso(clave, desde),
    );
    if (bloqueado) return;
    // Al paso al que se recién se llega no se le muestran de entrada sus
    // propios errores (son campos que el usuario todavía no tocó) -- sólo
    // aparecen si más adelante intenta avanzar desde ahí sin completarlos.
    setErrores((previo) => {
      const siguiente = { ...previo };
      for (const clave of Object.keys(siguiente))
        if (perteneceAlPaso(clave, hacia)) delete siguiente[clave];
      return siguiente;
    });
    cambiarPaso(hacia, "adelante");
  }
  async function guardar() {
    if (bloqueo.current || !verificado) return;
    if (!enviado) {
      const erroresActuales = validarFormulario();
      const claves = Object.keys(erroresActuales);
      if (claves.length > 0) {
        const primerPaso = claves.some((c) => perteneceAlPaso(c, "cuando-donde"))
          ? "cuando-donde"
          : "visitantes";
        cambiarPaso(primerPaso, "atras");
        return;
      }
    }
    bloqueo.current = true;
    setGuardando(true);
    setEnviado(true);
    setError(null);
    try {
      fechaValidacion.current ??= hoyCostaRica();
      const id = await crearCita(
        solicitud.current,
        formulario,
        fechaValidacion.current,
      );
      guardado.current = true;
      setModificado(false);
      navegar("/citas", { replace: true, state: { creada: id } });
    } catch (fallo) {
      setError(mensajeError(fallo));
    } finally {
      bloqueo.current = false;
      setGuardando(false);
    }
  }
  const mensajeCampo = (clave: string) =>
    errores[clave] ? (
      <span className="error-campo" id={`error-${clave}`}>
        {errores[clave]}
      </span>
    ) : null;
  const atributos = (clave: string) => ({
    "aria-invalid": !!errores[clave],
    "aria-describedby": errores[clave] ? `error-${clave}` : undefined,
  });

  return (
    <div className="pagina">
      <Link className="volver" to="/citas">
        <ArrowLeft aria-hidden="true" />
        Mis citas
      </Link>
      <div className="encabezado-pagina">
        <div>
          <p className="antetitulo">PREPARÁ SU LLEGADA</p>
          <h1 ref={titulo} tabIndex={-1}>
            {paso === "revision" ? "Revisá tu cita" : "Nueva cita"}
          </h1>
        </div>
        <PasoWizard pasos={ETIQUETAS_PASO} actual={PASOS.indexOf(paso)} />
      </div>
      <div className="formulario-layout">
        {/* React 19.3: los 3 pasos quedan siempre montados, cada uno en su
            propio <Activity> -- al ocultar un paso, React pausa sus efectos
            pero conserva su DOM y estado (a diferencia del swap condicional,
            que desmontaba todo). Esto sólo fue viable después de reemplazar
            FullCalendar por un calendario propio en SelectorFechas.tsx: el
            wrapper de FullCalendar reinicializaba su vista cada vez que
            Activity corría de nuevo sus efectos al mostrar un paso oculto
            (el DOM sobrevivía, pero su efecto de inicialización no), así
            que perdía el mes navegado -- un componente de estado simple
            (`useState`) no tiene ese problema. <ViewTransition> anima el
            cambio con la View Transition API nativa del navegador cuando
            hay soporte, sin hacer nada (sin errores) si no lo hay. */}
        <ViewTransition>
          <div>
            <Activity mode={paso === "cuando-donde" ? "visible" : "hidden"}>
            <form
              onSubmit={(evento) => {
                evento.preventDefault();
                continuar("cuando-donde", "visitantes");
              }}
              noValidate
            >
              {hayErroresDelPaso && (
                <div
                  ref={resumenErrores}
                  tabIndex={-1}
                  className="resumen-errores"
                >
                  <Aviso>Revisá los campos marcados para continuar.</Aviso>
                </div>
              )}
              <fieldset
                className="tarjeta bloque-formulario"
                disabled={cargando || !verificado}
              >
                <legend>Lugar y fechas</legend>
                <p className="descripcion-bloque">
                  ¿Dónde y cuándo vas a recibir a tus visitantes?
                </p>
                <div className="campo">
                  <span id="etiqueta-sitios">
                    Sitios de la visita <span className="obligatorio">*</span>
                  </span>
                  <p className="ayuda-campo">
                    Podés seleccionar más de un sitio.
                  </p>
                  {cargando ? (
                    <Cargando texto="Cargando sitios…" />
                  ) : errorSitios ? (
                    <Aviso>
                      {errorSitios}
                      <button
                        type="button"
                        className="btn btn-link p-0 align-baseline"
                        onClick={() => setIntentoSitios((v) => v + 1)}
                      >
                        Reintentar
                      </button>
                    </Aviso>
                  ) : sitios.length === 0 ? (
                    <Aviso tipo="info">
                      Todavía no hay sitios disponibles. Contactá a
                      administración.
                    </Aviso>
                  ) : (
                    <div
                      className="selector-sitios"
                      role="group"
                      aria-labelledby="etiqueta-sitios"
                      {...atributos("sitios")}
                    >
                      {sitios.map((sitio) => (
                        <label
                          className={`sitio-opcion ${formulario.sitios.includes(sitio.id) ? "seleccionado" : ""}`}
                          key={sitio.id}
                        >
                          <input
                            type="checkbox"
                            checked={formulario.sitios.includes(sitio.id)}
                            onChange={(e) =>
                              actualizar({
                                sitios: e.target.checked
                                  ? [...formulario.sitios, sitio.id]
                                  : formulario.sitios.filter(
                                      (id) => id !== sitio.id,
                                    ),
                              })
                            }
                          />
                          <span className="sitio-indicador">
                            <Check aria-hidden="true" />
                          </span>
                          <MapPin aria-hidden="true" />
                          <span>
                            <strong>{sitio.nombre}</strong>
                          </span>
                        </label>
                      ))}
                    </div>
                  )}
                  {mensajeCampo("sitios")}
                </div>
                <div className="campo">
                  Fechas de la visita
                  <CampoFechas
                    desde={formulario.fecha_desde}
                    hasta={formulario.fecha_hasta}
                    erroresDesde={errores.fecha_desde}
                    erroresHasta={errores.fecha_hasta}
                    onCambiar={(fecha_desde, fecha_hasta) => {
                      actualizar({ fecha_desde, fecha_hasta });
                      const resultado = validarRangoFechas(
                        fecha_desde,
                        fecha_hasta,
                      );
                      setErrores((previo) => {
                        const siguiente = { ...previo };
                        if (resultado.fecha_desde)
                          siguiente.fecha_desde = resultado.fecha_desde;
                        else delete siguiente.fecha_desde;
                        if (resultado.fecha_hasta)
                          siguiente.fecha_hasta = resultado.fecha_hasta;
                        else delete siguiente.fecha_hasta;
                        return siguiente;
                      });
                    }}
                  />
                </div>
                <label className="campo">
                  Hora aproximada de llegada{" "}
                  <span className="opcional">Opcional</span>
                  <input
                    type="time"
                    className="form-control"
                    style={{ maxWidth: "12rem" }}
                    value={formulario.hora_estimada}
                    {...atributos("hora_estimada")}
                    onChange={(e) =>
                      actualizar({ hora_estimada: e.target.value })
                    }
                  />
                  {mensajeCampo("hora_estimada")}
                  <span className="ayuda-campo">
                    Es sólo para orientar al personal del sitio -- no hace
                    falta llegar puntual ni se bloquea el ingreso a otra hora.
                  </span>
                </label>
                <label className="campo">
                  Motivo de la visita <span className="opcional">Opcional</span>
                  <textarea
                    className="form-control"
                    rows={3}
                    maxLength={1000}
                    placeholder="Por ejemplo: reunión de coordinación"
                    value={formulario.motivo}
                    {...atributos("motivo")}
                    onChange={(e) => actualizar({ motivo: e.target.value })}
                  />
                  {mensajeCampo("motivo")}
                  <span className="ayuda-campo contador">
                    {formulario.motivo.length}/1000
                  </span>
                </label>
              </fieldset>
              <div className="acciones-formulario">
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={
                    !verificado ||
                    cargando ||
                    !!errorSitios ||
                    sitios.length === 0
                  }
                >
                  Continuar
                  <ArrowRight aria-hidden="true" />
                </button>
              </div>
            </form>
            </Activity>
            <Activity mode={paso === "visitantes" ? "visible" : "hidden"}>
            <form
              onSubmit={(evento) => {
                evento.preventDefault();
                continuar("visitantes", "revision");
              }}
              noValidate
            >
              {hayErroresDelPaso && (
                <div
                  ref={resumenErrores}
                  tabIndex={-1}
                  className="resumen-errores"
                >
                  <Aviso>Revisá los campos marcados para continuar.</Aviso>
                </div>
              )}
              <fieldset
                className="tarjeta bloque-formulario"
                disabled={!verificado}
              >
                <legend>
                  Visitantes{" "}
                  <span className="contador-grupo">
                    {formulario.visitantes.length}
                  </span>
                </legend>
                <p className="descripcion-bloque">
                  El documento debe coincidir con el que presentarán al llegar.
                </p>
                <div className="visitantes-formulario">
                  {formulario.visitantes.map((persona, i) => (
                    <VisitanteFormulario
                      key={claves[i]}
                      indice={i}
                      total={formulario.visitantes.length}
                      persona={persona}
                      esUltimo={i === formulario.visitantes.length - 1}
                      tieneError={["nombre", "cedula", "empresa", "placa_vehiculo"].some(
                        (campo) => errores[`visitantes.${i}.${campo}`],
                      )}
                      mensajeCampo={mensajeCampo}
                      atributos={atributos}
                      onCambiar={(campo, valor) => visitante(i, campo, valor)}
                      onQuitar={
                        formulario.visitantes.length > 1
                          ? () => {
                              setClaves((c) =>
                                c.filter((_, indice) => indice !== i),
                              );
                              actualizar({
                                visitantes: formulario.visitantes.filter(
                                  (_, indice) => indice !== i,
                                ),
                              });
                            }
                          : null
                      }
                    />
                  ))}
                </div>
                <button
                  type="button"
                  className="btn btn-outline-secondary agregar-visitante"
                  disabled={formulario.visitantes.length >= MAX_VISITANTES}
                  onClick={() => {
                    setClaves((c) => [...c, crypto.randomUUID()]);
                    actualizar({
                      visitantes: [...formulario.visitantes, visitanteVacio()],
                    });
                  }}
                >
                  <Plus aria-hidden="true" />
                  Agregar visitante
                </button>
                <p className="ayuda-campo">
                  Hasta {MAX_VISITANTES} personas por cita.
                </p>
              </fieldset>
              <div className="acciones-formulario">
                <span className="ayuda-campo">* Campos obligatorios</span>
                <button
                  type="button"
                  className="btn btn-outline-secondary"
                  onClick={() => cambiarPaso("cuando-donde", "atras")}
                >
                  <ArrowLeft aria-hidden="true" />
                  Atrás
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={!verificado}
                >
                  Continuar
                  <ArrowRight aria-hidden="true" />
                </button>
              </div>
            </form>
            </Activity>
            <Activity mode={paso === "revision" ? "visible" : "hidden"}>
            <section className="tarjeta bloque-revision">
              <div className="titulo-bloque">
                <Check aria-hidden="true" />
                <h2>Detalles de la visita</h2>
              </div>
              <dl className="datos-revision">
                <div>
                  <dt>Fechas</dt>
                  <dd>
                    {fechaLegible(formulario.fecha_desde)} —{" "}
                    {fechaLegible(formulario.fecha_hasta)}
                  </dd>
                </div>
                <div>
                  <dt>Sitios</dt>
                  <dd>
                    {sitios
                      .filter((s) => formulario.sitios.includes(s.id))
                      .map((s) => s.nombre)
                      .join(", ")}
                  </dd>
                </div>
                {formulario.hora_estimada && (
                  <div>
                    <dt>Hora aproximada</dt>
                    <dd>
                      {horaLegible(formulario.hora_estimada)}
                      <span>Informativa -- no bloquea el ingreso a otra hora.</span>
                    </dd>
                  </div>
                )}
                {formulario.motivo.trim() && (
                  <div>
                    <dt>Motivo</dt>
                    <dd>{formulario.motivo}</dd>
                  </div>
                )}
              </dl>
              <h3>Visitantes ({formulario.visitantes.length})</h3>
              <ul className="personas-revision">
                {formulario.visitantes.map((v, i) => (
                  <li key={claves[i]}>
                    <span className="avatar" aria-hidden="true">
                      {i + 1}
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
              {error && (
                <Aviso>
                  {error}
                  {enviado && (
                    <p>
                      No pudimos confirmar si tu cita quedó guardada. Presioná
                      «Reintentar guardado» -- es seguro, no se va a crear dos
                      veces.
                    </p>
                  )}
                </Aviso>
              )}
              <div className="acciones-formulario">
                {!enviado && (
                  <button
                    type="button"
                    className="btn btn-outline-secondary"
                    disabled={guardando}
                    onClick={() => cambiarPaso("visitantes", "atras")}
                  >
                    <ArrowLeft aria-hidden="true" />
                    Editar datos
                  </button>
                )}
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={guardando || !verificado}
                  onClick={() => void guardar()}
                >
                  {guardando
                    ? "Guardando…"
                    : enviado
                      ? "Reintentar guardado"
                      : "Confirmar y agendar"}
                  <Check aria-hidden="true" />
                </button>
              </div>
            </section>
            </Activity>
          </div>
        </ViewTransition>
        <aside className="resumen-lateral">
          <div className="tarjeta">
            <CalendarDays aria-hidden="true" />
            <h2>Tu visita, de un vistazo</h2>
            <dl>
              <div>
                <dt>Personas</dt>
                <dd>
                  {formulario.visitantes.length}{" "}
                  {formulario.visitantes.length === 1
                    ? "visitante"
                    : "visitantes"}
                </dd>
              </div>
              <div>
                <dt>Destinos</dt>
                <dd>
                  {formulario.sitios.length
                    ? `${formulario.sitios.length} ${formulario.sitios.length === 1 ? "sitio seleccionado" : "sitios seleccionados"}`
                    : "Por seleccionar"}
                </dd>
              </div>
              <div>
                <dt>Vigencia</dt>
                <dd>
                  {/^\d{4}-\d{2}-\d{2}$/.test(formulario.fecha_desde)
                    ? fechaLegible(formulario.fecha_desde)
                    : "Por definir"}
                  <span>
                    hasta{" "}
                    {/^\d{4}-\d{2}-\d{2}$/.test(formulario.fecha_hasta)
                      ? fechaLegible(formulario.fecha_hasta)
                      : "por definir"}
                  </span>
                </dd>
              </div>
              {formulario.hora_estimada && (
                <div>
                  <dt>Hora aproximada</dt>
                  <dd>{horaLegible(formulario.hora_estimada)}</dd>
                </div>
              )}
            </dl>
          </div>
          <div className="consejo">
            <MapPin aria-hidden="true" />
            <p>
              Al llegar, cada visitante debe presentar su documento en el punto
              de acceso.
            </p>
          </div>
        </aside>
      </div>
      {salida.state === "blocked" && (
        <Modal
          titulo={
            enviado ? "Guardado sin confirmar" : "Tenés cambios sin guardar"
          }
          ocupado={guardando}
          onCerrar={() => salida.reset()}
        >
          <p>
            {enviado
              ? "No sabemos con certeza si tu cita se guardó. Te recomendamos volver y presionar «Reintentar guardado» antes de salir; si igual salís, revisá Mis citas para confirmar antes de crear otra."
              : "Si salís de esta pantalla, perderás los datos que ingresaste."}
          </p>
          <div className="acciones-formulario">
            <button
              type="button"
              className="btn btn-outline-secondary"
              disabled={guardando}
              onClick={() => salida.reset()}
            >
              Seguir aquí
            </button>
            <button
              type="button"
              className="btn btn-danger"
              disabled={guardando}
              onClick={() => salida.proceed()}
            >
              Salir de la cita
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
