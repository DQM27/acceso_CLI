import { useState } from "react";
import type { FormEvent } from "react";
import { CalendarDays, MapPin, UserCheck } from "lucide-react";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, BotonTema } from "../componentes/Comunes";
import { CampoContrasena } from "../componentes/CampoContrasena";
import marca from "../assets/marca.png";

type Modo = "ingresar" | "pedir-codigo" | "codigo";

/** Entrada de anfitriones con el correo de la empresa y su contraseña. Misma
 * marca y misma tarjeta que el panel. Las cuentas no se crean acá: las da de
 * alta administración por SQL (ver README). El primer ingreso y la
 * recuperación usan un código de 6 dígitos que Supabase manda al correo. */
export default function Login() {
  const { iniciarSesion, solicitarCodigo, verificarCodigo, error, verificar } = useAuth();
  const [modo, setModo] = useState<Modo>("ingresar");
  const [correo, setCorreo] = useState("");
  const [contrasena, setContrasena] = useState("");
  const [codigo, setCodigo] = useState("");
  const [enviando, setEnviando] = useState(false);
  const [fallo, setFallo] = useState<string | null>(null);

  function cambiarModo(nuevo: Modo) {
    setModo(nuevo);
    setFallo(null);
    setContrasena("");
    setCodigo("");
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

  async function pedirCodigo(evento: FormEvent) {
    evento.preventDefault();
    if (enviando) return;
    setEnviando(true);
    setFallo(null);
    try {
      const resultado = await solicitarCodigo(correo);
      if (resultado.ok) cambiarModo("codigo");
      else setFallo(resultado.mensaje);
    } finally {
      setEnviando(false);
    }
  }

  async function confirmarCodigo(evento: FormEvent) {
    evento.preventDefault();
    if (enviando) return;
    setEnviando(true);
    setFallo(null);
    try {
      const resultado = await verificarCodigo(correo, codigo);
      // Si sale bien, el portal pasa solo a "Defina su contraseña".
      if (!resultado.ok) {
        setFallo(resultado.mensaje);
        setCodigo("");
      }
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
              <button type="button" className="self-center text-[13px] underline" onClick={() => cambiarModo("pedir-codigo")}>
                ¿Olvidó su contraseña o es su primer ingreso?
              </button>
            </>
          )}

          {modo === "pedir-codigo" && (
            <>
              <h2 className="m-0 text-base font-semibold">Crear o recuperar la contraseña</h2>
              <p className="m-0 text-[13px] text-muted">
                Escriba su correo de la empresa. Si tiene una cuenta de anfitrión, le enviaremos un código de 6 dígitos
                para definir su contraseña.
              </p>
              <form className="flex flex-col gap-3" onSubmit={pedirCodigo}>
                {campoCorreo}
                {fallo && <Aviso>{fallo}</Aviso>}
                <button type="submit" className="boton boton-primario min-h-11 w-full" disabled={enviando}>
                  {enviando ? "Enviando…" : "Enviar código"}
                </button>
              </form>
              <button type="button" className="self-center text-[13px] underline" onClick={() => cambiarModo("ingresar")}>
                Volver a ingresar
              </button>
            </>
          )}

          {modo === "codigo" && (
            <>
              <h2 className="m-0 text-base font-semibold">Escriba el código</h2>
              {/* El mismo texto exista o no la cuenta: no se revela quién es anfitrión. */}
              <Aviso tipo="info">
                Si {correo.trim()} corresponde a una cuenta de anfitrión, en unos minutos recibirá un código. Vence
                pronto y sirve una sola vez.
              </Aviso>
              <form className="flex flex-col gap-3" onSubmit={confirmarCodigo}>
                <label className="campo">
                  Código
                  <input
                    value={codigo}
                    required
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    pattern="[0-9 ]{6,12}"
                    maxLength={12}
                    onChange={(evento) => setCodigo(evento.target.value)}
                  />
                </label>
                {fallo && <Aviso>{fallo}</Aviso>}
                <button type="submit" className="boton boton-primario min-h-11 w-full" disabled={enviando}>
                  {enviando ? "Verificando…" : "Continuar"}
                </button>
              </form>
              <div className="flex justify-between text-[13px]">
                <button type="button" className="underline" onClick={() => cambiarModo("pedir-codigo")}>
                  Pedir otro código
                </button>
                <button type="button" className="underline" onClick={() => cambiarModo("ingresar")}>
                  Volver a ingresar
                </button>
              </div>
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
