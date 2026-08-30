/**
 * Convención para pantallas y componentes nuevos — mismo criterio que ya
 * sigue `comandos/mod.rs` del lado Tauri, ahora escrito acá:
 *
 * 1. Una pantalla nunca importa a otra pantalla. Si dos pantallas necesitan
 *    lo mismo, eso va a `componentes/` (ver `ListaFlotante.tsx`) o a un
 *    hook — nunca una pantalla llamando directo a otra. Única excepción
 *    real: cada pantalla con SU PROPIO Formulario* (Contratistas ↔
 *    FormularioContratista, etc.), que sigue siendo padre → hijo, no
 *    acoplamiento entre hermanas.
 * 2. `api/*.ts` es la única capa que llama `invoke()`. Ningún componente o
 *    pantalla invoca Tauri directo — el mapeo de tipos/errores del lado
 *    Rust queda en un solo lugar por dominio.
 * 3. Antes de escribir un `useState`/`useEffect` que "se parece a algo que
 *    ya vi" en otra pantalla, revisar `componentes/` primero. La
 *    duplicación que había entre `NuevoIngresoModal`/`SalidaModal`
 *    (buscador con lista flotante + navegación de flechas, casi idéntica
 *    en los dos) es justo el tipo de cosa que este punto evita — ya se
 *    sacó a `ListaFlotante.tsx`, no se vuelve a copiar.
 * 4. Un componente se separa a su propio archivo cuando mezcla markup con
 *    lógica que no le pertenece a la función que lo contiene — la señal
 *    que llevó a sacar `Sidebar.tsx` de `Shell` (que sí hace enrutamiento
 *    y orquesta modales, eso es responsabilidad suya).
 */
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { Toaster, toast } from "sonner";
import { Building2, ClipboardList, History, UserCheck, UserCog, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import Login from "./pantallas/Login";
import Activos from "./pantallas/Activos";
import Contratistas from "./pantallas/Contratistas";
import Empresas from "./pantallas/Empresas";
import Usuarios from "./pantallas/Usuarios";
import Historial from "./pantallas/Historial";
import Auditoria from "./pantallas/Auditoria";
import NuevoIngresoModal from "./pantallas/NuevoIngresoModal";
import SalidaModal from "./pantallas/SalidaModal";
import Consola from "./pantallas/Consola";
import { buscarActualizacion, cerrarSesion, instalarActualizacion, requiereConfiguracionInicial } from "./api";
import type { RolUsuario, UsuarioSesion } from "./api";

type Pantalla =
  | { tipo: "cargando" }
  | { tipo: "requiere-configuracion-inicial" }
  | { tipo: "login" }
  | { tipo: "shell"; sesion: UsuarioSesion };

const CLAVE_SIDEBAR_COLAPSADO = "sidebar:colapsado";

/** `localStorage` puede fallar (modo privado, cuota llena) — mismo criterio
 * que `leerEstadoGuardado`/`guardarLayout` en `Tabla.tsx`: perder la
 * preferencia guardada no es motivo para romper nada, sólo se vuelve al
 * valor por defecto (expandido). */
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
  const [pantalla, setPantalla] = useState<Pantalla>({ tipo: "cargando" });

  useEffect(() => {
    requiereConfiguracionInicial()
      .then((requiere) =>
        setPantalla(requiere ? { tipo: "requiere-configuracion-inicial" } : { tipo: "login" }),
      )
      .catch((error) => {
        // Deja intentar login igual — si el problema persiste, el propio
        // comando `login` lo va a reportar con su propio mensaje de error.
        console.error(error);
        setPantalla({ tipo: "login" });
      });
  }, []);

  if (pantalla.tipo === "cargando") {
    return null;
  }

  if (pantalla.tipo === "requiere-configuracion-inicial") {
    return (
      <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}>
        <p style={{ maxWidth: "24rem", textAlign: "center", color: "var(--muted)" }}>
          Todavía no existe un usuario ROOT. Creá el usuario ROOT inicial desde la consola
          (<code>--tui-clasica</code> o <code>--comandos</code>) y volvé a abrir esta ventana.
        </p>
      </div>
    );
  }

  if (pantalla.tipo === "login") {
    return <Login onAutenticado={(sesion) => setPantalla({ tipo: "shell", sesion })} />;
  }

  return (
    <Shell
      sesion={pantalla.sesion}
      onCerrarSesion={() => {
        cerrarSesion().finally(() => setPantalla({ tipo: "login" }));
      }}
    />
  );
}

export type Seccion =
  | "activos"
  | "historial"
  | "contratistas"
  | "auditoria"
  | "empresas"
  | "usuarios";

/** `rolesPermitidos` ausente = visible para cualquier rol logueado. Sólo
 * Auditoría lo restringe hoy — espejo de `RolUsuario::puede(VerAuditoria)`
 * en `src/domain/autorizacion.rs` (Root y Administrador sí, Operador no).
 * El resto de las pantallas no tiene una operación de sólo-lectura
 * restringida por rol en el núcleo (algunas acciones puntuales adentro sí,
 * ej. activar/desactivar, pero eso ya lo rechaza el comando — no hace falta
 * ocultar la sección entera por eso). Si el núcleo agrega otra operación de
 * rol para "ver X", el mismo patrón (agregar `rolesPermitidos` acá) alcanza
 * — no hace falta un mecanismo más genérico todavía. */
