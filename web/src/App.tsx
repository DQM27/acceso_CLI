import { Suspense, lazy, useState } from "react";
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
    <AuthProvider>
      <Contenido />
    </AuthProvider>
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
  const [seccion, setSeccion] = useState<Seccion>("historial");
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);
  // Independiente de `colapsado` (que es el modo ícono-solo de escritorio,
  // por doble click): en mobile el sidebar es un cajón que está oculto o
  // abierto de par en par, nunca "colapsado a íconos" -- ver el media query
  // en index.css.
  const [menuMovilAbierto, setMenuMovilAbierto] = useState(false);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  function cambiarSeccion(id: Seccion) {
    setSeccion(id);
    // En mobile, elegir una sección cierra el cajón -- si no, tapa la
    // pantalla recién elegida hasta que la persona lo cierre a mano.
    setMenuMovilAbierto(false);
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
            seccionActual={seccion}
            onCambiarSeccion={cambiarSeccion}
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
            <Suspense fallback={<div className="pantalla-cuerpo" role="status">Cargando pantalla…</div>}>
              {seccion === "dispositivos" ? (
                <Dispositivos sesion={sesion} />
              ) : seccion === "historial" ? (
                <Historial />
              ) : seccion === "contratistas" ? (
                <Contratistas />
              ) : seccion === "usuarios" ? (
                <Usuarios />
              ) : (
                <div className="pantalla-cuerpo">
                  <div className="tarjeta" style={{ padding: "1.5rem" }}>
                    <h2 style={{ margin: "0 0 0.5rem", color: "var(--acento)" }}>
                      {SECCIONES.find((s) => s.id === seccion)?.etiqueta}
                    </h2>
                    <p style={{ margin: 0, color: "var(--muted)" }}>
                      Login conectado — falta esta pantalla de verdad.
                    </p>
                  </div>
                </div>
              )}
            </Suspense>
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
