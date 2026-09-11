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
    <div className="acceso">
      <header className="acceso-barra">
        <a href="/" className="marca">
          <img src={marca} alt="" />
          <span>
            Brisas<span className="marca-subtitulo">Agenda de visitas</span>
          </span>
        </a>
        <SelectorTema />
      </header>
      <main className="acceso-contenido">
        <section className="acceso-presentacion">
          <p className="antetitulo">PORTAL DE ANFITRIONES</p>
          <h1>
            Una buena visita
            <br />
            empieza aquí.
          </h1>
          <p className="entrada">
            Prepará la llegada de tus visitantes. Elegí las fechas, los sitios y
            las personas que vas a recibir.
          </p>
          <ol className="pasos-acceso">
            <li>
              <CalendarDays aria-hidden="true" />
              <div>
                <strong>Elegí cuándo</strong>
                <span>Un día o un rango de fechas.</span>
              </div>
            </li>
            <li>
              <MapPin aria-hidden="true" />
              <div>
                <strong>Indicá dónde</strong>
                <span>Uno o varios sitios en la misma cita.</span>
              </div>
            </li>
            <li>
              <Users aria-hidden="true" />
              <div>
                <strong>Agregá a tus visitantes</strong>
                <span>Una persona o todo un grupo.</span>
              </div>
            </li>
          </ol>
        </section>
        <section className="tarjeta acceso-tarjeta">
          <span className="sello-icono">
            <CalendarDays aria-hidden="true" />
          </span>
          <p className="antetitulo">TU AGENDA, EN UN SOLO LUGAR</p>
          <h2>Te damos la bienvenida</h2>
          <p>
            Ingresá con la cuenta de Google que tenés autorizada para agendar
            visitas.
          </p>
          {error && (
            <Aviso>
              {error}
              <button className="enlace-boton" onClick={verificar}>
                Volver a verificar
              </button>
            </Aviso>
          )}
          <button
            className="boton boton-primario acceso-boton"
            disabled={enviando}
            onClick={() => void entrar()}
          >
            {enviando ? "Conectando…" : "Continuar con Google"}
            <ArrowRight aria-hidden="true" />
          </button>
          <p className="nota-acceso">
            ¿Necesitás acceso? Contactá a administración para autorizar tu
            cuenta.
          </p>
        </section>
      </main>
      <footer className="acceso-pie">
        <span>Brisas · Control de accesos</span>
        <span>Portal de anfitriones</span>
      </footer>
    </div>
  );
}
