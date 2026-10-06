import { lazy, Suspense, useEffect } from "react";
import { createBrowserRouter, Navigate, Outlet, RouterProvider, useLocation } from "react-router-dom";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { Aviso, Cargando } from "./componentes/Comunes";
import Login from "./pantallas/Login";
import MisVisitas from "./pantallas/MisVisitas";

const AgendarVisita = lazy(() => import("./pantallas/AgendarVisita"));
const DetalleVisita = lazy(() => import("./pantallas/DetalleVisita"));
const EditarVisita = lazy(() => import("./pantallas/EditarVisita"));
const Historial = lazy(() => import("./pantallas/Historial"));

const TITULOS: [RegExp, string][] = [
  [/^\/agendar/, "Agendar visita"],
  [/^\/historial/, "Historial"],
  [/^\/visitas\/[^/]+\/editar/, "Editar visita"],
  [/^\/visitas\/[^/]+/, "Visita"],
];

/** Sin sesión de anfitrión: el login. Con sesión: la pantalla pedida. */
function Portal() {
  const { anfitrion, cargando, error, verificar } = useAuth();
  const ruta = useLocation();
  useEffect(() => {
    const titulo = TITULOS.find(([patron]) => patron.test(ruta.pathname))?.[1] ?? "Mis visitas";
    document.title = `${titulo} · Visitas · Lattis`;
  }, [ruta.pathname]);

  if (cargando) return <Cargando texto="Verificando su acceso…" />;
  if (!anfitrion) return <Login />;
  return (
    <div key={anfitrion.id}>
      <a className="saltar" href="#contenido">
        Saltar al contenido
      </a>
      {error && (
        <div className="mx-auto max-w-[720px] px-4 pt-3">
          <Aviso>
            {error}{" "}
            <button type="button" className="underline" onClick={verificar}>
              Volver a verificar
            </button>
          </Aviso>
        </div>
      )}
      <Suspense fallback={<Cargando />}>
        <Outlet />
      </Suspense>
    </div>
  );
}

const enrutador = createBrowserRouter([
  {
    element: (
      <AuthProvider>
        <Portal />
      </AuthProvider>
    ),
    children: [
      { path: "/visitas", element: <MisVisitas /> },
      { path: "/historial", element: <Historial /> },
      { path: "/agendar", element: <AgendarVisita /> },
      { path: "/visitas/:id", element: <DetalleVisita /> },
      { path: "/visitas/:id/editar", element: <EditarVisita /> },
      // "/", la vieja vuelta de Google ("/auth/callback") y cualquier ruta
      // vieja ("/citas", "/nueva") van a Mis visitas.
      { path: "*", element: <Navigate to="/visitas" replace /> },
    ],
  },
]);

export default function App() {
  return <RouterProvider router={enrutador} />;
}
