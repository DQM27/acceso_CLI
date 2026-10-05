import { useEffect, useRef, useState } from "react";
import {
  mensajeBloqueoVisita,
  puedeContinuarVisita,
  RechazoVisita,
  registrarEntradaVisita,
  verificarCheckInVisita,
} from "../api";
import type { PreparacionVisita } from "../api";
import { validarGafeteOpcional } from "./VisitaCheckInModal.logica";

type Estado =
  | { tipo: "buscando" }
  | { tipo: "verificando" }
  | { tipo: "encontrada"; preparacion: PreparacionVisita }
  | { tipo: "bloqueada"; mensaje: string }
  /** El núcleo no deja seguir: sin cita, cita cancelada o vencida, acceso
   * negado, o la persona ya está adentro. Su mensaje se muestra tal cual. */
  | { tipo: "rechazada"; mensaje: string; alternativaPorCorreo: boolean }
  /** La visita existe pero es para otro día: aviso, no error. */
  | { tipo: "informativa"; mensaje: string; alternativaPorCorreo: boolean };

/**
 * Check-in de una visita agendada, por cédula -- lo usa `NuevaVisitaModal`
 * en el modo "Agendada". Mismo armazón que `NuevoIngresoModal` (buscador
 * arriba, panel de confirmación que se expande debajo, no se cierra solo al
 * registrar), pero sin lista flotante: la cédula es exacta.
 *
 * `cedulaInicial` (desde "Esperadas") la verifica apenas se abre: el guarda
 * igual compara el documento, elige el gafete y confirma. Si no hay cita
 * para hoy y nada lo impide, ofrece `onPorCorreo` con la cédula escrita.
 */
