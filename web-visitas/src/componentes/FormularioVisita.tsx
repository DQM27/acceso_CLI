import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useBlocker } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Check, Plus } from "lucide-react";
import { listarSitios, mensajeError } from "../api";
import {
  normalizarDocumento,
  validarCita,
  visitanteVacio,
  MAX_VISITANTES,
} from "../dominio";
import type { FormularioCita, VisitanteAnterior, VisitanteFormulario } from "../dominio";
import { hoyCostaRica, horaLegible, rangoLegible, sumarDias } from "../fecha";
import { Aviso, Cargando, Dialogo } from "./Comunes";
import BuscarAnteriores from "./BuscarAnteriores";
import SelectorFecha from "./SelectorFecha";
import { useEnLinea } from "../enLinea";
import { useEstadoReglas } from "../reglas";

type Errores = Record<string, string>;
type Cuando = "hoy" | "manana" | "otro" | "varios";

function cuandoDe(formulario: FormularioCita, hoy: string): Cuando {
  if (formulario.fecha_desde !== formulario.fecha_hasta) return "varios";
  if (formulario.fecha_desde === hoy) return "hoy";
  if (formulario.fecha_desde === sumarDias(hoy, 1)) return "manana";
  return "otro";
}

/** Formulario de una visita en una sola página: quién viene, cuándo, dónde y
 * el motivo, con el botón siempre visible abajo. Lo usan Agendar, Editar y
 * Duplicar. Valida con las reglas del núcleo (`validarCita`, WebAssembly); las que
 * necesitan datos (fecha del servidor, límites, que nadie haya entrado al
 * editar) las vuelve a revisar la base y su mensaje se muestra tal cual. */
