import { lazy, Suspense, useEffect, useState, ViewTransition } from "react";
import {
  createBrowserRouter,
  RouterProvider,
  Navigate,
  Route,
  Routes,
  useLocation,
} from "react-router-dom";
import { CalendarDays, CirclePlus, LogOut, Menu } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { Aviso, Cargando, SelectorTema } from "./componentes/Comunes";
import Sidebar from "./componentes/Sidebar";
import Login from "./pantallas/Login";
const MisCitas = lazy(() => import("./pantallas/MisCitas"));
const NuevaCita = lazy(() => import("./pantallas/NuevaCita"));

export type Seccion = "citas" | "nueva";

/** Ruta real de cada sección -- `Sidebar` arma sus `NavLink` con esto. */
export function rutaSeccion(id: Seccion): string {
  return `/${id}`;
}

// Sólo dos secciones hoy -- pensado para crecer (ver Sidebar.tsx, copiado
// tal cual de desktop/web) sin tener que rehacer el layout cuando se agregue
// una tercera.
const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "citas", etiqueta: "Mis citas", Icono: CalendarDays },
  { id: "nueva", etiqueta: "Nueva cita", Icono: CirclePlus },
];

const CLAVE_SIDEBAR_COLAPSADO = "brisas-visitas:sidebar:colapsado";

function leerSidebarColapsado(): boolean {
  try {
    return localStorage.getItem(CLAVE_SIDEBAR_COLAPSADO) === "1";
  } catch {
    return false;
  }
}

function guardarSidebarColapsado(colapsado: boolean) {
  try {
    localStorage.setItem(CLAVE_SIDEBAR_COLAPSADO, colapsado ? "1" : "0");
  } catch {
    // Ver comentario de leerSidebarColapsado.
  }
}

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
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);
  const [menuMovilAbierto, setMenuMovilAbierto] = useState(false);
  const ruta = useLocation();
  useEffect(() => {
    document.title = `${ruta.pathname === "/nueva" ? "Nueva cita" : "Mis citas"} · Brisas`;
    document.getElementById("contenido")?.focus();
  }, [ruta.pathname]);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  return (
    <div className="portal">
      <a className="saltar" href="#contenido">
        Saltar al contenido
      </a>
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        {menuMovilAbierto && (
          <div className="shell-sidebar-velo" onClick={() => setMenuMovilAbierto(false)} />
        )}

        <Sidebar
          secciones={SECCIONES}
          onNavegar={() => setMenuMovilAbierto(false)}
          colapsado={colapsado}
          onToggleColapsado={alternarColapsado}
          abiertoEnMovil={menuMovilAbierto}
        />

        <main className="contenido-shell">
          <button
            type="button"
            className="boton-menu-movil"
            onClick={() => setMenuMovilAbierto((a) => !a)}
            aria-label="Abrir menú"
          >
            <Menu aria-hidden="true" />
          </button>
          <div id="contenido" tabIndex={-1} className="contenido">
            {error && (
              <Aviso>
                {error}
                <button className="enlace-boton" onClick={verificar}>
                  Volver a verificar
                </button>
              </Aviso>
            )}
            <Suspense fallback={<Cargando />}>
              {/* React 19.3: cross-fade nativo entre Mis citas y Nueva cita
                  -- `createBrowserRouter` ya envuelve la navegación en
                  `startTransition` desde v6.4+, así que `ViewTransition`
                  detecta el cambio de ruta sin nada más que envolver el
                  contenido rutado. Nombre por defecto ("auto"): alcanza
                  para el cross-fade simple, sin animar elementos
                  individuales entre pantallas. */}
              <ViewTransition>
                <Routes>
                  <Route path="/citas" element={<MisCitas />} />
                  <Route path="/nueva" element={<NuevaCita />} />
                  <Route path="*" element={<Navigate to="/citas" replace />} />
                </Routes>
              </ViewTransition>
            </Suspense>
          </div>
        </main>
      </div>

      <div className="barra-estado">
        <span>Brisas · Agenda de visitas · Hora de Costa Rica</span>
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
