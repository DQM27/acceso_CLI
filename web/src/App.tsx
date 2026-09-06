import { Suspense, lazy, useState } from "react";
import { Toaster, toast } from "sonner";
import { History, IdCard, Menu, ShieldCheck, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import Login from "./pantallas/Login";
import { useVerificacionPorCorreo } from "./componentes/useVerificacionPorCorreo";
import type { UsuarioSesion } from "./api";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { SesionProvider } from "./contexto/SesionContexto";

// Descargar cada pantalla cuando el usuario entra a su sección.
const Dispositivos = lazy(() => import("./pantallas/Dispositivos"));
const Historial = lazy(() => import("./pantallas/Historial"));
const Contratistas = lazy(() => import("./pantallas/Contratistas"));
const Operadores = lazy(() => import("./pantallas/Operadores"));
const Administradores = lazy(() => import("./pantallas/Administradores"));

export type Seccion = "dispositivos" | "historial" | "contratistas" | "operadores" | "administradores";

// Sin distinción de rol -- se eliminó `admin_regional` (nunca tuvo alcance
// real, ver migración `elimina_admin_regional`). Cualquier fila en
// `administradores_panel` ve y puede tocar todo.
const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "operadores", etiqueta: "Operadores", Icono: UserCog },
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: IdCard },
  { id: "administradores", etiqueta: "Administradores", Icono: ShieldCheck },
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

/**
 * Botón de diagnóstico -- dispara SOLO el envío del código OTP
 * (`signInWithOtp`) al correo de la sesión actual, sin pasar por ningún
 * alta/baja de administrador. Aísla si un problema es de la plantilla de
 * correo (Auth > Emails > "Magic Link" en el dashboard de Supabase) o de
 * otra parte del flujo -- ver conversación sobre por qué llegaba un link
 * en vez de un código de 6 dígitos. TODO: sacar este botón una vez
 * confirmado que la plantilla ya manda el código bien.
 */
function BotonProbarOtp({ correo }: { correo: string }) {
  const { enviando, pedirConfirmacion } = useVerificacionPorCorreo(correo);

  async function alClicar() {
    const error = await pedirConfirmacion();
    if (error) {
      toast.error(error);
    } else {
      toast.info(`Código pedido para ${correo} -- revisá el correo (no confirma nada acá).`);
    }
  }

  return (
    <button
      type="button"
      className="boton-discreto"
      style={{
        margin: "0.5rem",
        padding: "0.4rem 0.6rem",
        fontSize: "0.75rem",
        textAlign: "left",
        color: "var(--muted)",
        width: "calc(100% - 1rem)",
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
      }}
      onClick={alClicar}
      disabled={enviando}
      title="Diagnóstico: manda un código OTP a este correo sin agregar/quitar ningún admin"
    >
      {enviando ? "Pidiendo código…" : "🧪 Probar código OTP"}
    </button>
  );
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
            pie={<BotonProbarOtp correo={sesion.correo} />}
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
                <Dispositivos />
              ) : seccion === "administradores" ? (
                <Administradores sesion={sesion} />
              ) : seccion === "historial" ? (
                <Historial />
              ) : seccion === "contratistas" ? (
                <Contratistas />
              ) : seccion === "operadores" ? (
                <Operadores />
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
          <span />
          <MenuUsuario sesion={sesion} onCerrarSesion={cerrarSesion} />
        </div>

        <Toaster theme="system" position="bottom-right" richColors={false} />
      </div>
    </SesionProvider>
  );
}
