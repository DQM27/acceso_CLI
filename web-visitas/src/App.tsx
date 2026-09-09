import { lazy, Suspense, useEffect, useState } from "react";
import {
  createBrowserRouter,
  RouterProvider,
  Navigate,
  NavLink,
  Route,
  Routes,
  useLocation,
} from "react-router-dom";
import { LogOut } from "lucide-react";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { Aviso, Cargando, SelectorTema } from "./componentes/Comunes";
import Login from "./pantallas/Login";
import marca from "./assets/marca.png";
const MisCitas = lazy(() => import("./pantallas/MisCitas"));
const NuevaCita = lazy(() => import("./pantallas/NuevaCita"));

function Contenido() {
  const { anfitrion, cargando } = useAuth();
  if (cargando)
    return (
      <main className="error-fatal">
        <Cargando texto="Verificando tu acceso…" />
      </main>
    );
  if (!anfitrion) return <Login />;
  return <Portal key={anfitrion.id} />;
}

function Portal() {
  const { anfitrion, error, verificar, cerrarSesion } = useAuth();
  const [saliendo, setSaliendo] = useState(false);
  const ruta = useLocation();
  useEffect(() => {
    document.title = `${ruta.pathname === "/nueva" ? "Nueva cita" : "Mis citas"} · Brisas`;
    document.getElementById("contenido")?.focus();
  }, [ruta.pathname]);
  return (
    <div className="portal">
      <a className="saltar" href="#contenido">
        Saltar al contenido
      </a>
      <header className="barra-superior">
        <NavLink to="/citas" className="marca">
          <img src={marca} alt="" />
          <span>
            Brisas<span className="marca-subtitulo">Agenda de visitas</span>
          </span>
        </NavLink>
        <div className="barra-cuenta">
          <SelectorTema />
          <span className="separador" />
          <span className="avatar" aria-hidden="true">
            {anfitrion!.nombre.charAt(0).toUpperCase()}
          </span>
          <div className="identidad">
            <strong>{anfitrion!.nombre}</strong>
            <span>{anfitrion!.correo}</span>
          </div>
          <button
            className="boton boton-discreto solo-icono"
            disabled={saliendo}
            aria-label="Cerrar sesión"
            onClick={async () => {
              setSaliendo(true);
              try {
                await cerrarSesion();
              } finally {
                setSaliendo(false);
              }
            }}
          >
            <LogOut aria-hidden="true" />
          </button>
        </div>
      </header>
      <div className="portal-cuerpo">
        <main id="contenido" tabIndex={-1} className="contenido">
          {error && (
            <Aviso>
              {error}
              <button className="enlace-boton" onClick={verificar}>
                Volver a verificar
              </button>
            </Aviso>
          )}
          <Suspense fallback={<Cargando />}>
            <Routes>
              <Route path="/citas" element={<MisCitas />} />
              <Route path="/nueva" element={<NuevaCita />} />
              <Route path="*" element={<Navigate to="/citas" replace />} />
            </Routes>
          </Suspense>
        </main>
        <footer className="portal-pie">
          <span>Brisas · Agenda de visitas</span>
          <span>Hora de Costa Rica</span>
        </footer>
      </div>
    </div>
  );
}

const enrutador = createBrowserRouter([
  {
    path: "*",
    element: (
      <AuthProvider>
        <Contenido />
      </AuthProvider>
    ),
  },
]);
export default function App() {
  return <RouterProvider router={enrutador} />;
}
