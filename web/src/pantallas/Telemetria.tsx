import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Activity, RotateCcw, Search, TriangleAlert } from "lucide-react";
import Modal from "../componentes/Modal";
import { useLista } from "../componentes/useLista";
import { listarDispositivosYSitios } from "../api/dispositivos";
import type { Dispositivo, Sitio } from "../api/dispositivos";
import {
  GRUPOS_POR_DEFECTO,
  GRUPOS_TELEMETRIA,
  LARGO_MAXIMO_NOTA,
  guardarOrdenTelemetria,
  listarCambiosTelemetria,
  listarOrdenesTelemetria,
} from "../api/telemetria";
import type { CambioOrdenTelemetria, GrupoTelemetria, OrdenTelemetria } from "../api/telemetria";
import {
  DESCRIPCION_GRUPO,
  OPCIONES_VENCIMIENTO,
  coincideBusqueda,
  esLoDeSiempre,
  gruposVigentes,
  ordenVencida,
  ordenarGrupos,
  textoGrupos,
  venceEnDesde,
} from "./Telemetria.logica";
import { mensajeError } from "../mensajeError";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

interface DatosTelemetria {
  sitios: Sitio[];
  /** Vigentes y visibles en el panel, de todos los tipos. */
  equipos: Dispositivo[];
  ordenes: Map<string, OrdenTelemetria>;
  /** Contra esta hora se decide qué orden ya venció. */
  cargadoEn: Date;
}

async function cargarDatos(): Promise<DatosTelemetria> {
  const [estado, ordenes] = await Promise.all([listarDispositivosYSitios(), listarOrdenesTelemetria()]);
  return {
    sitios: [...estado.sitios].sort((a, b) => a.nombre.localeCompare(b.nombre, "es")),
    equipos: estado.dispositivos
      .filter((equipo) => !equipo.revoked_at && !equipo.oculto_en_panel)
      .sort((a, b) => a.etiqueta.localeCompare(b.etiqueta, "es")),
    ordenes,
    cargadoEn: new Date(),
  };
}

