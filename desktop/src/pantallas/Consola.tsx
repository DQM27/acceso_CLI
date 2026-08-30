import { useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent, KeyboardEvent, MouseEvent as ReactMouseEvent, ReactNode } from "react";
import type { Seccion } from "../App";
import FormularioContratista from "./FormularioContratista";
import FormularioEmpresa from "./FormularioEmpresa";
import FormularioUsuario from "./FormularioUsuario";
import {
  autocompletarComando,
  cambiarMiPassword,
  ejecutarComando,
  etiquetaCampo,
  etiquetaEntidad,
  gafetesDe,
  listarEmpresas,
  listarIngresosActivos,
  mensajeMotivoDenegacion,
  prepararIngreso,
  puedeContinuar,
  registrarIngreso,
  registrarSalida,
  sanearGafetes,
  valorPresentable,
} from "../api";
import type {
  CambioAuditado,
  ContextState,
  ContratistaResumen,
  Empresa,
  IngresoActivoResumen,
  MedioIngreso,
  PreparacionIngreso,
  RolUsuario,
} from "../api";

type Linea =
  | { tipo: "entrada"; texto: string }
  | { tipo: "salida"; id: number; contenido: ReactNode };

/** `/nuevo [contratista|empresa|usuario]` abre el formulario correspondiente
 * directo desde la consola, en vez de una tarjeta "Enter para abrir" — un
 * único paso, igual que cualquier otro comando de la consola. */
type ModalNuevo = "contratista" | "empresa" | "usuario" | null;

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

