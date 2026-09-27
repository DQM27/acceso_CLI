import { useState } from "react";
import { ArrowRight, CalendarDays, MapPin, Users } from "lucide-react";
import { useAuth } from "../contexto/AuthContexto";
import { Aviso, SelectorTema } from "../componentes/Comunes";
import marca from "../assets/marca.png";

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
      <header className="flex items-center justify-between px-6 py-5">
        <a href="/" className="flex items-center gap-3 text-lg font-semibold">
          <img src={marca} alt="" className="h-10 w-10 rounded-[var(--radio)]" />
          <span>
            Visitas
            <span className="block text-xs font-normal text-muted">Portal de anfitriones</span>
          </span>
        </a>
        <SelectorTema />
      </header>
      <main className="mx-auto grid w-full max-w-[420px] flex-1 items-center px-6 py-8">
        <section className="tarjeta flex flex-col gap-5 p-8 shadow-[var(--sombra-panel)]">
          <span className="grid h-12 w-12 place-items-center rounded-[var(--radio)] bg-[var(--acento-suave)] text-[var(--acento)]">
            <CalendarDays aria-hidden="true" />
          </span>
          <div>
            <h1 className="text-xl font-semibold">Bienvenido</h1>
            <p className="mt-1 text-sm text-muted">
              Ingrese con la cuenta de Google que tiene autorizada para agendar visitas.
            </p>
          </div>
          <ul className="flex flex-col gap-3 text-sm">
            <li className="flex items-center gap-3">
              <CalendarDays aria-hidden="true" className="text-[var(--acento)]" size={18} />
              Elija cuándo: un día o un rango de fechas.
            </li>
            <li className="flex items-center gap-3">
              <MapPin aria-hidden="true" className="text-[var(--acento)]" size={18} />
              Indique dónde: uno o varios sitios en un mismo paso.
            </li>
            <li className="flex items-center gap-3">
              <Users aria-hidden="true" className="text-[var(--acento)]" size={18} />
              Agregue a sus visitantes: una persona o todo un grupo.
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
          <button
            type="button"
            className="boton boton-primario justify-between px-5"
            disabled={enviando}
            onClick={() => void entrar()}
          >
            {enviando ? "Conectando…" : "Continuar con Google"}
            <ArrowRight aria-hidden="true" />
          </button>
          <p className="border-t border-borde pt-4 text-xs text-muted">
            ¿Necesita acceso? Contacte a administración para autorizar su cuenta.
          </p>
        </section>
      </main>
      <footer className="flex items-center justify-between px-6 py-4 text-xs text-muted">
        <span>Lattis · Control de acceso</span>
        <span>Portal de anfitriones</span>
      </footer>
    </div>
  );
}
