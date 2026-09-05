import { useEffect, useRef, useState } from "react";
import { Toaster, toast } from "sonner";
import { CheckCircle2, History, IdCard, Loader2, ShieldCheck, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import Login from "./pantallas/Login";
import Historial from "./pantallas/Historial";
import Contratistas from "./pantallas/Contratistas";
import Operadores from "./pantallas/Operadores";
import Administradores from "./pantallas/Administradores";
import { borrarAccionPendiente, leerAccionPendienteVigente } from "./componentes/accionesPendientes";
import { agregarAdministrador, eliminarAdministrador } from "./api/administradores";
import type { RolAdminPanel, UsuarioSesion } from "./api";
import { AuthProvider, useAuth } from "./contexto/AuthContexto";
import { SesionProvider } from "./contexto/SesionContexto";

export type Seccion = "dispositivos" | "historial" | "contratistas" | "operadores" | "administradores";

const SECCIONES: {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
  rolesPermitidos?: RolAdminPanel[];
}[] = [
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: IdCard },
  {
    id: "historial",
    etiqueta: "Historial",
    Icono: History,
    // RLS de `ingresos`/`sitios` sólo deja leer a admin_global por ahora
    // (ver migración agrega_columnas_historial_a_ingresos) -- admin_regional
    // queda afuera hasta que administradores_panel sepa qué sitios
    // administra cada quien.
    rolesPermitidos: ["admin_global"],
  },
  {
    id: "contratistas",
    etiqueta: "Contratistas",
    Icono: Users,
    // Mismo motivo que "historial" -- RLS de `contratistas` sólo deja
    // pasar a admin_global por ahora (migración
    // admin_global_gestiona_contratistas).
    rolesPermitidos: ["admin_global"],
  },
  { id: "operadores", etiqueta: "Operadores", Icono: UserCog },
  {
    id: "administradores",
    etiqueta: "Administradores",
    Icono: ShieldCheck,
    // Quién puede entrar al panel es cosa de admin_global -- espejo de
    // `administradores_panel` (sólo admin_global puede agregar/quitar
    // filas ahí, ver la migración de RLS).
    rolesPermitidos: ["admin_global"],
  },
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
          await agregarAdministrador(accion.correoNuevo, accion.rolNuevo);
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

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  const seccionesVisibles = SECCIONES.filter(
    (item) => !item.rolesPermitidos || item.rolesPermitidos.includes(sesion.rol),
  );

  return (
    <SesionProvider value={null}>
      <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          <Sidebar
            secciones={seccionesVisibles}
            seccionActual={seccion}
            onCambiarSeccion={setSeccion}
            colapsado={colapsado}
            onToggleColapsado={alternarColapsado}
          />

          <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
            {seccion === "administradores" ? (
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