/** "18/10/2026 14:32", en la hora del navegador. */
function textoFecha(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

const TEXTO_TIPO: Record<Dispositivo["tipo"], string> = { pc: "PC", mobile: "Celular", visor: "Visor" };

/** Una línea por equipo: qué mide ahora y hasta cuándo. */
function FilaEquipo({
  equipo,
  orden,
  ahora,
  onCambiar,
}: {
  equipo: Dispositivo;
  orden: OrdenTelemetria | undefined;
  ahora: Date;
  onCambiar: () => void;
}) {
  const vencida = orden !== undefined && ordenVencida(orden, ahora);
  return (
    <li className="flex flex-wrap items-center justify-between gap-2 rounded border border-(--borde) p-2">
      <div className="flex min-w-0 flex-col">
        <span className="font-medium">
          {equipo.etiqueta} <span className="text-sm font-normal text-(--muted)">· {TEXTO_TIPO[equipo.tipo]}</span>
        </span>
        <span className="text-sm">{textoGrupos(gruposVigentes(orden, ahora))}</span>
        <span className="text-sm text-(--muted)">
          {!orden
            ? "Sin orden: lo de siempre"
            : vencida && orden.vence_en
              ? `La orden venció el ${textoFecha(orden.vence_en)}: volvió a lo de siempre`
              : orden.vence_en
                ? `Hasta el ${textoFecha(orden.vence_en)}`
                : "Sin vencimiento"}
          {orden?.nota ? ` · ${orden.nota}` : ""}
        </span>
      </div>
      <button type="button" className="boton" onClick={onCambiar}>
        Cambiar
      </button>
    </li>
  );
}

/** Formulario de la orden de un equipo. Arranca con lo que mide hoy. */
function EditarOrden({
  equipo,
  orden,
  ahora,
  onCerrar,
  onGuardada,
}: {
  equipo: Dispositivo;
  orden: OrdenTelemetria | undefined;
  ahora: Date;
  onCerrar: () => void;
  onGuardada: () => void;
}) {
  const vigente = orden !== undefined && !ordenVencida(orden, ahora);
  // Sin orden o con la orden vencida, el equipo ya mide lo de siempre.
  const yaEsLoDeSiempre = !vigente || (esLoDeSiempre(orden.grupos) && orden.vence_en === null);
  const [marcados, setMarcados] = useState<Set<GrupoTelemetria>>(() => new Set(gruposVigentes(orden, ahora)));
  // "mantener" sólo si la orden vigente tiene vencimiento: conservarlo en vez
  // de alargarlo sin querer al cambiar un grupo.
  const puedeMantener = vigente && orden.vence_en !== null;
  const [vencimiento, setVencimiento] = useState<string>(puedeMantener ? "mantener" : "168");
  const [nota, setNota] = useState(vigente ? orden.nota : "");
  const [guardando, setGuardando] = useState(false);

  const grupos = ordenarGrupos(marcados, GRUPOS_TELEMETRIA);
  const horas = vencimiento === "mantener" || vencimiento === "nunca" ? null : Number(vencimiento);
  const sinVencimiento = vencimiento === "nunca";

  async function guardar(datos: { grupos: readonly GrupoTelemetria[]; vence_en: string | null; nota: string }) {
    setGuardando(true);
    try {
      await guardarOrdenTelemetria({ dispositivo_id: equipo.id, ...datos });
      toast.success(`Orden guardada para ${equipo.etiqueta}: ${textoGrupos(datos.grupos)}.`);
      onGuardada();
    } catch (fallo) {
      toast.error(mensajeError(fallo));
      setGuardando(false);
    }
  }

  function alGuardar() {
    const venceEn = vencimiento === "mantener" && orden ? orden.vence_en : venceEnDesde(horas, new Date());
    void guardar({ grupos, vence_en: venceEn, nota: nota.trim() });
  }

  function alternar(grupo: GrupoTelemetria) {
    setMarcados((actual) => {
      const siguiente = new Set(actual);
      if (siguiente.has(grupo)) siguiente.delete(grupo);
      else siguiente.add(grupo);
      return siguiente;
    });
  }

  return (
    <Modal titulo={`Telemetría de ${equipo.etiqueta}`} onCerrar={() => !guardando && onCerrar()}>
      <div className="flex flex-col gap-3">
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1 text-sm text-(--muted)">
            Estudios (base va siempre: equipos, versiones y caídas)
          </legend>
          {GRUPOS_TELEMETRIA.map((grupo) => (
            <label key={grupo} className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={marcados.has(grupo)}
                disabled={guardando}
                onChange={() => alternar(grupo)}
              />
              {DESCRIPCION_GRUPO[grupo]}
            </label>
          ))}
        </fieldset>

        <label className="campo">
          Vence en
          <select value={vencimiento} disabled={guardando} onChange={(evento) => setVencimiento(evento.target.value)}>
            {puedeMantener && orden.vence_en && (
              <option value="mantener">Mantener (hasta el {textoFecha(orden.vence_en)})</option>
            )}
            {OPCIONES_VENCIMIENTO.map((opcion) => (
              <option key={opcion.etiqueta} value={opcion.horas === null ? "nunca" : String(opcion.horas)}>
                {opcion.etiqueta}
              </option>
            ))}
          </select>
        </label>

        {marcados.has("depuracion") && sinVencimiento && (
          <p className="flex items-center gap-2 text-sm text-error">
            <TriangleAlert size={16} aria-hidden="true" />
            Depuración sin vencimiento hace más lento al equipo hasta que alguien la apague.
          </p>
        )}

        <label className="campo">
          <span className="flex justify-between">
            <span>Nota</span>
            <span aria-live="polite">
              {nota.length}/{LARGO_MAXIMO_NOTA}
            </span>
          </span>
          <input
            value={nota}
            maxLength={LARGO_MAXIMO_NOTA}
            disabled={guardando}
            placeholder="Ej.: el cliente dice que se traba al escanear"
            onChange={(evento) => setNota(evento.target.value)}
          />
        </label>

        <p className="text-sm text-(--muted)">
          Al vencer, el equipo vuelve solo a lo de siempre ({textoGrupos(GRUPOS_POR_DEFECTO)}), aunque no tenga red.
        </p>

        <div className="flex flex-wrap justify-between gap-2">
          <button
            type="button"
            className="boton boton-icono"
            disabled={guardando || yaEsLoDeSiempre}
            title="Lo que mide un equipo sin orden, sin vencimiento"
            onClick={() => void guardar({ grupos: GRUPOS_POR_DEFECTO, vence_en: null, nota: "" })}
          >
            <RotateCcw size={16} aria-hidden="true" />
            Volver a lo de siempre
          </button>
          <div className="flex gap-2">
            <button type="button" className="boton" disabled={guardando} onClick={onCerrar}>
              Cancelar
            </button>
            <button type="button" className="boton boton-primario" disabled={guardando} onClick={alGuardar}>
              {guardando ? "Guardando…" : "Guardar"}
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
}

/** Quién cambió qué y cuándo: para cruzar un cambio en los datos con la orden
 * que lo causó. */
function HistorialCambios({ cambios, cargando }: { cambios: CambioOrdenTelemetria[] | undefined; cargando: boolean }) {
  return (
    <section className="tarjeta flex flex-col gap-3 p-4">
      <h2 className="text-base font-semibold">Cambios</h2>
      {cargando && <p className="text-sm text-(--muted)">Cargando historial…</p>}
      {cambios?.length === 0 && <p className="text-sm text-(--muted)">Todavía no se cambió ninguna orden.</p>}
      {cambios && cambios.length > 0 && (
        <ul className="flex flex-col gap-2">
          {cambios.map((cambio) => (
            <li key={cambio.id} className="flex flex-col gap-1 rounded border border-(--borde) p-3">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <span className="font-medium">{cambio.etiqueta ?? "Equipo que ya no existe"}</span>
                <span className="text-sm text-(--muted)">{textoFecha(cambio.cambiado_en)}</span>
              </div>
              <p className="text-sm">
                {textoGrupos(cambio.grupos)} ·{" "}
                {cambio.vence_en ? `hasta el ${textoFecha(cambio.vence_en)}` : "sin vencimiento"}
              </p>
              <p className="text-sm text-(--muted)">
                Versión {cambio.version} · {cambio.cambiado_por}
                {cambio.nota ? ` · ${cambio.nota}` : ""}
              </p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/**
 * Estudios de telemetría por equipo (fase 3 del plan de telemetría). El
 * administrador enciende y apaga grupos de mediciones en cada equipo, con
 * vencimiento y nota. La nube le avisa al equipo al instante por Realtime, y
 * el equipo además la lee al sincronizar si estaba apagado o sin red.
 *
 * Qué versión de la orden aplicó cada equipo no se ve acá: lo informa el
 * propio equipo en su evento `orden_telemetria`, que va a la telemetría de
 * staging (Grafana), no a esta base.
 */
export default function Telemetria() {
  // Sin Realtime: las órdenes sólo las escribe este panel, que recarga al guardar.
  const { datos, cargando, recargar } = useLista(["telemetria-equipos"], cargarDatos, {
    intervaloMs: 60_000,
    tablas: "",
  });
  const historial = useLista(["telemetria-cambios"], listarCambiosTelemetria, { intervaloMs: 60_000, tablas: "" });
  const [editando, setEditando] = useState<string | null>(null);
  const [busqueda, setBusqueda] = useState("");

  const equipoEditado = datos?.equipos.find((equipo) => equipo.id === editando);
  const visibles = useMemo(() => {
    if (!datos) return [];
    const nombreSitio = new Map(datos.sitios.map((sitio) => [sitio.id, sitio.nombre]));
    return datos.equipos.filter((equipo) =>
      coincideBusqueda([equipo.etiqueta, nombreSitio.get(equipo.sitio_id) ?? "", TEXTO_TIPO[equipo.tipo]], busqueda),
    );
  }, [datos, busqueda]);

  return (
    <div className="flex h-full flex-col overflow-auto">
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 p-4">
        <section className="tarjeta flex flex-col gap-3 p-4">
          <h2 className="flex items-center gap-2 text-base font-semibold">
            <Activity size={18} aria-hidden="true" />
            Estudios por equipo
          </h2>
          <p className="text-sm text-(--muted)">
            Sin orden, cada equipo mide {textoGrupos(GRUPOS_POR_DEFECTO)}. Los errores se reportan siempre y no se
            controlan desde acá.
          </p>

          <label className="campo">
            <span className="sr-only">Buscar equipo</span>
            <span className="relative flex items-center">
              <Search size={16} className="pointer-events-none absolute left-2 text-(--muted)" aria-hidden="true" />
              <input
                type="search"
                className="w-full"
                // `.campo input` (sin capa) le gana a una clase de Tailwind.
                style={{ paddingLeft: "2rem" }}
                value={busqueda}
                placeholder="Buscar por equipo, unidad o tipo"
                onChange={(evento) => setBusqueda(evento.target.value)}
              />
            </span>
          </label>

          {cargando && <p className="text-sm text-(--muted)">Cargando equipos…</p>}

          {datos?.sitios.map((sitio) => {
            const delSitio = visibles.filter((equipo) => equipo.sitio_id === sitio.id);
            if (delSitio.length === 0) return null;
            return (
              <div key={sitio.id} className="flex flex-col gap-2">
                <h3 className="font-medium">{sitio.nombre}</h3>
                <ul className="flex flex-col gap-2">
                  {delSitio.map((equipo) => (
                    <FilaEquipo
                      key={equipo.id}
                      equipo={equipo}
                      orden={datos.ordenes.get(equipo.id)}
                      ahora={datos.cargadoEn}
                      onCambiar={() => setEditando(equipo.id)}
                    />
                  ))}
                </ul>
              </div>
            );
          })}
          {datos?.equipos.length === 0 && <p className="text-sm text-(--muted)">No hay equipos vigentes.</p>}
          {datos && datos.equipos.length > 0 && visibles.length === 0 && (
            <p className="text-sm text-(--muted)">Ningún equipo coincide con la búsqueda.</p>
          )}
        </section>

        <HistorialCambios cambios={historial.datos} cargando={historial.cargando} />
      </div>

      {datos && equipoEditado && (
        <EditarOrden
          equipo={equipoEditado}
          orden={datos.ordenes.get(equipoEditado.id)}
          ahora={datos.cargadoEn}
          onCerrar={() => setEditando(null)}
          onGuardada={() => {
            setEditando(null);
            void recargar();
            void historial.recargar();
          }}
        />
      )}
    </div>
  );
}