export function CheckInAgendada({
  cedulaInicial = "",
  onRegistrado,
  onPorCorreo,
}: {
  cedulaInicial?: string;
  onRegistrado: () => void;
  onPorCorreo?: (cedula: string) => void;
}) {
  const [cedula, setCedula] = useState(cedulaInicial);
  const [estado, setEstado] = useState<Estado>({ tipo: "buscando" });
  const [gafeteTexto, setGafeteTexto] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [mensaje, setMensaje] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const cedulaRef = useRef<HTMLInputElement>(null);
  const confirmarRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (estado.tipo === "encontrada") {
      confirmarRef.current?.focus();
    }
  }, [estado]);

  // Desde "Esperadas" la cédula ya viene: se verifica sola al abrir.
  const verificoInicial = useRef(false);
  useEffect(() => {
    if (!cedulaInicial || verificoInicial.current) return;
    verificoInicial.current = true;
    void verificar(cedulaInicial);
    // Sólo al montar: `verificar` cambia en cada render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function cambiarCedula(texto: string) {
    setCedula(texto);
    setMensaje(null);
    if (estado.tipo !== "buscando") {
      setEstado({ tipo: "buscando" });
    }
  }

  async function verificar(texto = cedula) {
    const valor = texto.trim();
    if (!valor) return;
    setError(null);
    setMensaje(null);
    setGafeteTexto("");
    setEstado({ tipo: "verificando" });
    try {
      // "¿Ya está adentro?" lo revisa el núcleo acá mismo: si ya entró,
      // `verificarCheckInVisita` falla con su mensaje (cae en el catch).
      const preparacion = await verificarCheckInVisita(valor);
      if (!puedeContinuarVisita(preparacion)) {
        setEstado({ tipo: "bloqueada", mensaje: mensajeBloqueoVisita(preparacion) });
        return;
      }
      setEstado({ tipo: "encontrada", preparacion });
    } catch (error) {
      const rechazo = error instanceof RechazoVisita ? error : null;
      setEstado({
        tipo: rechazo?.informativo ? "informativa" : "rechazada",
        mensaje: String(error),
        alternativaPorCorreo: rechazo?.alternativaPorCorreo ?? false,
      });
    }
  }

  async function confirmarEntrada() {
    if (estado.tipo !== "encontrada") return;
    const resultado = validarGafeteOpcional(gafeteTexto);
    if (!resultado.valido) {
      setError(resultado.mensaje);
      return;
    }
    setError(null);
    setEnviando(true);
    try {
      await registrarEntradaVisita(estado.preparacion.visitante.cedula, resultado.numero);
      setMensaje(`✓ Entrada registrada — ${estado.preparacion.visitante.nombre}`);
      setEstado({ tipo: "buscando" });
      setCedula("");
      cedulaRef.current?.focus();
      onRegistrado();
    } catch (error) {
      setError(String(error));
    } finally {
      setEnviando(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
      <form
        onSubmit={(evento) => {
          evento.preventDefault();
          verificar();
        }}
        style={{ display: "flex", gap: "0.75rem", alignItems: "flex-end" }}
      >
        <label className="campo" style={{ flex: 1 }}>
          Cédula del visitante
          <input
            ref={cedulaRef}
            value={cedula}
            onChange={(evento) => cambiarCedula(evento.target.value)}
            autoFocus
            placeholder="Escanee o escriba la cédula…"
          />
        </label>
        <button
          type="submit"
          className="boton boton-primario"
          disabled={estado.tipo === "verificando" || !cedula.trim()}
        >
          {estado.tipo === "verificando" ? "Verificando…" : "Verificar"}
        </button>
      </form>

      {mensaje && <p style={{ color: "var(--exito)", margin: 0 }}>{mensaje}</p>}

      {estado.tipo === "rechazada" && (
        <p className="login-error" role="alert">
          {estado.mensaje}
        </p>
      )}

      {estado.tipo === "informativa" && (
        <p className="aviso-informativo" role="status">
          {estado.mensaje}
        </p>
      )}

      {(estado.tipo === "rechazada" || estado.tipo === "informativa") &&
        estado.alternativaPorCorreo &&
        onPorCorreo && (
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button type="button" className="boton" onClick={() => onPorCorreo(cedula.trim())}>
              Registrar como autorizada por correo
            </button>
          </div>
        )}

      {estado.tipo === "bloqueada" && (
        <p className="login-error" role="alert">
          {estado.mensaje}
        </p>
      )}

      {estado.tipo === "encontrada" && (
        <form
          onSubmit={(evento) => {
            evento.preventDefault();
            confirmarEntrada();
          }}
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "0.9rem",
            padding: "0.85rem",
            border: "1px solid var(--borde)",
            borderRadius: "var(--radio-chico)",
            background: "var(--campo-fondo)",
          }}
        >
          <div>
            <p style={{ margin: 0, fontWeight: 600, color: "var(--texto)" }}>
              {estado.preparacion.visitante.nombre}
            </p>
            <p style={{ margin: "0.15rem 0 0", color: "var(--muted)", fontSize: "0.85rem" }}>
              {estado.preparacion.visitante.cedula}
              {estado.preparacion.visitante.empresa && ` · ${estado.preparacion.visitante.empresa}`}
              {" · Anfitrión: "}
              {estado.preparacion.cita.anfitrion_nombre}
              {estado.preparacion.cita.motivo && ` · ${estado.preparacion.cita.motivo}`}
              {estado.preparacion.cita.hora_estimada &&
                ` · Llegada estimada: ${estado.preparacion.cita.hora_estimada.slice(0, 5)}`}
            </p>
          </div>

          <label className="campo">
            Número de gafete (opcional)
            <input
              value={gafeteTexto}
              onChange={(evento) => setGafeteTexto(evento.target.value.replace(/\D/g, ""))}
              inputMode="numeric"
              autoFocus
              placeholder="S/G si vacío"
            />
          </label>

          {error && (
            <p className="login-error" role="alert">
              {error}
            </p>
          )}

          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button ref={confirmarRef} type="submit" className="boton boton-primario" disabled={enviando}>
              {enviando ? "Registrando…" : "Registrar entrada"}
            </button>
          </div>
        </form>
      )}
    </div>
  );
}
