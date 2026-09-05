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
import { Suspense, lazy, useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import {
  Archive,
  Building2,
  ClipboardList,
  Cloud,
  History,
  IdCard,
  UserCheck,
  UserCog,
  Users,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Sidebar from "./componentes/Sidebar";
import MenuUsuario from "./componentes/MenuUsuario";
import BarraNube from "./componentes/BarraNube";
import ErrorBoundary from "./componentes/ErrorBoundary";
import Login from "./pantallas/Login";
import Activos from "./pantallas/Activos";
import {
  buscarActualizacion,
  cerrarSesion,
  instalarActualizacion,
  requiereConfiguracionInicial,
  sincronizarConNube,
} from "./api";
import type { ResumenSincronizacion, RolUsuario, UsuarioSesion } from "./api";
import { emitirActualizacion, iniciarRealtimeNube } from "./nubeRealtime";
import { SesionProvider } from "./contexto/SesionContexto";
import { BarraEstadoProvider } from "./contexto/BarraEstadoContexto";

// Cargadas bajo demanda (`lazy`): salvo Activos (sección por defecto) y
// Login, ninguna pantalla ni modal hace falta en el primer render — cada
// una se pide recién cuando el usuario navega a su sección o abre su modal,
// en vez de sumarse al bundle inicial (ver el <Suspense> que las envuelve
// en `Shell`).
const Contratistas = lazy(() => import("./pantallas/Contratistas"));
const Empresas = lazy(() => import("./pantallas/Empresas"));
const Usuarios = lazy(() => import("./pantallas/Usuarios"));
const Historial = lazy(() => import("./pantallas/Historial"));
const Auditoria = lazy(() => import("./pantallas/Auditoria"));
const Gafetes = lazy(() => import("./pantallas/Gafetes"));
const Respaldos = lazy(() => import("./pantallas/Respaldos"));
const Nube = lazy(() => import("./pantallas/Nube"));
const NuevoIngresoModal = lazy(() => import("./pantallas/NuevoIngresoModal"));
const SalidaModal = lazy(() => import("./pantallas/SalidaModal"));

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
          (<code>--tui-clasica</code> o <code>--cli</code>) y volvé a abrir esta ventana.
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
      onVolverALogin={() => setPantalla({ tipo: "login" })}
    />
  );
}

export type Seccion =
  | "activos"
  | "historial"
  | "contratistas"
  | "auditoria"
  | "empresas"
  | "usuarios"
  | "gafetes"
  | "respaldos"
  | "nube";

/** `rolesPermitidos` ausente = visible para cualquier rol logueado.
 * Auditoría lo restringe — espejo de `RolUsuario::puede(VerAuditoria)` en
 * `src/domain/autorizacion.rs` (Root y Administrador sí, Operador no).
 * Usuarios también — espejo de `Operacion::GestionarUsuarios`
 * (`AppCore::buscar_usuarios` la exige; un Operador entraba y veía la
 * tabla vacía con un toast de error en vez de no ver la sección). El resto
 * de las pantallas no tiene una operación de sólo-lectura restringida por
 * rol en el núcleo (algunas acciones puntuales adentro sí, ej.
 * activar/desactivar, pero eso ya lo rechaza el comando — no hace falta
 * ocultar la sección entera por eso). Si el núcleo agrega otra operación de
 * rol para "ver X", el mismo patrón (agregar `rolesPermitidos` acá) alcanza
 * — no hace falta un mecanismo más genérico todavía. */
const SECCIONES: {
  id: Seccion;
  etiqueta: string;
  Icono: LucideIcon;
  rolesPermitidos?: RolUsuario[];
}[] = [
  { id: "activos", etiqueta: "Activos", Icono: UserCheck },
  { id: "historial", etiqueta: "Historial", Icono: History },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  {
    id: "auditoria",
    etiqueta: "Auditoría",
    Icono: ClipboardList,
    rolesPermitidos: ["Root", "Administrador"],
  },
  { id: "empresas", etiqueta: "Empresas", Icono: Building2 },
  {
    id: "usuarios",
    etiqueta: "Usuarios",
    Icono: UserCog,
    rolesPermitidos: ["Root", "Administrador"],
  },
  { id: "gafetes", etiqueta: "Gafetes", Icono: IdCard },
  {
    id: "respaldos",
    etiqueta: "Respaldos",
    Icono: Archive,
    // Espejo de `Operacion::GestionarRespaldos` (`src/domain/autorizacion.rs`):
    // sólo Root puede gestionar respaldos, ni siquiera Administrador.
    rolesPermitidos: ["Root"],
  },
  {
    id: "nube",
    etiqueta: "Nube",
    Icono: Cloud,
    // Espejo de `Operacion::GestionarNube` (`src/domain/autorizacion.rs`):
    // el secreto de dispositivo es delicado, sólo Root lo administra.
    rolesPermitidos: ["Root"],
  },
];

/**
 * Interfaz central: sidebar izquierdo con las secciones + área de contenido
 * a la derecha. Cada sección nueva (ingresos, activos, historial...) sólo
 * agrega una entrada acá y su propio componente — no toca el resto.
 */
