import { useState } from "react";
import { Toaster } from "sonner";
import { History, IdCard, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import type { UsuarioSesion } from "./api";
import { SesionProvider } from "./contexto/SesionContexto";

// TODO: reemplazar por la sesión real una vez conectado Supabase Auth
// (Google OAuth + Email OTP, ver docs/plan-panel-administrativo-web.md).
const SESION_PLACEHOLDER: UsuarioSesion = { nombre: "—", rol: "admin_global" };

export type Seccion = "dispositivos" | "historial" | "contratistas" | "operadores";

const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: IdCard },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "operadores", etiqueta: "Operadores", Icono: UserCog },
];

const CLAVE_SIDEBAR_COLAPSADO = "panel-web:sidebar:colapsado";

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
                  Scaffold listo — falta conectar Supabase Auth y las pantallas de verdad.
                </p>
              </div>
            </div>
          </main>
        </div>

        <div className="barra-estado">
          <span />
          <MenuUsuario sesion={SESION_PLACEHOLDER} onCerrarSesion={() => {}} />
        </div>

        <Toaster theme="system" position="bottom-right" richColors={false} />
      </div>
    </SesionProvider>
  );
}
