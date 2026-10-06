import { useState } from "react";
import type { FormEvent } from "react";
import { CalendarDays, MapPin, UserCheck } from "lucide-react";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, BotonTema } from "../componentes/Comunes";
import { CampoContrasena } from "../componentes/CampoContrasena";
import { LONGITUD_MINIMA, problemaDeContrasenaNueva } from "../lib/contrasena";
import marca from "../assets/marca.png";

type Modo = "ingresar" | "activar" | "olvido";

/** Entrada de anfitriones con el correo de la empresa y su contraseña. Misma
 * marca y misma tarjeta que el panel. Las cuentas las crea administración
 * desde el panel y entregan un código de activación: con él, la persona
 * elige su contraseña (primer ingreso, o después de "restablecer"). */
export default function Login() {
  const { iniciarSesion, activarCuenta, error, verificar } = useAuth();
  const [modo, setModo] = useState<Modo>("ingresar");
  const [correo, setCorreo] = useState("");
  const [contrasena, setContrasena] = useState("");
  const [codigo, setCodigo] = useState("");
  const [nueva, setNueva] = useState("");
  const [repetida, setRepetida] = useState("");
  const [problema, setProblema] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);
  const [fallo, setFallo] = useState<string | null>(null);

  function cambiarModo(nuevo: Modo) {
    setModo(nuevo);
    setFallo(null);
    setProblema(null);
    setContrasena("");
    setCodigo("");
    setNueva("");
    setRepetida("");
  }

  async function ingresar(evento: FormEvent) {
    evento.preventDefault();
    if (enviando) return;
    setEnviando(true);
    setFallo(null);
    try {
      const resultado = await iniciarSesion(correo, contrasena);
      if (!resultado.ok) {
        setFallo(resultado.mensaje);
        setContrasena("");
      }
    } finally {
      setEnviando(false);
    }
  }

  async function activar(evento: FormEvent) {
    evento.preventDefault();
    if (enviando) return;
    const motivo = problemaDeContrasenaNueva(nueva, correo);
    setProblema(motivo ?? (nueva !== repetida ? "Las dos contraseñas no coinciden." : null));
    if (motivo || nueva !== repetida) return;
    setEnviando(true);
    setFallo(null);
    try {
      // Si sale bien, la sesión queda abierta y el portal pasa a la agenda.
      const resultado = await activarCuenta(correo, codigo, nueva);
      if (!resultado.ok) setFallo(resultado.mensaje);
    } finally {
      setEnviando(false);
    }
  }

  const campoCorreo = (
    <label className="campo">
      Correo de la empresa
      <input
        type="email"
        value={correo}
        required
        autoComplete="username"
        autoCapitalize="none"
        autoCorrect="off"
        spellCheck={false}
        inputMode="email"
        maxLength={254}
        onChange={(evento) => setCorreo(evento.target.value)}
      />
    </label>
  );

  return (
    <div className="flex min-h-dvh flex-col">
      <header className="flex h-14 items-center justify-end px-4">
        <BotonTema />
      </header>
      <main id="contenido" className="mx-auto flex w-full max-w-[420px] flex-1 flex-col justify-center gap-5 px-4 pb-16">
        <div className="flex items-center gap-3">
          <img src={marca} alt="" className="h-11 w-11 rounded" />
          <div>
            <h1 className="m-0 text-lg">Agenda de visitas</h1>
            <p className="m-0 text-[13px] text-muted">Lattis</p>
          </div>
        </div>

        <section className="tarjeta flex flex-col gap-4 p-5">
          {modo === "ingresar" && (
            <>
              <p className="m-0">Avise a la portería quién viene a visitarlo, cuándo y a qué lugar.</p>
              <ul className="m-0 flex list-none flex-col gap-2.5 p-0 text-[13px] text-muted">
                <li className="flex items-center gap-2.5">
                  <CalendarDays size={18} className="text-acento" aria-hidden="true" />
                  Un día o varios, con hora estimada.
                </li>
                <li className="flex items-center gap-2.5">
                  <MapPin size={18} className="text-acento" aria-hidden="true" />
                  Uno o varios lugares en la misma visita.
                </li>
                <li className="flex items-center gap-2.5">
                  <UserCheck size={18} className="text-acento" aria-hidden="true" />
                  Vea cuándo llega cada persona.
                </li>
              </ul>
              {error && (
                <Aviso>
                  {error}{" "}
                  <button type="button" className="underline" onClick={verificar}>
                    Volver a verificar
                  </button>
                </Aviso>
              )}
              <form className="flex flex-col gap-3" onSubmit={ingresar}>
                {campoCorreo}
                <CampoContrasena
                  etiqueta="Contraseña"
                  valor={contrasena}
                  onCambiar={setContrasena}
                  autoComplete="current-password"
                />
                {fallo && <Aviso>{fallo}</Aviso>}
                <button type="submit" className="boton boton-primario min-h-11 w-full" disabled={enviando}>
                  {enviando ? "Ingresando…" : "Ingresar"}
                </button>
              </form>
              <div className="flex flex-col items-center gap-2 text-[13px]">
                <button type="button" className="underline" onClick={() => cambiarModo("activar")}>
                  Primer ingreso: tengo un código de activación
                </button>
                <button type="button" className="underline" onClick={() => cambiarModo("olvido")}>
                  ¿Olvidó su contraseña?
                </button>
              </div>
            </>
          )}

          {modo === "activar" && (
            <>
              <h2 className="m-0 text-base font-semibold">Activar su cuenta</h2>
              <p className="m-0 text-[13px] text-muted">
                Escriba su correo, el código que le entregó administración y la contraseña que quiere usar desde ahora.
              </p>
              <form className="flex flex-col gap-3" onSubmit={activar}>
                {campoCorreo}
                <label className="campo">
                  Código de activación
                  <input
                    value={codigo}
                    required
                    autoComplete="one-time-code"
                    autoCapitalize="characters"
                    autoCorrect="off"
                    spellCheck={false}
                    maxLength={14}
                    onChange={(evento) => setCodigo(evento.target.value.toUpperCase())}
                  />
                </label>
                <CampoContrasena
                  etiqueta="Contraseña nueva"
                  valor={nueva}
                  onCambiar={setNueva}
                  autoComplete="new-password"
                  ayuda={`Al menos ${LONGITUD_MINIMA} caracteres. Puede ser una frase; no hace falta mezclar símbolos.`}
                  error={problema}
                />
                <CampoContrasena
                  etiqueta="Repita la contraseña"
                  valor={repetida}
                  onCambiar={setRepetida}
                  autoComplete="new-password"
                />
                {fallo && <Aviso>{fallo}</Aviso>}
                <button type="submit" className="boton boton-primario min-h-11 w-full" disabled={enviando}>
                  {enviando ? "Activando…" : "Activar e ingresar"}
                </button>
              </form>
              <button type="button" className="self-center text-[13px] underline" onClick={() => cambiarModo("ingresar")}>
                Volver a ingresar
              </button>
            </>
          )}

          {modo === "olvido" && (
            <>
              <h2 className="m-0 text-base font-semibold">¿Olvidó su contraseña?</h2>
              <Aviso tipo="info">
                Pida a administración que restablezca su cuenta. Le entregarán un código de activación nuevo, que vence
                en 72 horas; con él podrá elegir una contraseña nueva en «Primer ingreso».
              </Aviso>
              <button type="button" className="boton min-h-11 w-full" onClick={() => cambiarModo("ingresar")}>
                Volver a ingresar
              </button>
            </>
          )}

          <p className="m-0 text-center text-[12px] text-muted">
            Sólo para anfitriones autorizados. Si no tiene acceso, pídalo a administración.
          </p>
        </section>
      </main>
    </div>
  );
}
