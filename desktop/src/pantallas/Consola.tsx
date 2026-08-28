import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode } from "react";
import {
  autocompletarComando,
  ejecutarComando,
  gafetesDe,
  listarIngresosActivos,
  mensajeMotivoDenegacion,
  prepararIngreso,
  puedeContinuar,
  registrarIngreso,
  registrarSalida,
  sanearGafetes,
} from "../api";
import type {
  ContextState,
  ContratistaResumen,
  IngresoActivoResumen,
  MedioIngreso,
  PreparacionIngreso,
} from "../api";

type Linea =
  | { tipo: "entrada"; texto: string }
  | { tipo: "salida"; id: number; contenido: ReactNode };

/**
 * Piloto de consola tipo terminal — otro lenguaje para hacer lo mismo que
 * ya hacen las pantallas normales, pensado para quien escribe más rápido
 * de lo que hace clic y prefiere comandos. Reusa el mismo parser+resolver
 * de `--comandos` (`ejecutarComando`, ver `src/application/comandos.rs`)
 * para las 12 líneas de comando existentes; sólo el render y el modo
 * enclavado de `/gafete` están reimplementados acá en React — el resto
 * del lenguaje (`/ingreso`, `/salida`, `/activos`, `/ayuda`, etc.) llega
 * gratis porque el cálculo ya era independiente de la terminal real.
 */
const TAMANO_INICIAL = { ancho: 900, alto: 620 };
const TAMANO_MINIMO = { ancho: 480, alto: 320 };