export default function FormularioVisita({
  inicial,
  textoBoton,
  onGuardar,
}: {
  inicial: FormularioCita;
  textoBoton: string;
  onGuardar: (formulario: FormularioCita) => Promise<void>;
}) {
  const hoy = hoyCostaRica();
  const [formulario, setFormulario] = useState<FormularioCita>(inicial);
  const [cuando, setCuando] = useState<Cuando>(() => cuandoDe(inicial, hoy));
  const [errores, setErrores] = useState<Errores>({});
  const [errorGeneral, setErrorGeneral] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const [guardado, setGuardado] = useState(false);
  const sinCambios = JSON.stringify(formulario) === JSON.stringify(inicial);
  // Sin conexión el aviso lo da AuthContexto; aquí sólo se frena el envío.
  const enLinea = useEnLinea();
  // Las reglas del núcleo (WebAssembly) validan antes de enviar.
  const estadoReglas = useEstadoReglas();

  const sitios = useQuery({ queryKey: ["sitios"], queryFn: ({ signal }) => listarSitios(signal), staleTime: 300_000 });

  // Con un solo lugar posible, ya queda elegido.
  const unico = sitios.data?.length === 1 ? sitios.data[0] : undefined;
  const elegidos = formulario.sitios.length === 0 && unico ? [unico.id] : formulario.sitios;

  // Salir con cambios sin guardar pide confirmación (navegación interna y
  // cerrar la pestaña).
  const bloqueo = useBlocker(
    ({ currentLocation, nextLocation }) =>
      !sinCambios && !guardado && currentLocation.pathname !== nextLocation.pathname,
  );
  useEffect(() => {
    if (sinCambios || guardado) return;
    const alSalir = (evento: BeforeUnloadEvent) => evento.preventDefault();
    window.addEventListener("beforeunload", alSalir);
    return () => window.removeEventListener("beforeunload", alSalir);
  }, [sinCambios, guardado]);

  function cambiar<K extends keyof FormularioCita>(campo: K, valor: FormularioCita[K]) {
    setFormulario((f) => ({ ...f, [campo]: valor }));
    setErrores((e) => ({ ...e, [campo]: "" }));
  }

  function elegirCuando(opcion: Cuando) {
    setCuando(opcion);
    if (opcion === "hoy") setFormulario((f) => ({ ...f, fecha_desde: hoy, fecha_hasta: hoy }));
    if (opcion === "manana") {
      const manana = sumarDias(hoy, 1);
      setFormulario((f) => ({ ...f, fecha_desde: manana, fecha_hasta: manana }));
    }
    if (opcion === "varios")
      setFormulario((f) => ({ ...f, fecha_hasta: f.fecha_hasta > f.fecha_desde ? f.fecha_hasta : sumarDias(f.fecha_desde, 1) }));
    setErrores((e) => ({ ...e, fecha_desde: "", fecha_hasta: "" }));
  }

  function cambiarVisitante(indice: number, cambios: Partial<VisitanteFormulario>) {
    setFormulario((f) => ({
      ...f,
      visitantes: f.visitantes.map((v, i) => (i === indice ? { ...v, ...cambios } : v)),
    }));
    setErrores((e) => {
      const limpio = { ...e };
      for (const campo of Object.keys(cambios)) delete limpio[`visitantes.${indice}.${campo}`];
      return limpio;
    });
  }

  function agregarAnterior(persona: VisitanteAnterior) {
    setFormulario((f) => {
      if (f.visitantes.some((v) => normalizarDocumento(v.cedula) === normalizarDocumento(persona.cedula))) return f;
      const nueva = {
        nombre: persona.nombre.toUpperCase(),
        cedula: persona.cedula,
        empresa: persona.empresa ?? "",
        placa_vehiculo: persona.placa_vehiculo ?? "",
      };
      // Si la única tarjeta está vacía, se reemplaza en vez de sumar otra.
      const soloVacia = f.visitantes.length === 1 && !f.visitantes[0].nombre && !f.visitantes[0].cedula;
      return { ...f, visitantes: soloVacia ? [nueva] : [...f.visitantes, nueva] };
    });
  }

  async function enviar(evento: FormEvent) {
    evento.preventDefault();
    setErrorGeneral(null);
    const completo = { ...formulario, sitios: elegidos };
    const resultado = validarCita(completo, hoy);
    if (!resultado.ok) {
      setErrores(resultado.errores);
      setErrorGeneral("Revise los datos marcados.");
      return;
    }
    setEnviando(true);
    try {
      setGuardado(true);
      await onGuardar(completo);
    } catch (error) {
      setGuardado(false);
      setErrorGeneral(mensajeError(error));
    } finally {
      setEnviando(false);
    }
  }

  const nombresSitios = (sitios.data ?? []).filter((s) => elegidos.includes(s.id)).map((s) => s.nombre);
  const personas = formulario.visitantes.length;
  const resumenFecha = rangoLegible(formulario.fecha_desde, formulario.fecha_hasta);
  const resumenHora = horaLegible(formulario.hora_estimada);

  return (
    <form onSubmit={enviar} noValidate className="flex min-h-[calc(100dvh-56px)] flex-col">
      <div className="mx-auto flex w-full max-w-[720px] flex-1 flex-col gap-3.5 p-4">
        {estadoReglas === "error" && (
          <Aviso>No se pudieron cargar las reglas para revisar la visita. Recargue la página.</Aviso>
        )}
        {errorGeneral && <Aviso>{errorGeneral}</Aviso>}

        <section className="tarjeta flex flex-col gap-3 p-3.5" aria-labelledby="titulo-quien">
          <h2 id="titulo-quien" className="m-0 text-[15px]">
            ¿Quién viene?
          </h2>
          <BuscarAnteriores alElegir={agregarAnterior} />
          {formulario.visitantes.map((visitante, indice) => (
            <TarjetaPersona
              key={indice}
              indice={indice}
              visitante={visitante}
              errores={errores}
              puedeQuitar={personas > 1}
              onCambiar={(cambios) => cambiarVisitante(indice, cambios)}
              onQuitar={() =>
                setFormulario((f) => ({ ...f, visitantes: f.visitantes.filter((_, i) => i !== indice) }))
              }
            />
          ))}
          {errores.visitantes && <span className="campo-error">{errores.visitantes}</span>}
          {personas < MAX_VISITANTES && (
            <button
              type="button"
              className="boton border-dashed bg-panel text-acento"
              onClick={() => setFormulario((f) => ({ ...f, visitantes: [...f.visitantes, visitanteVacio()] }))}
            >
              <Plus aria-hidden="true" />
              Agregar persona
            </button>
          )}
        </section>

        <section className="tarjeta flex flex-col gap-3 p-3.5" aria-labelledby="titulo-cuando">
          <h2 id="titulo-cuando" className="m-0 text-[15px]">
            ¿Cuándo?
          </h2>
          <div className="flex flex-wrap gap-1.5" role="group" aria-label="Día">
            <button type="button" className="capsula" aria-pressed={cuando === "hoy"} onClick={() => elegirCuando("hoy")}>
              Hoy
            </button>
            <button type="button" className="capsula" aria-pressed={cuando === "manana"} onClick={() => elegirCuando("manana")}>
              Mañana
            </button>
            <button type="button" className="capsula" aria-pressed={cuando === "varios"} onClick={() => elegirCuando("varios")}>
              Varios días
            </button>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <SelectorFecha
              rango={cuando === "varios"}
              desde={formulario.fecha_desde}
              hasta={formulario.fecha_hasta}
              minimo={hoy}
              error={errores.fecha_desde || errores.fecha_hasta}
              onCambiar={(desde, hasta) => {
                setFormulario((f) => ({ ...f, fecha_desde: desde, fecha_hasta: hasta }));
                if (cuando !== "varios") setCuando(cuandoDe({ ...formulario, fecha_desde: desde, fecha_hasta: hasta }, hoy));
                setErrores((er) => ({ ...er, fecha_desde: "", fecha_hasta: "" }));
              }}
            />
            <label className="campo" data-error={!!errores.hora_estimada}>
              Hora estimada
              <input type="time" value={formulario.hora_estimada} onChange={(e) => cambiar("hora_estimada", e.target.value)} />
              {errores.hora_estimada && <span className="campo-error">{errores.hora_estimada}</span>}
            </label>
          </div>
        </section>

        <section className="tarjeta flex flex-col gap-2.5 p-3.5" aria-labelledby="titulo-donde">
          <h2 id="titulo-donde" className="m-0 text-[15px]">
            ¿Dónde?
          </h2>
          {sitios.isPending && <Cargando texto="Cargando lugares…" />}
          {sitios.isError && <Aviso>{mensajeError(sitios.error)}</Aviso>}
          {sitios.data?.map((sitio) => {
            const elegido = elegidos.includes(sitio.id);
            return (
              <button
                key={sitio.id}
                type="button"
                className="opcion"
                aria-pressed={elegido}
                onClick={() =>
                  cambiar("sitios", elegido ? elegidos.filter((id) => id !== sitio.id) : [...elegidos, sitio.id])
                }
              >
                {elegido && <Check size={16} aria-hidden="true" />}
                {sitio.nombre}
              </button>
            );
          })}
          {errores.sitios && <span className="campo-error">{errores.sitios}</span>}
        </section>

        <section className="tarjeta p-3.5">
          <label className="campo text-[15px] font-semibold text-texto" data-error={!!errores.motivo}>
            Motivo
            <textarea
              rows={3}
              placeholder="Opcional: lo ve el puesto de control"
              value={formulario.motivo ?? ""}
              maxLength={1000}
              onChange={(e) => cambiar("motivo", e.target.value)}
              className="font-normal"
            />
            {errores.motivo && <span className="campo-error">{errores.motivo}</span>}
          </label>
        </section>
      </div>

      <div className="barra-pie">
        <div className="mx-auto flex max-w-[720px] items-center justify-between gap-3">
          <div className="min-w-0 text-[13px] text-muted">
            <div className="truncate">
              {personas} {personas === 1 ? "persona" : "personas"} · {resumenFecha}
              {resumenHora ? ` · ${resumenHora}` : ""}
            </div>
            <div className="truncate">{nombresSitios.length ? nombresSitios.join(", ") : "Elija el lugar"}</div>
          </div>
          <button type="submit" className="boton boton-primario min-h-11 shrink-0 px-5" disabled={enviando || !enLinea || estadoReglas !== "lista"}>
            {enviando ? "Guardando…" : textoBoton}
          </button>
        </div>
      </div>

      {bloqueo.state === "blocked" && (
        <Dialogo titulo="¿Salir sin guardar?" onCerrar={() => bloqueo.reset()}>
          <p className="m-0">Los datos de esta visita se van a perder.</p>
          <div className="flex justify-end gap-2">
            <button type="button" className="boton" onClick={() => bloqueo.reset()}>
              Seguir editando
            </button>
            <button type="button" className="boton boton-peligro" onClick={() => bloqueo.proceed()}>
              Salir
            </button>
          </div>
        </Dialogo>
      )}
    </form>
  );
}