const SECCIONES: {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
  rolesPermitidos?: RolUsuario[];
}[] = [
  { id: "activos", etiqueta: "Ingresos activos", Icono: UserCheck },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  {
    id: "auditoria",
    etiqueta: "Auditoría",
    Icono: ClipboardList,
    rolesPermitidos: ["Root", "Administrador"],
  },
  { id: "empresas", etiqueta: "Empresas", Icono: Building2 },
  { id: "usuarios", etiqueta: "Usuarios", Icono: UserCog },
];

/**
 * Interfaz central: sidebar izquierdo con las secciones + área de contenido
 * a la derecha. Cada sección nueva (ingresos, activos, historial...) sólo
 * agrega una entrada acá y su propio componente — no toca el resto.
 */
function Shell({
  sesion,
  onCerrarSesion,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
}) {
  const [seccion, setSeccion] = useState<Seccion>("activos");
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);

  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [modalSalida, setModalSalida] = useState(false);
  // Sube en cada registro/salida exitosa — Activos lo usa para refrescar su
  // grilla aunque haya salido desde otra pantalla.
  const [refrescarActivos, setRefrescarActivos] = useState(0);

  // Ctrl+Shift+N/S (no Ctrl+N/S solos — esas convenciones quedan libres
  // para un "nuevo"/"salida" más genéricos más adelante) desde cualquier
  // pantalla: ambos modales son autosuficientes (buscan y registran sin
  // depender de qué sección esté abierta), así que no tiene sentido
  // atarlos a un botón dentro de Activos únicamente. Deshabilitados por
  // defecto mientras se escribe en un campo de texto (comportamiento por
  // defecto de la librería).
  useHotkeys("ctrl+shift+n", () => setModalNuevoIngreso(true), { preventDefault: true });
  useHotkeys("ctrl+shift+s", () => setModalSalida(true), { preventDefault: true });

  // Una sola vez por sesión (no cada X minutos todavía — la app se abre y
  // cierra bastante seguido, esto ya cubre el caso normal). Falla en
  // silencio a propósito: sin conexión o GitHub caído no debe interrumpir a
  // alguien que ya está trabajando, sólo no hay novedad que avisar.
  useEffect(() => {
    let vigente = true;
    buscarActualizacion()
      .then((actualizacion) => {
        if (!vigente || !actualizacion) return;
        toast(`Versión ${actualizacion.version} disponible`, {
          description: "Se descarga, se instala y la app se reinicia sola.",
          duration: Infinity,
          action: {
            label: "Actualizar",
            onClick: () => {
              toast.promise(instalarActualizacion(actualizacion), {
                loading: "Descargando actualización…",
                success: "Actualizado — reiniciando…",
                error: (error) => `No se pudo actualizar: ${String(error)}`,
              });
            },
          },
        });
      })
      .catch((error) => console.error("No se pudo buscar actualizaciones:", error));
    return () => {
      vigente = false;
    };
  }, []);

  const seccionesVisibles = SECCIONES.filter(
    (item) => !item.rolesPermitidos || item.rolesPermitidos.includes(sesion.rol),
  );

  return (
    <div style={{ display: "flex", height: "100%" }}>
      <Sidebar
        secciones={seccionesVisibles}
        seccionActual={seccion}
        onCambiarSeccion={setSeccion}
        colapsado={colapsado}
        onToggleColapsado={() =>
          setColapsado((actual) => {
            const siguiente = !actual;
            guardarSidebarColapsado(siguiente);
            return siguiente;
          })
        }
        sesion={sesion}
        onCerrarSesion={onCerrarSesion}
      />

      <main style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
        {seccion === "activos" && (
          <Activos
            refrescarSenal={refrescarActivos}
            onAbrirNuevoIngreso={() => setModalNuevoIngreso(true)}
            onAbrirSalida={() => setModalSalida(true)}
          />
        )}
        {seccion === "historial" && <Historial />}
        {seccion === "contratistas" && <Contratistas />}
        {seccion === "auditoria" && <Auditoria />}
        {seccion === "empresas" && <Empresas />}
        {seccion === "usuarios" && <Usuarios />}
      </main>

      {modalNuevoIngreso && (
        <NuevoIngresoModal
          onRegistrado={() => setRefrescarActivos((n) => n + 1)}
          onCerrar={() => setModalNuevoIngreso(false)}
        />
      )}

      {modalSalida && (
        <SalidaModal
          onRegistrado={() => setRefrescarActivos((n) => n + 1)}
          onCerrar={() => setModalSalida(false)}
        />
      )}

      <Consola onNavegar={setSeccion} onCerrarSesion={onCerrarSesion} />
      {/* theme="system": mismo criterio que el resto de la app (paleta
          clara/oscura sigue `prefers-color-scheme`, sin toggle manual
          todavía) — estilizado con las variables propias en index.css, no
          los colores por defecto de sonner. */}
      <Toaster theme="system" position="bottom-right" richColors={false} />
    </div>
  );
}
