import { useEffect, useRef, useState } from "react";
import { Toaster, toast } from "sonner";
import { CheckCircle2, History, IdCard, Loader2, Menu, ShieldCheck, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import Login from "./pantallas/Login";
import Dispositivos from "./pantallas/Dispositivos";
import Historial from "./pantallas/Historial";
import Contratistas from "./pantallas/Contratistas";
import Operadores from "./pantallas/Operadores";
import Administradores from "./pantallas/Administradores";
import { borrarAccionPendiente, leerAccionPendienteVigente } from "./componentes/accionesPendientes";
import { agregarAdministrador, eliminarAdministrador } from "./api/administradores";
import type { UsuarioSesion } from "./api";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { SesionProvider } from "./contexto/SesionContexto";

export type Seccion = "dispositivos" | "historial" | "contratistas" | "operadores" | "administradores";

// Sin distinción de rol -- se eliminó `admin_regional` (nunca tuvo alcance
// real, ver migración `elimina_admin_regional`). Cualquier fila en
// `administradores_panel` ve y puede tocar todo.
const SECCIONES: { id: Seccion; etiqueta: string; Icono: LucideIcon }[] = [
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: IdCard },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "operadores", etiqueta: "Operadores", Icono: UserCog },
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
type EstadoAccionPendiente = { paso: "verificando" } | { paso: "lista"; mensaje: string };

function Contenido() {
  const { sesion, cargando } = useAuth();
  const [estadoAccion, setEstadoAccion] = useState<EstadoAccionPendiente | null>(null);

  // Retoma una acción sensible confirmada por correo (ver
  // `accionesPendientes.ts` y `Administradores.tsx`) apenas la sesión está
  // lista -- pasa acá y no en `Administradores.tsx` porque esa pantalla ni
  // siquiera está montada al volver del link (la Shell arranca siempre en
  // "dispositivos"). `intentado` evita reintentar en cada re-render de este
  // componente una vez que ya se resolvió (o no había nada que resolver).
  const intentado = useRef(false);
  useEffect(() => {
    if (!sesion || intentado.current) return;
    intentado.current = true;

    const accion = leerAccionPendienteVigente(sesion.correo);
    if (!accion) return;

    // Recibe `accion` por parámetro (no por closure) a propósito: el
    // narrowing de una unión discriminada no cruza el límite de una
    // función anidada aunque la variable capturada sea `const`.
    async function resolver(accion: NonNullable<ReturnType<typeof leerAccionPendienteVigente>>) {
      setEstadoAccion({ paso: "verificando" });
      try {
        if (accion.tipo === "agregar_admin") {
          await agregarAdministrador(accion.correoNuevo);
          setEstadoAccion({ paso: "lista", mensaje: `${accion.correoNuevo} ya puede entrar al panel.` });
        } else {
          await eliminarAdministrador(accion.correoAQuitar);
          setEstadoAccion({ paso: "lista", mensaje: `${accion.correoAQuitar} ya no tiene acceso.` });
        }
        // El check queda visible un momento antes de pasar al panel --
        // desaparecer de golpe se sentiría como que no pasó nada.
        setTimeout(() => setEstadoAccion(null), 1400);
      } catch (error) {
        toast.error(String(error));
        setEstadoAccion(null);
      } finally {
        borrarAccionPendiente();
      }
    }

    resolver(accion);
  }, [sesion]);

  if (cargando) {
    return null;
  }

  if (estadoAccion) {
    return <PantallaAccionPendiente estado={estadoAccion} />;
  }

  if (!sesion) {
    return <Login />;
  }

  return <Shell sesion={sesion} />;
}

/** Spinner mientras se termina de agregar/quitar el admin confirmado por
 * correo, con un check al terminar -- sin esto, el momento entre volver del
 * link y ver el panel se sentía como una pantalla en blanco rota, no como
 * "está pasando algo". */
function PantallaAccionPendiente({ estado }: { estado: EstadoAccionPendiente }) {
  return (
    <div className="grid min-h-full place-items-center bg-fondo px-6 py-10 text-texto">
      <div
        className="tarjeta"
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: "1rem",
          padding: "2rem",
          width: "100%",
          maxWidth: "22rem",
          textAlign: "center",
        }}
      >
        {estado.paso === "verificando" ? (
          <>
            <Loader2 size={32} strokeWidth={2} className="girando" color="var(--acento)" />
            <p style={{ margin: 0, color: "var(--muted)" }}>Confirmando…</p>
          </>
        ) : (
          <>
            <CheckCircle2 size={32} strokeWidth={2} color="var(--exito)" />
            <p style={{ margin: 0 }}>{estado.mensaje}</p>
          </>
        )}
      </div>
    </div>
  );
}

function Shell({ sesion }: { sesion: UsuarioSesion }) {
  const { cerrarSesion } = useAuth();
  const [seccion, setSeccion] = useState<Seccion>("dispositivos");
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