function TarjetaPersona({
  indice,
  visitante,
  errores,
  puedeQuitar,
  onCambiar,
  onQuitar,
}: {
  indice: number;
  visitante: VisitanteFormulario;
  errores: Errores;
  puedeQuitar: boolean;
  onCambiar: (cambios: Partial<VisitanteFormulario>) => void;
  onQuitar: () => void;
}) {
  const error = (campo: string) => errores[`visitantes.${indice}.${campo}`];
  return (
    <fieldset className="m-0 flex flex-col gap-2 rounded-chico border border-borde p-3">
      <div className="flex items-center justify-between">
        <legend className="float-left p-0 font-semibold">Persona {indice + 1}</legend>
        {puedeQuitar && (
          <button type="button" className="boton boton-discreto boton-compacto text-error" onClick={onQuitar}>
            Quitar
          </button>
        )}
      </div>
      <div className="grid grid-cols-2 gap-2">
        <label className="campo col-span-2" data-error={!!error("nombre")}>
          Nombre
          {/* En mayúscula al escribir, como en el resto de la app. */}
          <input
            value={visitante.nombre}
            autoComplete="off"
            onChange={(e) => onCambiar({ nombre: e.target.value.toUpperCase() })}
          />
          {error("nombre") && <span className="campo-error">{error("nombre")}</span>}
        </label>
        <label className="campo" data-error={!!error("cedula")}>
          Cédula o documento
          <input value={visitante.cedula} autoComplete="off" onChange={(e) => onCambiar({ cedula: e.target.value })} />
          {error("cedula") && <span className="campo-error">{error("cedula")}</span>}
        </label>
        <label className="campo" data-error={!!error("placa_vehiculo")}>
          Placa
          <input
            value={visitante.placa_vehiculo ?? ""}
            placeholder="Opcional"
            autoComplete="off"
            onChange={(e) => onCambiar({ placa_vehiculo: e.target.value })}
          />
        </label>
        <label className="campo col-span-2" data-error={!!error("empresa")}>
          Empresa
          <input
            value={visitante.empresa ?? ""}
            placeholder="Opcional"
            autoComplete="off"
            onChange={(e) => onCambiar({ empresa: e.target.value })}
          />
        </label>
      </div>
    </fieldset>
  );
}
