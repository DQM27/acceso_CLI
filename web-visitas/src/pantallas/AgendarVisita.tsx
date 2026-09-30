import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, Plus, Search, Trash2, User } from "lucide-react";
import { crearVisitas, buscarVisitantesAnteriores, listarSitiosDelAnfitrion, mensajeError } from "../api";
import {
  ETIQUETAS_TIPO_DOCUMENTO,
  TIPOS_DOCUMENTO,
  esquemaNuevaVisita,
  invitadoVacio,
  normalizarDocumento,
} from "../dominio";
import type { InvitadoFormulario, TipoDocumento, VisitanteAnterior } from "../dominio";
import { fechaLegible, hoyCostaRica, horaLegible } from "../fecha";
import { Cargando } from "../componentes/Comunes";

function manana(): string {
  const fecha = new Date(`${hoyCostaRica()}T12:00:00Z`);
  fecha.setUTCDate(fecha.getUTCDate() + 1);
  return fecha.toISOString().slice(0, 10);
}

export default function AgendarVisita() {
  const navegar = useNavigate();
  const { data: sitios, isPending: cargandoSitios } = useQuery({
    queryKey: ["sitios-del-anfitrion"],
    queryFn: ({ signal }) => listarSitiosDelAnfitrion(signal),
  });

  const [fechaDesde, setFechaDesde] = useState(hoyCostaRica());
  const [fechaHasta, setFechaHasta] = useState(hoyCostaRica());
  const [horaDesde, setHoraDesde] = useState("08:00");
  const [horaHasta, setHoraHasta] = useState("17:00");
  const [sitiosElegidos, setSitiosElegidos] = useState<string[]>([]);
  const [motivo, setMotivo] = useState("");
  const [invitados, setInvitados] = useState<InvitadoFormulario[]>([]);
  const [busqueda, setBusqueda] = useState("");
  const [erroresCampos, setErroresCampos] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!sitios || sitios.length !== 1) return;
    // `Promise.resolve().then(...)` en vez de llamar `setSitiosElegidos`
    // directo -- evita que `react-hooks/set-state-in-effect` marque esta
    // actualización como síncrona dentro del efecto.
    Promise.resolve().then(() => setSitiosElegidos([sitios[0].id]));
  }, [sitios]);

  const [busquedaDebounced, setBusquedaDebounced] = useState("");
  useEffect(() => {
    const id = setTimeout(() => setBusquedaDebounced(busqueda.trim()), 250);
    return () => clearTimeout(id);
  }, [busqueda]);

  const { data: sugerencias } = useQuery({
    queryKey: ["visitantes-anteriores", busquedaDebounced],
    queryFn: ({ signal }) => buscarVisitantesAnteriores(busquedaDebounced, signal),
    enabled: busquedaDebounced.length >= 2,
  });

  const documentosYaAgregados = useMemo(
    () => new Set(invitados.map((i) => `${i.tipo_documento}|${normalizarDocumento(i.numero_documento)}`)),
    [invitados],
  );

  function agregarVisitanteAnterior(visitante: VisitanteAnterior) {
    const clave = `${visitante.tipo_documento}|${visitante.numero_documento}`;
    if (documentosYaAgregados.has(clave)) return;
    setInvitados((actual) => [
      ...actual,
      {
        tipo_documento: visitante.tipo_documento,
        numero_documento: visitante.numero_documento,
        nombre: visitante.nombre,
        empresa: visitante.empresa ?? "",
        telefono: "",
        correo: "",
        placa_vehiculo: "",
      },
    ]);
    setBusqueda("");
  }

  function agregarPersonaNueva() {
    setInvitados((actual) => [...actual, invitadoVacio()]);
  }

  function quitarInvitado(indice: number) {
    setInvitados((actual) => actual.filter((_, i) => i !== indice));
  }

  function actualizarInvitado(indice: number, campo: keyof InvitadoFormulario, valor: string) {
    setInvitados((actual) =>
      actual.map((inv, i) => (i === indice ? { ...inv, [campo]: valor } : inv)),
    );
  }

  const crear = useMutation({
    mutationFn: async () => {
      const grupoId = crypto.randomUUID();
      const formulario = {
        fecha_desde: fechaDesde,
        fecha_hasta: fechaHasta,
        hora_desde: horaDesde,
        hora_hasta: horaHasta,
        tipo_visita: "",
        motivo,
        requiere_escolta: false,
        sitios: sitiosElegidos,
        invitados,
      };
      return crearVisitas(grupoId, formulario);
    },
    onSuccess: () => {
      toast.success("Visita agendada.");
      navegar("/");
    },
    onError: (e) => {
      const resultado = esquemaNuevaVisita().safeParse({
        fecha_desde: fechaDesde,
        fecha_hasta: fechaHasta,
        hora_desde: horaDesde,
        hora_hasta: horaHasta,
        tipo_visita: "",
        motivo,
        requiere_escolta: false,
        sitios: sitiosElegidos,
        invitados,
      });
      if (!resultado.success) {
        const errores: Record<string, string> = {};
        for (const issue of resultado.error.issues) errores[issue.path.join(".")] = issue.message;
        setErroresCampos(errores);
        toast.error("Revise los datos marcados en el formulario.");
        return;
      }
      toast.error(mensajeError(e));
    },
  });

  function alEnviar(evento: React.FormEvent) {
    evento.preventDefault();
    setErroresCampos({});
    crear.mutate();
  }

  const resumen = useMemo(() => {
    const personas = invitados.length;
    const sitiosTexto = sitios?.filter((s) => sitiosElegidos.includes(s.id)).map((s) => s.nombre).join(", ");
    const fechaTexto =
      fechaDesde === fechaHasta ? fechaLegible(fechaDesde) : `${fechaLegible(fechaDesde)} a ${fechaLegible(fechaHasta)}`;
    if (personas === 0 || !sitiosTexto) return null;
    return `${personas} persona${personas === 1 ? "" : "s"} · ${sitiosTexto} · ${fechaTexto}, ${horaLegible(horaDesde)} a ${horaLegible(horaHasta)}`;
  }, [invitados.length, sitios, sitiosElegidos, fechaDesde, fechaHasta, horaDesde, horaHasta]);

  if (cargandoSitios) return <Cargando texto="Cargando sitios…" />;

  return (
    <form onSubmit={alEnviar} className="flex flex-col gap-8 pb-24">
      <div className="flex items-center gap-3">
        <button type="button" className="boton boton-fantasma boton-icono" onClick={() => navegar("/")} aria-label="Volver">
          <ArrowLeft aria-hidden="true" />
        </button>
        <h1 className="text-xl font-semibold">Agendar visita</h1>
      </div>

      <section className="flex flex-col gap-4">
        <h2 className="text-sm font-semibold text-muted">¿Quién viene?</h2>

        <div className="campo">
          <label htmlFor="buscar-visitante">Nombre o documento</label>
          <div className="relative">
            <Search aria-hidden="true" size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
            <input
              id="buscar-visitante"
              className="pl-9!"
              placeholder="Buscar entre sus visitantes anteriores…"
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
            />
          </div>
          {busquedaDebounced.length >= 2 && sugerencias && sugerencias.length > 0 && (
            <ul className="tarjeta flex flex-col divide-y divide-borde">
              {sugerencias.map((s) => (
                <li key={s.id}>
                  <button
                    type="button"
                    className="flex w-full items-center gap-3 p-3 text-left hover:bg-[var(--campo-fondo)]"
                    onClick={() => agregarVisitanteAnterior(s)}
                  >
                    <User aria-hidden="true" size={16} className="text-muted" />
                    <span className="min-w-0 flex-1">
                      <span className="block font-medium">{s.nombre}</span>
                      <span className="block text-xs text-muted">
                        {ETIQUETAS_TIPO_DOCUMENTO[s.tipo_documento]} {s.numero_documento}
                        {s.empresa ? ` · ${s.empresa}` : ""}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        {invitados.length > 0 && (
          <ul className="flex flex-col gap-3">
            {invitados.map((invitado, indice) => (
              <li key={indice} className="tarjeta flex flex-col gap-3 p-4">
                <div className="flex items-start justify-between gap-2">
                  <span className="font-medium">Persona {indice + 1}</span>
                  <button
                    type="button"
                    className="boton boton-fantasma boton-icono"
                    aria-label="Quitar persona"
                    onClick={() => quitarInvitado(indice)}
                  >
                    <Trash2 aria-hidden="true" size={16} />
                  </button>
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <label className="campo">
                    Tipo de documento
                    <select
                      value={invitado.tipo_documento}
                      onChange={(e) => actualizarInvitado(indice, "tipo_documento", e.target.value as TipoDocumento)}
                    >
                      {TIPOS_DOCUMENTO.map((t) => (
                        <option key={t} value={t}>
                          {ETIQUETAS_TIPO_DOCUMENTO[t]}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="campo">
                    Número de documento
                    <input
                      value={invitado.numero_documento}
                      onChange={(e) => actualizarInvitado(indice, "numero_documento", e.target.value)}
                      aria-invalid={!!erroresCampos[`invitados.${indice}.numero_documento`]}
                    />
                  </label>
                </div>
                <label className="campo">
                  Nombre completo
                  <input
                    value={invitado.nombre}
                    onChange={(e) => actualizarInvitado(indice, "nombre", e.target.value)}
                    aria-invalid={!!erroresCampos[`invitados.${indice}.nombre`]}
                  />
                </label>
                <label className="campo">
                  Empresa (opcional)
                  <input
                    value={invitado.empresa ?? ""}
                    onChange={(e) => actualizarInvitado(indice, "empresa", e.target.value)}
                  />
                </label>
                <label className="campo">
                  Placa del vehículo (opcional)
                  <input
                    value={invitado.placa_vehiculo ?? ""}
                    onChange={(e) => actualizarInvitado(indice, "placa_vehiculo", e.target.value)}
                  />
                </label>
              </li>
            ))}
          </ul>
        )}

        <button type="button" className="boton self-start" onClick={agregarPersonaNueva}>
          <Plus aria-hidden="true" size={16} />
          Agregar persona nueva
        </button>
        {erroresCampos.invitados && <p className="campo-error">{erroresCampos.invitados}</p>}
      </section>

      <section className="flex flex-col gap-4">
        <h2 className="text-sm font-semibold text-muted">¿Cuándo?</h2>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="boton"
            onClick={() => {
              setFechaDesde(hoyCostaRica());
              setFechaHasta(hoyCostaRica());
            }}
          >
            Hoy
          </button>
          <button
            type="button"
            className="boton"
            onClick={() => {
              setFechaDesde(manana());
              setFechaHasta(manana());
            }}
          >
            Mañana
          </button>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <label className="campo">
            Desde
            <input
              type="date"
              value={fechaDesde}
              min={hoyCostaRica()}
              onChange={(e) => setFechaDesde(e.target.value)}
              aria-invalid={!!erroresCampos.fecha_desde}
            />
            {erroresCampos.fecha_desde && <span className="campo-error">{erroresCampos.fecha_desde}</span>}
          </label>
          <label className="campo">
            Hasta
            <input
              type="date"
              value={fechaHasta}
              min={fechaDesde}
              onChange={(e) => setFechaHasta(e.target.value)}
              aria-invalid={!!erroresCampos.fecha_hasta}
            />
            {erroresCampos.fecha_hasta && <span className="campo-error">{erroresCampos.fecha_hasta}</span>}
          </label>
          <label className="campo">
            Hora de entrada
            <input type="time" value={horaDesde} onChange={(e) => setHoraDesde(e.target.value)} />
          </label>
          <label className="campo">
            Hora de salida
            <input
              type="time"
              value={horaHasta}
              onChange={(e) => setHoraHasta(e.target.value)}
              aria-invalid={!!erroresCampos.hora_hasta}
            />
            {erroresCampos.hora_hasta && <span className="campo-error">{erroresCampos.hora_hasta}</span>}
          </label>
        </div>
      </section>

      {sitios && sitios.length > 1 && (
        <section className="flex flex-col gap-4">
          <h2 className="text-sm font-semibold text-muted">¿Dónde?</h2>
          <div className="flex flex-wrap gap-2">
            {sitios.map((sitio) => {
              const elegido = sitiosElegidos.includes(sitio.id);
              return (
                <button
                  key={sitio.id}
                  type="button"
                  className={`boton ${elegido ? "boton-primario" : ""}`}
                  onClick={() =>
                    setSitiosElegidos((actual) =>
                      elegido ? actual.filter((s) => s !== sitio.id) : [...actual, sitio.id],
                    )
                  }
                >
                  {sitio.nombre}
                </button>
              );
            })}
          </div>
          {erroresCampos.sitios && <p className="campo-error">{erroresCampos.sitios}</p>}
        </section>
      )}

      <section className="flex flex-col gap-4">
        <h2 className="text-sm font-semibold text-muted">Motivo (opcional)</h2>
        <label className="campo">
          <textarea value={motivo} onChange={(e) => setMotivo(e.target.value)} rows={3} />
        </label>
      </section>

      <div className="sticky bottom-0 flex flex-col gap-2 border-t border-borde bg-[var(--fondo)] py-4">
        {resumen && <p className="text-sm text-muted">{resumen}</p>}
        <button type="submit" className="boton boton-primario w-full" disabled={crear.isPending}>
          {crear.isPending ? "Agendando…" : "Agendar"}
        </button>
      </div>
    </form>
  );
}
