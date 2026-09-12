import { Suspense, lazy, useEffect, useState } from "react";
import { BrowserRouter, Navigate, useLocation } from "react-router-dom";
import { Toaster } from "sonner";
import { History, Menu, MonitorSmartphone, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import SelectorTema from "./componentes/SelectorTema";
import Login from "./pantallas/Login";
import type { UsuarioSesion } from "./api";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { SesionProvider } from "./contexto/SesionContexto";

// Descargar cada pantalla cuando el usuario entra a su sección.
const Dispositivos = lazy(() => import("./pantallas/Dispositivos"));
const Historial = lazy(() => import("./pantallas/Historial"));
const Contratistas = lazy(() => import("./pantallas/Contratistas"));
const Usuarios = lazy(() => import("./pantallas/Usuarios"));

export type Seccion = "dispositivos" | "historial" | "contratistas" | "usuarios";

/** Ruta real de cada sección -- `Sidebar` arma sus `NavLink` con esto y
 * `Shell` compara `location.pathname` contra el mismo valor para decidir
 * qué mostrar, así las dos fuentes no pueden desincronizarse en silencio. */
export function rutaSeccion(id: Seccion): string {
  return `/${id}`;
}

// Sin distinción de rol -- se eliminó `admin_regional` (nunca tuvo alcance
// real, ver migración `elimina_admin_regional`). Cualquier fila en
// `administradores_panel` ve y puede tocar todo.
//
// Sin sección "Administradores" a propósito: el alta/baja de quién puede
// entrar al panel se saca deliberadamente del panel mismo (mismo criterio
// que ROOT en desktop/TUI, que tampoco se gestiona desde ninguna app --
// ver `crear_root_inicial`) -- así un compromiso del panel web (XSS, una
// dependencia comprometida) no puede fabricarse a sí mismo un admin nuevo.
// Se gestiona con SQL directo en el dashboard de Supabase:
//   insert into administradores_panel (correo) values ('nuevo@admin.com');
//   delete from administradores_panel where correo = 'quitar@admin.com';
//
// "usuarios" (no "operadores") a propósito -- mismo nombre que la sección
// equivalente de escritorio (`App.tsx` de desktop/), aunque ahí sea
// Root/Administrador/Operador y acá sea Administrador/Operador (ver
// `Usuarios.tsx` de esta carpeta): es la misma entidad global, conviene que
// se llame igual en las dos apps.
const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "usuarios", etiqueta: "Usuarios", Icono: UserCog },
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: MonitorSmartphone },
];

const CLAVE_SIDEBAR_COLAPSADO = "web:sidebar:colapsado";

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

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Contenido />
      </AuthProvider>
    </BrowserRouter>
  );
}

/** Separado de `App` porque `useAuth` necesita estar DENTRO de
 * `<AuthProvider>`, no en el mismo componente que lo declara. */
function Contenido() {
  const { sesion, cargando } = useAuth();

  if (cargando) {
    return null;
  }

  if (!sesion) {
    return <Login />;
  }

  return <Shell sesion={sesion} />;
}

function Shell({ sesion }: { sesion: UsuarioSesion }) {
  const { cerrarSesion } = useAuth();
  const location = useLocation();
  const seccionActual = SECCIONES.find((s) => rutaSeccion(s.id) === location.pathname)?.id;
  // Cada sección visitada se queda MONTADA (oculta con CSS) en vez de
  // desmontarse al navegar a otra -- antes, con <Routes>/<Route>, salir de
  // una pestaña la desmontaba del todo, así que volver a ella disparaba
  // todo de nuevo (pedía los datos otra vez, AG Grid se reconstruía de
  // cero) con el parpadeo de "Cargando pantalla…" cada vez, aunque ya se
  // hubiera visto. Ahora sólo la PRIMERA visita a cada sección paga ese
  // costo (bajar su chunk + el primer fetch) -- volver después es
  // instantáneo. El costo real: cada sección visitada sigue con su
  // `useAutoRefresh` (canal Realtime + poll) corriendo de fondo aunque no
  // se esté viendo -- aceptable para un panel con un puñado de personas
  // usándolo a la vez, no para miles.
  const [visitadas, setVisitadas] = useState<Seccion[]>(() => (seccionActual ? [seccionActual] : []));
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);
  // Independiente de `colapsado` (que es el modo ícono-solo de escritorio,
  // por doble click): en mobile el sidebar es un cajón que está oculto o
  // abierto de par en par, nunca "colapsado a íconos" -- ver el media query
  // en index.css.
  const [menuMovilAbierto, setMenuMovilAbierto] = useState(false);

  useEffect(() => {
    if (!seccionActual) return;
    // `Promise.resolve().then(...)` en vez de llamar `setVisitadas` directo
    // -- evita que `react-hooks/set-state-in-effect` marque esta
    // actualización como síncrona dentro del efecto.
    Promise.resolve().then(() => {
      setVisitadas((actual) => (actual.includes(seccionActual) ? actual : [...actual, seccionActual]));
    });
  }, [seccionActual]);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  return (
    <SesionProvider value={null}>
      <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          {menuMovilAbierto && (
            <div className="shell-sidebar-velo" onClick={() => setMenuMovilAbierto(false)} />
          )}

          <Sidebar
            secciones={SECCIONES}
            // En mobile, elegir una sección cierra el cajón -- si no, tapa la
            // pantalla recién elegida hasta que la persona lo cierre a mano.
            onNavegar={() => setMenuMovilAbierto(false)}
            colapsado={colapsado}
            onToggleColapsado={alternarColapsado}
            abiertoEnMovil={menuMovilAbierto}
          />

          <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
            <button
              type="button"
              className="boton-menu-movil"
              onClick={() => setMenuMovilAbierto((a) => !a)}
              aria-label="Abrir menú"
            >
              <Menu size={20} strokeWidth={2} aria-hidden="true" />
            </button>
            {/* Ruta desconocida (incluida "/") -- mismo default de siempre:
                caer en Historial en vez de una pantalla en blanco. */}
            {!seccionActual && <Navigate to={rutaSeccion("historial")} replace />}
            {visitadas.map((id) => (
              <div
                key={id}
                style={{
                  display: id === seccionActual ? "flex" : "none",
                  flexDirection: "column",
                  flex: 1,
                  minHeight: 0,
                }}
              >
                {/* Suspense por sección, no uno compartido -- así la
                    primera visita a una sección NUEVA (bajando su chunk)
                    no vuelve a tapar con "Cargando pantalla…" las que ya
                    están montadas y visibles detrás. */}
                <Suspense fallback={<div className="pantalla-cuerpo" role="status">Cargando pantalla…</div>}>
                  {id === "historial" ? (
                    <Historial />
                  ) : id === "contratistas" ? (
                    <Contratistas />
                  ) : id === "usuarios" ? (
                    <Usuarios />
                  ) : (
                    <Dispositivos sesion={sesion} />
                  )}
                </Suspense>
              </div>
            ))}
          </main>
        </div>

        <div className="barra-estado">
          <SelectorTema />
          <MenuUsuario sesion={sesion} onCerrarSesion={cerrarSesion} />
        </div>

        <Toaster theme="system" position="bottom-right" richColors={false} />
      </div>
    </SesionProvider>
  );
}