export default function Consola({
  actorRol,
  onNavegar,
  onCerrarSesion,
}: {
  actorRol: RolUsuario;
  onNavegar: (seccion: Seccion) => void;
  onCerrarSesion: () => void;
}) {
  const [abierta, setAbierta] = useState(false);
  const [historial, setHistorial] = useState<Linea[]>([]);
  const [texto, setTexto] = useState("");
  const [modoGafete, setModoGafete] = useState<IngresoActivoResumen[] | null>(null);
  const [sugerencias, setSugerencias] = useState<string[]>([]);
  const [completado, setCompletado] = useState<string | null>(null);
  const [tamano, setTamano] = useState(TAMANO_INICIAL);
  const [modalNuevo, setModalNuevo] = useState<ModalNuevo>(null);
  const [empresasParaFormulario, setEmpresasParaFormulario] = useState<Empresa[]>([]);
  // Historial de comandos confirmados (no de gafetes) para navegar con
  // ↑/↓, como cualquier terminal. `indiceHistorial` es `null` mientras se
  // escribe en punta (sin navegar); `borradorRef` guarda lo que había en el
  // input antes de la primera flecha, para devolverlo al pasarse de largo.
  const [historialComandos, setHistorialComandos] = useState<string[]>([]);
  const [indiceHistorial, setIndiceHistorial] = useState<number | null>(null);
  const borradorRef = useRef("");
  const contadorRef = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (abierta) inputRef.current?.focus();
  }, [abierta]);

  useEffect(() => {
    const prefiereMenosMovimiento = window.matchMedia(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: prefiereMenosMovimiento ? "auto" : "smooth",
    });
  }, [historial]);

  // Autocompletado en vivo, mismo par sugerencias/Tab que ya tiene
  // `--comandos` en cada tecla — no aplica en modo gafete (ahí el texto es
  // sólo números, no hay nada que sugerir). Con debounce: sin esto, cada
  // tecla dispara un invoke() completo a Rust en el acto — al escribir
  // rápido se siente como micro-tirones en vez de fluido.
  useEffect(() => {
    if (modoGafete || !texto.trim()) {
      setSugerencias([]);
      setCompletado(null);
      return;
    }
    let vigente = true;
    const temporizador = window.setTimeout(() => {
      autocompletarComando(texto).then((r) => {
        if (!vigente) return;
        setSugerencias(r.sugerencias);
        setCompletado(r.completado);
      });
    }, 120);
    return () => {
      vigente = false;
      window.clearTimeout(temporizador);
    };
  }, [texto, modoGafete]);

  function historialAnterior() {
    if (historialComandos.length === 0) return;
    if (indiceHistorial === null) borradorRef.current = texto;
    const nuevo =
      indiceHistorial === null
        ? historialComandos.length - 1
        : Math.max(0, indiceHistorial - 1);
    setIndiceHistorial(nuevo);
    setTexto(historialComandos[nuevo]);
  }

  function historialSiguiente() {
    if (indiceHistorial === null) return;
    const nuevo = indiceHistorial + 1;
    if (nuevo >= historialComandos.length) {
      setIndiceHistorial(null);
      setTexto(borradorRef.current);
      return;
    }
    setIndiceHistorial(nuevo);
    setTexto(historialComandos[nuevo]);
  }

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

  /** Abre el formulario de alta correspondiente y cierra la consola — el
   * formulario es un `Modal` propio (z-index 100) y la consola flota por
   * encima (z-index 200); dejar ambos abiertos los superpone en el mismo
   * centro de pantalla. El historial de la consola no se pierde, sigue ahí
   * para cuando se vuelva a abrir. */
  async function abrirNuevo(tipo: "contratista" | "empresa" | "usuario") {
    if (tipo === "contratista") {
      setEmpresasParaFormulario(await listarEmpresas());
    }
    setModalNuevo(tipo);
    setAbierta(false);
  }

  async function ejecutar(entrada: string) {
    try {
      const resultado = await ejecutarComando(entrada);
      if (typeof resultado === "object" && "AbrirSalidaGafete" in resultado) {
        await entrarModoGafete(resultado.AbrirSalidaGafete.texto);
        return;
      }
      if (resultado === "AbrirHistorial") {
        onNavegar("historial");
        setAbierta(false);
        return;
      }
      if (resultado === "NuevoContratista") {
        await abrirNuevo("contratista");
        return;
      }
      if (resultado === "NuevoEmpresa") {
        await abrirNuevo("empresa");
        return;
      }
      if (resultado === "NuevoUsuario") {
        await abrirNuevo("usuario");
        return;
      }
      agregar(<RenderContextState resultado={resultado} onCerrarSesion={onCerrarSesion} />);
    } catch (error) {
      agregar(<span style={{ color: "var(--c-error)" }}>{String(error)}</span>);
    }
  }

  function cambiarTexto(valor: string) {
    setIndiceHistorial(null);
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
      return;
    }
    if (evento.key === "ArrowUp" && !modoGafete) {
      evento.preventDefault();
      historialAnterior();
      return;
    }
    if (evento.key === "ArrowDown" && !modoGafete) {
      evento.preventDefault();
      historialSiguiente();
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
    setHistorialComandos((actual) =>
      actual[actual.length - 1] === valor ? actual : [...actual, valor],
    );
    setIndiceHistorial(null);
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

      {modalNuevo === "contratista" && (
        <FormularioContratista
          empresas={empresasParaFormulario}
          onCerrar={() => setModalNuevo(null)}
          onGuardado={() => {
            setModalNuevo(null);
            agregar(<span style={{ color: "var(--c-exito)" }}>✓ Contratista creado.</span>);
          }}
        />
      )}
      {modalNuevo === "empresa" && (
        <FormularioEmpresa
          onCerrar={() => setModalNuevo(null)}
          onGuardado={() => {
            setModalNuevo(null);
            agregar(<span style={{ color: "var(--c-exito)" }}>✓ Empresa creada.</span>);
          }}
        />
      )}
      {modalNuevo === "usuario" && (
        <FormularioUsuario
          actorRol={actorRol}
          onCerrar={() => setModalNuevo(null)}
          onGuardado={() => {
            setModalNuevo(null);
            agregar(<span style={{ color: "var(--c-exito)" }}>✓ Usuario creado.</span>);
          }}
        />
      )}
    </>
  );
}

function RenderContextState({
  resultado,
  onCerrarSesion,
}: {
  resultado: ContextState;
  onCerrarSesion: () => void;
}) {
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
      case "ConfirmarCerrarSesion":
        return <ConfirmarCerrarSesionCard onConfirmar={onCerrarSesion} />;
      case "ConfirmarCambioPassword":
        return <CambiarPasswordCard />;
      case "ConfirmarModoClasico":
        return (
          <span style={{ color: "var(--c-muted)" }}>
            La interfaz clásica es de la consola de Windows — no aplica en esta ventana. Cerrá
            esta app y abrí <code>control_acceso.exe --tui-clasica</code> si la necesitás.
          </span>
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

  if ("TablaAuditoria" in resultado) {
    // El rol ya lo verificó el resolver (Operacion::VerAuditoria) — si
    // llegó hasta acá, el actor está autorizado; no hace falta repetir el
    // chequeo del lado del cliente. `items`/`total` vienen ya paginados del
    // resolver, igual que TablaActivos arriba — sin un segundo fetch.
    const { items, total } = resultado.TablaAuditoria;
    return <TablaAuditoriaSalida items={items} total={total} />;
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
  const [error, setError] = useState<string | null>(null);

  async function salida(id: number) {
    setError(null);
    try {
      await registrarSalida(id);
      setOcultos((actual) => new Set(actual).add(id));
    } catch (error) {
      setError(String(error));
    }
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
              {a.contratista_nombre} · {a.empresa_nombre} · gafete{" "}
              {a.gafete_numero !== null ? a.gafete_numero : "S/G"}
            </span>
          </div>
        ))}
      <p style={{ margin: "0.15rem 0 0", color: "var(--c-muted)" }}>{total} adentro.</p>
      {error && <p style={{ color: "var(--c-error)", margin: "0.15rem 0 0" }}>{error}</p>}
    </div>
  );
}

