import { useState } from "react";
import { Toaster } from "sonner";
import { History, IdCard, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import Login from "./pantallas/Login";
import type { UsuarioSesion } from "./api";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { SesionProvider } from "./contexto/SesionContexto";

export type Seccion = "dispositivos" | "historial" | "contratistas" | "operadores";

const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: IdCard },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "operadores", etiqueta: "Operadores", Icono: UserCog },
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
  const [seccion, setSeccion] = useState<Seccion>("dispositivos");
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);

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
          <Sidebar
            secciones={SECCIONES}
            seccionActual={seccion}
            onCambiarSeccion={setSeccion}
            colapsado={colapsado}
            onToggleColapsado={alternarColapsado}
          />

          <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
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
          </main>
        </div>

        <div className="barra-estado">
          <span />
          <MenuUsuario sesion={sesion} onCerrarSesion={cerrarSesion} />
        </div>

        <Toaster theme="system" position="bottom-right" richColors={false} />
      </div>
    </SesionProvider>
  );
}