export default function Consola() {
  const [abierta, setAbierta] = useState(false);
  const [historial, setHistorial] = useState<Linea[]>([]);
  const [texto, setTexto] = useState("");
  const [modoGafete, setModoGafete] = useState<IngresoActivoResumen[] | null>(null);
  const [sugerencias, setSugerencias] = useState<string[]>([]);
  const [completado, setCompletado] = useState<string | null>(null);
  const [tamano, setTamano] = useState(TAMANO_INICIAL);
  const contadorRef = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (abierta) inputRef.current?.focus();
  }, [abierta]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [historial]);

  // Autocompletado en vivo, mismo par sugerencias/Tab que ya tiene
  // `--comandos` en cada tecla — no aplica en modo gafete (ahí el texto es
  // sólo números, no hay nada que sugerir).
  useEffect(() => {
    if (modoGafete || !texto.trim()) {
      setSugerencias([]);
      setCompletado(null);
      return;
    }
    let vigente = true;
    autocompletarComando(texto).then((r) => {
      if (!vigente) return;
      setSugerencias(r.sugerencias);
      setCompletado(r.completado);
    });
    return () => {
      vigente = false;
    };
  }, [texto, modoGafete]);

  function alArrastrarBorde(eventoInicial: ReactMouseEvent) {
    eventoInicial.preventDefault();
    const inicio = { x: eventoInicial.clientX, y: eventoInicial.clientY, ...tamano };
    function mover(evento: MouseEvent) {
      setTamano({
        ancho: Math.max(TAMANO_MINIMO.ancho, inicio.ancho + (evento.clientX - inicio.x)),
        alto: Math.max(TAMANO_MINIMO.alto, inicio.alto + (evento.clientY - inicio.y)),
      });
    }
    function soltar() {
      window.removeEventListener("mousemove", mover);
      window.removeEventListener("mouseup", soltar);
    }
    window.addEventListener("mousemove", mover);
    window.addEventListener("mouseup", soltar);
  }

  function agregar(contenido: ReactNode) {
    contadorRef.current += 1;
    setHistorial((actual) => [...actual, { tipo: "salida", id: contadorRef.current, contenido }]);
  }

  async function entrarModoGafete(textoInicial: string) {
    const pagina = await listarIngresosActivos();
    setModoGafete(pagina.items);
    agregar(
      <span style={{ color: "var(--c-muted)" }}>
        Modo gafete — números separados por coma, Enter registra, Esc sale.
      </span>,
    );
    if (textoInicial.trim()) {
      await confirmarGafete(textoInicial, pagina.items);
    }
  }

  async function confirmarGafete(textoGafetes: string, activos: IngresoActivoResumen[]) {
    const numeros = gafetesDe(textoGafetes);
    const porGafete = new Map(
      activos.filter((a) => a.gafete_numero !== null).map((a) => [a.gafete_numero as number, a]),
    );
    const registrados: string[] = [];
    const fallidos: string[] = [];
    for (const numero of numeros) {
      const activo = porGafete.get(numero);
      if (!activo) {
        fallidos.push(`gafete ${numero}: sin ingreso activo`);
        continue;
      }
      try {
        await registrarSalida(activo.registro_id);
        registrados.push(activo.contratista_nombre);
      } catch (error) {
        fallidos.push(`gafete ${numero}: ${String(error)}`);
      }
    }
    if (registrados.length > 0) {
      agregar(<span style={{ color: "var(--c-exito)" }}>✓ Salida registrada — {registrados.join(", ")}</span>);
    }
    if (fallidos.length > 0) {
      agregar(<span style={{ color: "var(--c-error)" }}>{fallidos.join(" · ")}</span>);
    }
    const pagina = await listarIngresosActivos();
    setModoGafete(pagina.items);
  }

  async function ejecutar(entrada: string) {
    try {
      const resultado = await ejecutarComando(entrada);
      if (typeof resultado === "object" && "AbrirSalidaGafete" in resultado) {
        await entrarModoGafete(resultado.AbrirSalidaGafete.texto);
        return;
      }
      agregar(<RenderContextState resultado={resultado} />);
    } catch (error) {
      agregar(<span style={{ color: "var(--c-error)" }}>{String(error)}</span>);
    }
  }

  function cambiarTexto(valor: string) {
    setTexto(modoGafete ? sanearGafetes(valor) : valor);
  }

  function alTecla(evento: KeyboardEvent<HTMLInputElement>) {
    if (evento.key === "Escape" && modoGafete) {
      evento.preventDefault();
      setModoGafete(null);
      agregar(<span style={{ color: "var(--c-muted)" }}>Modo gafete cerrado.</span>);
      return;
    }
    if (evento.key === "Tab" && completado) {
      evento.preventDefault();
      setTexto(completado);
    }
  }

  async function alEnviar(evento: FormEvent) {
    evento.preventDefault();
    const valor = texto;
    setTexto("");
    if (modoGafete) {
      setHistorial((actual) => [
        ...actual,
        { tipo: "entrada", texto: `gafete> ${valor}` },
      ]);
      await confirmarGafete(valor, modoGafete);
      return;
    }
    if (!valor.trim()) return;
    setHistorial((actual) => [...actual, { tipo: "entrada", texto: valor }]);
    await ejecutar(valor);
  }

  const gafetesEnVivo = useMemo(
    () => (modoGafete ? gafetesDe(texto) : []),
    [modoGafete, texto],
  );
  const porGafeteEnVivo = useMemo(() => {
    const mapa = new Map<number, IngresoActivoResumen>();
    for (const a of modoGafete ?? []) {
      if (a.gafete_numero !== null) mapa.set(a.gafete_numero, a);
    }
    return mapa;
  }, [modoGafete]);

  return (
    <>
      <button
        type="button"
        className="consola-boton"
        title="Consola (piloto)"
        onClick={() => setAbierta((a) => !a)}
      >
        &gt;_
      </button>

      {abierta && (
        <div
          className="consola-ventana"
          style={{ width: `${tamano.ancho}px`, height: `${tamano.alto}px` }}
        >
          <div className="consola-encabezado">
            <span>&gt;_ Consola</span>
            <button type="button" className="boton" onClick={() => setAbierta(false)}>
              ✕
            </button>
          </div>

          <div className="consola-scroll" ref={scrollRef}>
            {historial.length === 0 && (
              <p style={{ color: "var(--c-muted)", margin: 0 }}>
                Piloto — probá <code>/ayuda</code>, <code>/activos</code>, <code>/gafete</code>, o
                escribí un nombre para buscar.
              </p>
            )}
            {historial.map((linea, indice) =>
              linea.tipo === "entrada" ? (
                <div key={indice} className="consola-linea-entrada">
                  <span style={{ color: "var(--c-acento)" }}>»</span> {linea.texto}
                </div>
              ) : (
                <div key={linea.id} className="consola-linea-salida">
                  {linea.contenido}
                </div>
              ),
            )}

            {modoGafete && gafetesEnVivo.length > 0 && (
              <div style={{ display: "flex", flexDirection: "column", gap: "0.15rem", marginTop: "0.3rem" }}>
                {gafetesEnVivo.map((numero, indice) => {
                  const activo = porGafeteEnVivo.get(numero);
                  return (
                    <div key={`${numero}-${indice}`} style={{ display: "flex", gap: "0.5rem" }}>
                      <span style={{ color: "var(--c-muted)" }}>gafete {numero}:</span>
                      {activo ? (
                        <span>{activo.contratista_nombre}</span>
                      ) : (
                        <span style={{ color: "var(--c-error)" }}>sin ingreso activo</span>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          {!modoGafete && (sugerencias.length > 0 || completado) && (
            <div className="consola-sugerencias">
              {completado && (
                <span>
                  <kbd>Tab</kbd> {completado.trim()}
                </span>
              )}
              {sugerencias.map((s, i) => (
                <span key={i}>{s}</span>
              ))}
            </div>
          )}

          <form onSubmit={alEnviar} className="consola-prompt">
            <span style={{ color: modoGafete ? "var(--c-advertencia)" : "var(--c-acento)" }}>
              {modoGafete ? "gafete>" : "»"}
            </span>
            <input
              ref={inputRef}
              value={texto}
              onChange={(evento) => cambiarTexto(evento.target.value)}
              onKeyDown={alTecla}
              placeholder={modoGafete ? "Ej. 2, 25, 85 (Esc para salir)" : "Escribí un comando… (/ayuda, Tab autocompleta)"}
              autoComplete="off"
            />
          </form>

          <div className="consola-resize" onMouseDown={alArrastrarBorde} title="Arrastrar para redimensionar" />
        </div>
      )}
    </>
  );
}

function RenderContextState({ resultado }: { resultado: ContextState }) {
  if (typeof resultado === "string") {
    switch (resultado) {
      case "Ayuda":
        return (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.15rem" }}>
            <p style={{ margin: 0 }}>Comandos: /ingreso /salida /gafete /activos /nuevo /editar</p>
            <p style={{ margin: 0 }}>/historial /auditoria /clave /clasico /cerrarsesion</p>
            <p style={{ margin: 0, color: "var(--c-muted)" }}>
              Escribir texto libre busca contratistas por nombre o cédula.
            </p>
          </div>
        );
      default:
        return (
          <span style={{ color: "var(--c-muted)" }}>
            No implementado todavía en la consola — usá la pantalla correspondiente.
          </span>
        );
    }
  }

  if ("Inicio" in resultado) {
    return <span style={{ color: "var(--c-muted)" }}>{resultado.Inicio.total_dentro} adentro ahora.</span>;
  }

  if ("MensajeError" in resultado) {
    return <span style={{ color: "var(--c-error)" }}>{resultado.MensajeError.mensaje}</span>;
  }

  if ("TablaActivos" in resultado) {
    const { items, total } = resultado.TablaActivos;
    return <TablaSalida items={items} total={total} />;
  }

  if ("CoincidenciasActivos" in resultado) {
    const { items, descripcion } = resultado.CoincidenciasActivos;
    if (items.length === 0) {
      return <span style={{ color: "var(--c-muted)" }}>Sin ingreso activo para "{descripcion}".</span>;
    }
    return <TablaSalida items={items} total={items.length} />;
  }

  if ("Coincidencias" in resultado) {
    const { items, consulta } = resultado.Coincidencias;
    if (items.length === 0) {
      return <span style={{ color: "var(--c-muted)" }}>Sin coincidencias para "{consulta}".</span>;
    }
    return <ListaIngreso items={items} />;
  }

  if ("FichaContratista" in resultado) {
    const r = resultado.FichaContratista.resumen;
    return (
      <span>
        {r.nombre} · {r.cedula} · {r.empresa_nombre}
      </span>
    );
  }

  if ("CoincidenciasEmpresas" in resultado) {
    const { items } = resultado.CoincidenciasEmpresas;
    return (
      <div>
        {items.map((e) => (
          <div key={e.id}>
            {e.nombre} · {e.contratistas} contratista(s)
          </div>
        ))}
      </div>
    );
  }

  if ("CoincidenciasUsuarios" in resultado) {
    const { items } = resultado.CoincidenciasUsuarios;
    return (
      <div>
        {items.map((u) => (
          <div key={u.id}>
            {u.nombre} · {u.cedula} · {u.rol}
          </div>
        ))}
      </div>
    );
  }

  return (
    <span style={{ color: "var(--c-muted)" }}>
      No implementado todavía en la consola — usá la pantalla correspondiente.
    </span>
  );
}

/** Fila con botón de salida directo — mismo criterio que Activos: la salida
 * individual no pide confirmación aparte, ya es un solo clic explícito. */
function TablaSalida({ items, total }: { items: IngresoActivoResumen[]; total: number }) {
  const [ocultos, setOcultos] = useState<Set<number>>(new Set());

  async function salida(id: number) {
    await registrarSalida(id);
    setOcultos((actual) => new Set(actual).add(id));
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.15rem" }}>
      {items
        .filter((a) => !ocultos.has(a.registro_id))
        .map((a) => (
          <div key={a.registro_id} style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            <button
              type="button"
              className="boton"
              style={{ padding: "0.05rem 0.5rem", fontSize: "0.78rem" }}
              onClick={() => salida(a.registro_id)}
            >
              Salida
            </button>
            <span>
              {a.contratista_nombre} · {a.empresa_nombre}
              {a.gafete_numero !== null && ` · gafete ${a.gafete_numero}`}
            </span>
          </div>
        ))}
      <p style={{ margin: "0.15rem 0 0", color: "var(--c-muted)" }}>{total} adentro.</p>
    </div>
  );
}

/** Fila de resultado de búsqueda para ingreso — clic arma la tarjeta de
 * medio/gafete inline, mismo flujo que NuevoIngresoModal pero como líneas
 * de la consola en vez de un panel de modal. */
function ListaIngreso({ items }: { items: ContratistaResumen[] }) {
  const [elegido, setElegido] = useState<ContratistaResumen | null>(null);
  const [preparacion, setPreparacion] = useState<PreparacionIngreso | null>(null);
  const [bloqueo, setBloqueo] = useState<string | null>(null);
  const [medio, setMedio] = useState<MedioIngreso>("Caminando");
  const [gafete, setGafete] = useState("");
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function elegir(contratista: ContratistaResumen) {
    setElegido(contratista);
    setPreparacion(null);
    setBloqueo(null);
    setError(null);
    const p = await prepararIngreso(contratista.id);
    if (puedeContinuar(p)) {
      setPreparacion(p);
      setMedio("Caminando");
      setGafete("");
    } else {
      setBloqueo(
        p.tiene_ingreso_activo
          ? "Ya tiene un ingreso activo."
          : typeof p.resultado_acceso === "object"
            ? mensajeMotivoDenegacion(p.resultado_acceso.Denegado)
            : "No se puede continuar.",
      );
    }
  }

  async function confirmar() {
    if (!preparacion) return;
    setError(null);
    try {
      const numero = preparacion.requiere_gafete ? Number.parseInt(gafete, 10) : null;
      if (preparacion.requiere_gafete && (!gafete.trim() || Number.isNaN(numero))) {
        setError("Ingrese un número de gafete válido");
        return;
      }
      await registrarIngreso(preparacion.contratista_id, medio, numero);
      setMensaje(`✓ Ingreso registrado — ${preparacion.nombre}`);
      setPreparacion(null);
      setElegido(null);
    } catch (error) {
      setError(String(error));
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.3rem" }}>
      {items.map((c) => (
        <button
          key={c.id}
          type="button"
          onClick={() => elegir(c)}
          style={{
            display: "block",
            textAlign: "left",
            background: "none",
            border: "none",
            color: elegido?.id === c.id ? "var(--c-acento)" : "var(--c-texto)",
            cursor: "pointer",
            padding: 0,
            font: "inherit",
          }}
        >
          {c.nombre} · {c.cedula} · {c.empresa_nombre}
        </button>
      ))}

      {bloqueo && <p style={{ color: "var(--c-error)", margin: "0.2rem 0 0" }}>{bloqueo}</p>}

      {preparacion && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.6rem",
            marginTop: "0.2rem",
            flexWrap: "wrap",
          }}
        >
          {(["Caminando", "Vehiculo"] as const).map((opcion) => (
            <label key={opcion} style={{ display: "flex", alignItems: "center", gap: "0.25rem" }}>
              <input type="radio" checked={medio === opcion} onChange={() => setMedio(opcion)} />
              {opcion}
            </label>
          ))}
          {preparacion.requiere_gafete && (
            <input
              value={gafete}
              onChange={(evento) => setGafete(evento.target.value.replace(/\D/g, ""))}
              placeholder="Gafete"
              style={{ width: "5rem" }}
              className="consola-input-chico"
            />
          )}
          <button type="button" className="boton boton-primario" onClick={confirmar}>
            Confirmar
          </button>
        </div>
      )}

      {mensaje && <p style={{ color: "var(--c-exito)", margin: "0.2rem 0 0" }}>{mensaje}</p>}
      {error && <p style={{ color: "var(--c-error)", margin: "0.2rem 0 0" }}>{error}</p>}
    </div>
  );
}