/** `/auditoria` — `items`/`total` ya vienen paginados del resolver (ver
 * arriba), en líneas compactas en vez de la grilla AG Grid completa de la
 * pantalla Auditoría; mismas etiquetas (`etiquetaEntidad`/`etiquetaCampo`/
 * `valorPresentable`) para no inventar una traducción distinta. */
function TablaAuditoriaSalida({ items, total }: { items: CambioAuditado[]; total: number }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.15rem" }}>
      {items.map((item) => (
        <div key={item.id} style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
          <span style={{ color: "var(--c-muted)" }}>
            {new Date(item.fecha_hora).toLocaleString("es-CR", {
              dateStyle: "short",
              timeStyle: "short",
            })}
          </span>
          <span>{item.usuario_nombre}</span>
          <span style={{ color: "var(--c-muted)" }}>
            {etiquetaEntidad(item.entidad)} "{item.entidad_nombre}" · {etiquetaCampo(item.campo)}
          </span>
          <span>
            {valorPresentable(item.campo, item.valor_anterior)} →{" "}
            {valorPresentable(item.campo, item.valor_nuevo)}
          </span>
        </div>
      ))}
      <p style={{ margin: "0.15rem 0 0", color: "var(--c-muted)" }}>{total} cambio(s).</p>
    </div>
  );
}

/** `/cerrarsesion` — confirmación explícita con clic, mismo criterio que el
 * resto de la consola (botones en vez del ciclo Enter/Esc del resolver). */
function ConfirmarCerrarSesionCard({ onConfirmar }: { onConfirmar: () => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
      <span>¿Cerrar la sesión actual?</span>
      <button type="button" className="boton boton-primario" onClick={onConfirmar}>
        Cerrar sesión
      </button>
    </div>
  );
}

/** `/clave` — cambio de la propia contraseña en un solo paso. El núcleo ya
 * verifica `passwordActual` y valida la nueva en la misma llamada
 * (`AppCore::cambiar_mi_password`), así que a diferencia de la TUI no hace
 * falta un paso de verificación aparte antes de pedir la nueva. */
function CambiarPasswordCard() {
  const [passwordActual, setPasswordActual] = useState("");
  const [nuevaPassword, setNuevaPassword] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function confirmar() {
    if (nuevaPassword.length < 8) {
      setError("La contraseña debe tener al menos 8 caracteres");
      return;
    }
    setEnviando(true);
    setError(null);
    try {
      await cambiarMiPassword(passwordActual, nuevaPassword);
      setMensaje("✓ Contraseña actualizada.");
      setPasswordActual("");
      setNuevaPassword("");
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  if (mensaje) {
    return <span style={{ color: "var(--c-exito)" }}>{mensaje}</span>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem", maxWidth: "20rem" }}>
      <input
        type="password"
        value={passwordActual}
        onChange={(evento) => setPasswordActual(evento.target.value)}
        placeholder="Contraseña actual"
        className="consola-input-chico"
        autoComplete="current-password"
      />
      <input
        type="password"
        value={nuevaPassword}
        onChange={(evento) => setNuevaPassword(evento.target.value)}
        placeholder="Contraseña nueva (mínimo 8 caracteres)"
        className="consola-input-chico"
        autoComplete="new-password"
      />
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
        <button
          type="button"
          className="boton boton-primario"
          disabled={enviando || !passwordActual || !nuevaPassword}
          onClick={confirmar}
        >
          Cambiar contraseña
        </button>
        {error && <span style={{ color: "var(--c-error)" }}>{error}</span>}
      </div>
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
    try {
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
    } catch (error) {
      setError(String(error));
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
