import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { Link, useBlocker, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  CalendarDays,
  Check,
  MapPin,
  Plus,
  Trash2,
  Users,
} from "lucide-react";
import { crearCita, listarSitios, mensajeError } from "../api";
import { esquemaNuevaCita, MAX_VISITANTES, visitanteVacio } from "../dominio";
import type { FormularioCita, Sitio } from "../dominio";
import { fechaLegible, hoyCostaRica } from "../fecha";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, Cargando, Modal } from "../componentes/Comunes";
import SelectorFechas from "../componentes/SelectorFechas";

export default function NuevaCita() {
  const { verificado } = useAuth();
  const navegar = useNavigate();
  const [formulario, setFormulario] = useState<FormularioCita>(() => ({
    fecha_desde: hoyCostaRica(),
    fecha_hasta: hoyCostaRica(),
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
  const [paso, setPaso] = useState<"datos" | "revision">("datos");
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
  const claves = useRef([crypto.randomUUID()]);
  const titulo = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    const controlador = new AbortController();
    setCargando(true);
    setErrorSitios(null);
    listarSitios(controlador.signal)
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
  useEffect(() => {
    titulo.current?.focus();
  }, [paso]);
  useEffect(() => {
    if (Object.keys(errores).length) resumenErrores.current?.focus();
  }, [errores]);

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
  function validarFormulario() {
    const resultado = esquemaNuevaCita().safeParse(formulario);
    if (!resultado.success) {
      const porCampo: Record<string, string> = {};
      for (const problema of resultado.error.issues)
        porCampo[problema.path.join(".")] ??= problema.message;
      setErrores(porCampo);
      return false;
    }
    setErrores({});
    return true;
  }
  function revisar(evento: FormEvent) {
    evento.preventDefault();
    if (!validarFormulario()) return;
    setPaso("revision");
  }
  async function guardar() {
    if (bloqueo.current || !verificado) return;
    if (!enviado && !validarFormulario()) {
      setPaso("datos");
      return;
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
            {paso === "datos" ? "Nueva cita" : "Revisá tu cita"}
          </h1>
        </div>
        <span className="indicador-paso">
          {paso === "datos" ? "1. Datos de la visita" : "2. Confirmación"}
        </span>
      </div>
      <div className="formulario-layout">
        <div>
          {paso === "datos" ? (
            <form onSubmit={revisar} noValidate>
              {Object.keys(errores).length > 0 && (
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
                <legend>
                  <span className="numero-paso">1</span> Lugar y fechas
                </legend>
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
                        className="enlace-boton"
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
                            {sitio.direccion && (
                              <small>{sitio.direccion}</small>
                            )}
                          </span>
                        </label>
                      ))}
                    </div>
                  )}
                  {mensajeCampo("sitios")}
                </div>
                <div className="campo">
                  Fechas de la visita
                  <span className="ayuda-campo">
                    Hacé click en un día, o arrastrá para elegir un rango.
                  </span>
                  <div
                    role="group"
                    aria-label="Fechas de la visita"
                    aria-invalid={!!(errores.fecha_desde || errores.fecha_hasta)}
                  >
                    <SelectorFechas
                      desde={formulario.fecha_desde}
                      hasta={formulario.fecha_hasta}
                      onCambiar={(fecha_desde, fecha_hasta) =>
                        actualizar({ fecha_desde, fecha_hasta })
                      }
                    />
                  </div>
                  {mensajeCampo("fecha_desde") ?? mensajeCampo("fecha_hasta")}
                </div>
                <label className="campo">
                  Motivo de la visita <span className="opcional">Opcional</span>
                  <textarea
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
              <fieldset
                className="tarjeta bloque-formulario"
                disabled={!verificado}
              >
                <legend>
                  <span className="numero-paso">2</span> Visitantes{" "}
                  <span className="contador-grupo">
                    {formulario.visitantes.length}
                  </span>
                </legend>
                <p className="descripcion-bloque">
                  El documento debe coincidir con el que presentarán al llegar.
                </p>
                <div className="visitantes-formulario">
                  {formulario.visitantes.map((persona, i) => (
                    <section
                      className="visitante-formulario"
                      key={claves.current[i]}
                      aria-label={`Visitante ${i + 1}`}
                    >
                      <div className="visitante-encabezado">
                        <h3>
                          <Users aria-hidden="true" />
                          Visitante {i + 1}
                        </h3>
                        {formulario.visitantes.length > 1 && (
                          <button
                            type="button"
                            className="boton boton-discreto boton-peligro"
                            aria-label={`Quitar visitante ${i + 1}`}
                            onClick={() => {
                              claves.current.splice(i, 1);
                              actualizar({
                                visitantes: formulario.visitantes.filter(
                                  (_, indice) => indice !== i,
                                ),
                              });
                            }}
                          >
                            <Trash2 aria-hidden="true" />
                            Quitar
                          </button>
                        )}
                      </div>
                      <div className="dos-columnas">
                        <label className="campo">
                          Nombre completo <span className="obligatorio">*</span>
                          <input
                            autoComplete="off"
                            value={persona.nombre}
                            maxLength={150}
                            required
                            {...atributos(`visitantes.${i}.nombre`)}
                            onChange={(e) =>
                              visitante(i, "nombre", e.target.value)
                            }
                          />
                          {mensajeCampo(`visitantes.${i}.nombre`)}
                        </label>
                        <label className="campo">
                          Cédula o documento{" "}
                          <span className="obligatorio">*</span>
                          <input
                            autoComplete="off"
                            spellCheck={false}
                            value={persona.cedula}
                            maxLength={60}
                            required
                            {...atributos(`visitantes.${i}.cedula`)}
                            onChange={(e) =>
                              visitante(i, "cedula", e.target.value)
                            }
                          />
                          {mensajeCampo(`visitantes.${i}.cedula`)}
                        </label>
                        <label className="campo">
                          Empresa <span className="opcional">Opcional</span>
                          <input
                            autoComplete="off"
                            value={persona.empresa}
                            maxLength={150}
                            {...atributos(`visitantes.${i}.empresa`)}
                            onChange={(e) =>
                              visitante(i, "empresa", e.target.value)
                            }
                          />
                          {mensajeCampo(`visitantes.${i}.empresa`)}
                        </label>
                        <label className="campo">
                          Placa del vehículo{" "}
                          <span className="opcional">Opcional</span>
                          <input
                            autoComplete="off"
                            value={persona.placa_vehiculo}
                            maxLength={20}
                            {...atributos(`visitantes.${i}.placa_vehiculo`)}
                            onChange={(e) =>
                              visitante(i, "placa_vehiculo", e.target.value)
                            }
                          />
                          {mensajeCampo(`visitantes.${i}.placa_vehiculo`)}
                        </label>
                      </div>
                    </section>
                  ))}
                </div>
                <button
                  type="button"
                  className="boton agregar-visitante"
                  disabled={formulario.visitantes.length >= MAX_VISITANTES}
                  onClick={() => {
                    claves.current.push(crypto.randomUUID());
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
                  type="submit"
                  className="boton boton-primario"
                  disabled={
                    !verificado ||
                    cargando ||
                    !!errorSitios ||
                    sitios.length === 0
                  }
                >
                  Revisar cita
                  <ArrowRight aria-hidden="true" />
                </button>
              </div>
            </form>
          ) : (
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
                  <li key={claves.current[i]}>
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
                      El resultado del envío no está confirmado. Usá «Reintentar
                      guardado» para comprobar la misma solicitud sin
                      duplicarla.
                    </p>
                  )}
                </Aviso>
              )}
              <div className="acciones-formulario">
                {!enviado && (
                  <button
                    className="boton"
                    disabled={guardando}
                    onClick={() => setPaso("datos")}
                  >
                    <ArrowLeft aria-hidden="true" />
                    Editar datos
                  </button>
                )}
                <button
                  className="boton boton-primario"
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
          )}
        </div>
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
              ? "Todavía no confirmamos el resultado del envío. Recomendamos reintentar el guardado antes de salir. Si salís, revisá Mis citas antes de crear otra para evitar duplicados."
              : "Si salís de esta pantalla, perderás los datos que ingresaste."}
          </p>
          <div className="acciones-formulario">
            <button
              className="boton"
              disabled={guardando}
              onClick={() => salida.reset()}
            >
              Seguir aquí
            </button>
            <button
              className="boton boton-peligro"
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
