import { lazy, Suspense, useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation } from "react-router-dom";
import { LogOut } from "lucide-react";
import { Toaster } from "sonner";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { Aviso, Cargando, SelectorTema } from "./componentes/Comunes";
import Login from "./pantallas/Login";
import marca from "./assets/marca.png";

const MisVisitas = lazy(() => import("./pantallas/MisVisitas"));
const AgendarVisita = lazy(() => import("./pantallas/AgendarVisita"));
const DetalleVisita = lazy(() => import("./pantallas/DetalleVisita"));

const TITULOS: Record<string, string> = {
  "/": "Mis visitas",
  "/agendar": "Agendar visita",
};

function Contenido() {
  const { anfitrion, cargando } = useAuth();
  if (cargando)
    return (
      <main className="flex min-h-dvh items-center justify-center">
        <Cargando texto="Verificando su acceso…" />
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
    const titulo = TITULOS[ruta.pathname] ?? "Detalle de visita";
    document.title = `${titulo} · Visitas`;
    document.getElementById("contenido")?.focus();
  }, [ruta.pathname]);

  if (!anfitrion) return null;

  return (
    <div className="flex min-h-dvh flex-col">
      <a className="saltar" href="#contenido">
        Saltar al contenido
      </a>
      <header className="flex items-center justify-between border-b border-borde px-4 py-3 sm:px-6">
        <a href="/" className="flex items-center gap-2 font-semibold">
          <img src={marca} alt="" className="h-8 w-8 rounded-[var(--radio-chico)]" />
          <span className="hidden sm:inline">Visitas</span>
        </a>
        <div className="flex items-center gap-2 sm:gap-3">
          <SelectorTema />
          <span className="hidden text-sm sm:block">
            <strong className="block leading-tight">{anfitrion.nombre}</strong>
            <span className="text-xs text-muted">{anfitrion.correo}</span>
          </span>
          <button
            type="button"
            className="boton boton-fantasma boton-icono"
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

      <main id="contenido" tabIndex={-1} className="mx-auto w-full max-w-[720px] flex-1 px-4 py-6 sm:px-6">
        {error && (
          <div className="mb-4">
            <Aviso>
              {error}{" "}
              <button type="button" className="underline" onClick={verificar}>
                Volver a verificar
              </button>
            </Aviso>
          </div>
        )}
        <Suspense fallback={<Cargando />}>
          <Routes>
            <Route path="/" element={<MisVisitas />} />
            <Route path="/agendar" element={<AgendarVisita />} />
            <Route path="/visitas/:id" element={<DetalleVisita />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Suspense>
      </main>
      <Toaster theme="system" position="bottom-center" richColors={false} />
    </div>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Contenido />
      </AuthProvider>
    </BrowserRouter>
  );
}
