import { useState } from "react";
import type { FormEvent } from "react";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, BotonTema } from "../componentes/Comunes";
import { CampoContrasena } from "../componentes/CampoContrasena";
import { LONGITUD_MINIMA, problemaDeContrasenaNueva } from "../lib/contrasena";
import marca from "../assets/marca.png";

/** Después de entrar con un código de correo (primer ingreso o
 * recuperación): la persona define su contraseña antes de ver la agenda. */
export default function DefinirContrasena() {
  const { anfitrion, definirContrasena, cerrarSesion } = useAuth();
  const correo = anfitrion?.correo ?? "";
  const [contrasena, setContrasena] = useState("");
  const [repetida, setRepetida] = useState("");
  const [problema, setProblema] = useState<string | null>(null);
  const [fallo, setFallo] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  async function guardar(evento: FormEvent) {
    evento.preventDefault();
    if (enviando) return;
    const motivo = problemaDeContrasenaNueva(contrasena, correo);
    setProblema(motivo);
    if (motivo) return;
    if (contrasena !== repetida) {
      setProblema("Las dos contraseñas no coinciden.");
      return;
    }
    setEnviando(true);
    setFallo(null);
    try {
      const resultado = await definirContrasena(contrasena);
      if (!resultado.ok) setFallo(resultado.mensaje);
    } finally {
      setEnviando(false);
    }
  }

  return (
    <div className="flex min-h-dvh flex-col">
      <header className="flex h-14 items-center justify-end px-4">
        <BotonTema />
      </header>
      <main id="contenido" className="mx-auto flex w-full max-w-[420px] flex-1 flex-col justify-center gap-5 px-4 pb-16">
        <div className="flex items-center gap-3">
          <img src={marca} alt="" className="h-11 w-11 rounded" />
          <div>
            <h1 className="m-0 text-lg">Defina su contraseña</h1>
            <p className="m-0 text-[13px] text-muted">Lattis</p>
          </div>
        </div>

        <section className="tarjeta flex flex-col gap-4 p-5">
          <form className="flex flex-col gap-3" onSubmit={guardar}>
            <p className="m-0 text-[13px] text-muted">
              Cuenta: <span className="text-texto">{correo}</span>
            </p>
            {/* Para que el gestor de contraseñas guarde la cuenta correcta. */}
            <input type="email" value={correo} autoComplete="username" readOnly hidden />
            <CampoContrasena
              etiqueta="Contraseña nueva"
              valor={contrasena}
              onCambiar={setContrasena}
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
              {enviando ? "Guardando…" : "Guardar contraseña"}
            </button>
          </form>
          <button type="button" className="self-center text-[13px] underline" onClick={() => void cerrarSesion()}>
            Cancelar y salir
          </button>
        </section>
      </main>
    </div>
  );
}
