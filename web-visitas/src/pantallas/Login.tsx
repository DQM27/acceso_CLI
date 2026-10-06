import { useState } from "react";
import { CalendarDays, MapPin, UserCheck } from "lucide-react";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, BotonTema } from "../componentes/Comunes";
import marca from "../assets/marca.png";

/** Entrada de anfitriones: Google con la cuenta de la empresa. Misma marca y
 * misma tarjeta que el panel. */
export default function Login() {
  const { iniciarSesion, error, verificar } = useAuth();
  const [enviando, setEnviando] = useState(false);
  async function entrar() {
    setEnviando(true);
    try {
      await iniciarSesion();
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
            <h1 className="m-0 text-lg">Agenda de visitas</h1>
            <p className="m-0 text-[13px] text-muted">Lattis</p>
          </div>
        </div>

        <section className="tarjeta flex flex-col gap-4 p-5">
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
          <button type="button" className="boton boton-primario min-h-11 w-full" disabled={enviando} onClick={entrar}>
            {enviando ? "Abriendo Google…" : "Continuar con Google"}
          </button>
          <p className="m-0 text-center text-[12px] text-muted">
            Sólo para anfitriones autorizados. Si no tiene acceso, pídalo a administración.
          </p>
        </section>
      </main>
    </div>
  );
}