function Shell({
  sesion,
  onCerrarSesion,
  onVolverALogin,
}: {
  sesion: UsuarioSesion;
  onCerrarSesion: () => void;
  onVolverALogin: () => void;
}) {
  const [seccion, setSeccion] = useState<Seccion>("activos");
  const [colapsado, setColapsado] = useState(leerSidebarColapsado);
  // La pantalla montada publica acá su propio texto (ver `useBarraEstado`) —
  // `null` mientras ninguna lo hizo todavía (primer render) o entre una
  // pantalla y la siguiente.
  const [mensajeEstado, setMensajeEstado] = useState<string | null>(null);

  function alternarColapsado() {
    setColapsado((actual) => {
      const siguiente = !actual;
      guardarSidebarColapsado(siguiente);
      return siguiente;
    });
  }

  const [modalNuevoIngreso, setModalNuevoIngreso] = useState(false);
  const [modalSalida, setModalSalida] = useState(false);
  // Sube en cada registro/salida exitosa — Activos lo usa para refrescar su
  // grilla aunque haya salido desde otra pantalla.
  const [refrescarActivos, setRefrescarActivos] = useState(0);
  const [sincronizandoManual, setSincronizandoManual] = useState(false);

  // Ctrl+Shift+N/S (no Ctrl+N/S solos — esas convenciones quedan libres
  // para un "nuevo"/"salida" más genéricos más adelante) desde cualquier
  // pantalla: ambos modales son autosuficientes (buscan y registran sin
  // depender de qué sección esté abierta), así que no tiene sentido
  // atarlos a un botón dentro de Activos únicamente. Deshabilitados por
  // defecto mientras se escribe en un campo de texto (comportamiento por
  // defecto de la librería).
  useHotkeys("ctrl+shift+n", () => setModalNuevoIngreso(true), { preventDefault: true });
  useHotkeys("ctrl+shift+s", () => setModalSalida(true), { preventDefault: true });
  // Mismo atajo que la TUI clásica y --cli (Ctrl+Q cierra sesión desde
  // cualquier pantalla) — acá sin tarjeta de confirmación porque el botón
  // "Cerrar sesión" del sidebar tampoco la pide, así el atajo y el botón se
  // comportan igual.
  useHotkeys("ctrl+q", onCerrarSesion, { preventDefault: true });

  // Los avisos privados refrescan de inmediato; el pulso periódico recupera
  // cambios aunque se pierda el socket o el equipo haya estado sin conexión.
  useEffect(() => {
    const cancelarRealtime = iniciarRealtimeNube({
      onSincronizado: () => setRefrescarActivos((n) => n + 1),
    });
    const cancelarSincronizacionAutomatica = listen<ResumenSincronizacion>(
      "nube://sincronizado",
      ({ payload }) => {
        setRefrescarActivos((n) => n + 1);
        emitirActualizacion(payload);
      },
    );

    return () => {
      cancelarRealtime();
      cancelarSincronizacionAutomatica.then((cancelar) => cancelar());
    };
  }, [sesion.id]);

  // Botón "Sincronizar" de la barra de estado (`BarraNube.tsx`) — visible
  // para cualquier rol activo, ver su doc-comment. `sincronizar_con_nube`
  // ya falla con un mensaje claro (`GestionNubeError::SinSecreto`) si este
  // dispositivo todavía no tiene el secreto configurado, así que no hace
  // falta ocultar el botón para quien no puede configurarlo (eso sigue
  // siendo exclusivo de ROOT en la pantalla Nube).
  async function sincronizarManualmente() {
    setSincronizandoManual(true);
    try {
      const resumen = await sincronizarConNube();
      setRefrescarActivos((n) => n + 1);
      emitirActualizacion(resumen, "manual");
      if (resumen.fallidos === 0) {
        toast.success(`Sincronizado — ${resumen.enviados} enviados.`);
      } else {
        toast.warning(
          `${resumen.enviados} enviados, ${resumen.fallidos} fallidos — reintenta más tarde.`,
        );
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setSincronizandoManual(false);
    }
  }

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
    <SesionProvider value={sesion.id}>
      <BarraEstadoProvider value={setMensajeEstado}>
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
              {/* `key={seccion}` resetea el boundary al cambiar de sección — sin
                  esto, una vez que una pantalla rompe, el error queda "pegado" acá
                  aunque se elija otra sección del menú, porque este `<main>` nunca
                  se desmonta. */}
              <ErrorBoundary
                key={seccion}
                mensaje="Esta sección no pudo cargar. La sesión sigue activa — elegí otra desde el menú, o reiniciá la app si el problema persiste."
              >
                {/* Fallback `null`: las pantallas lazy vienen del mismo bundle
                    local (nada de red de por medio), el chunk carga en
                    milisegundos — no vale la pena un spinner que sólo
                    parpadearía. */}
                <Suspense fallback={null}>
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
                  {seccion === "usuarios" && <Usuarios actorRol={sesion.rol} />}
                  {seccion === "gafetes" && <Gafetes />}
                  {seccion === "respaldos" && <Respaldos onRestaurado={onVolverALogin} />}
                  {seccion === "nube" && <Nube />}
                </Suspense>
              </ErrorBoundary>
            </main>
          </div>

          {/* De lado a lado, debajo de sidebar + contenido — mismo lugar que
              la barra de estado de VSC. Cada pantalla publica su propio texto
              acá (`useBarraEstado`) en vez de dibujarlo ella misma sobre la
              grilla; a la derecha, el usuario (antes fijo en el sidebar). */}
          <div className="barra-estado">
            <span>{mensajeEstado}</span>
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem" }}>
              <BarraNube
                sincronizando={sincronizandoManual}
                onSincronizar={sincronizarManualmente}
              />
              <MenuUsuario sesion={sesion} onCerrarSesion={onCerrarSesion} />
            </div>
          </div>

          <Suspense fallback={null}>
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
          </Suspense>

          {/* theme="system": mismo criterio que el resto de la app (paleta
              clara/oscura sigue `prefers-color-scheme`, sin toggle manual
              todavía) — estilizado con las variables propias en index.css, no
              los colores por defecto de sonner. */}
          <Toaster theme="system" position="bottom-right" richColors={false} />
        </div>
      </BarraEstadoProvider>
    </SesionProvider>
  );
}
