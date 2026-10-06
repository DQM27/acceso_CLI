import { Suspense, lazy, useEffect, useMemo, useState } from "react";
import { BrowserRouter, Navigate, useLocation, useNavigate } from "react-router-dom";
import { Toaster } from "sonner";
import { BellRing, CalendarCheck, DoorOpen, History, LogIn, Menu, MonitorSmartphone, UserCog, Users } from "lucide-react";
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
const AdentroAhora = lazy(() => import("./pantallas/AdentroAhora"));
const Usuarios = lazy(() => import("./pantallas/Usuarios"));
const Anfitriones = lazy(() => import("./pantallas/Anfitriones"));
const Sesiones = lazy(() => import("./pantallas/Sesiones"));
const Avisos = lazy(() => import("./pantallas/Avisos"));

export type Seccion =
  | "dispositivos"
  | "historial"
  | "adentro"
  | "contratistas"
  | "usuarios"
  | "anfitriones"
  | "sesiones"
  | "avisos";

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
  { id: "adentro", etiqueta: "Adentro ahora", Icono: DoorOpen },
  { id: "contratistas", etiqueta: "Contratistas", Icono: Users },
  { id: "usuarios", etiqueta: "Usuarios", Icono: UserCog },
  // Cuentas de la web de visitas (visitas.megabrisas.com).
  { id: "anfitriones", etiqueta: "Anfitriones", Icono: CalendarCheck },
  { id: "sesiones", etiqueta: "Sesiones", Icono: LogIn },
  { id: "dispositivos", etiqueta: "Dispositivos", Icono: MonitorSmartphone },
  { id: "avisos", etiqueta: "Avisos", Icono: BellRing },
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

// Orden y secciones ocultas del menú, por navegador (igual que en escritorio,
// que las guarda por usuario). `orden` guarda TODOS los ids -- incluidos los
// ocultos, para poder volver a mostrarlos desde el menú contextual --, y
// `ocultas` es el subconjunto no visible.
const CLAVE_SIDEBAR_ORDEN = "web:sidebar:orden";
const CLAVE_SIDEBAR_OCULTAS = "web:sidebar:ocultas";

function seccionValida(id: unknown): id is Seccion {
  return typeof id === "string" && SECCIONES.some((seccion) => seccion.id === id);
}

/** Lista de secciones guardada; `null` si no hay nada o el JSON está roto. Los
 * ids que ya no existen en `SECCIONES` se descartan: una sección quitada del
 * código no debe resucitar como "oculta" ni "reordenada" fantasma. */
function leerListaSecciones(clave: string): Seccion[] | null {
  try {
    const guardado = localStorage.getItem(clave);
    if (!guardado) return null;
    return (JSON.parse(guardado) as unknown[]).filter(seccionValida);
  } catch {
    return null;
  }
}

function guardarListaSecciones(clave: string, ids: Seccion[]) {
  try {
    localStorage.setItem(clave, JSON.stringify(ids));
  } catch {
    // Perder la preferencia no es motivo para romper el menú.
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
  const navigate = useNavigate();
  const [ordenSidebar, setOrdenSidebar] = useState<Seccion[]>(
    () => leerListaSecciones(CLAVE_SIDEBAR_ORDEN) ?? SECCIONES.map((seccion) => seccion.id),
  );
  const [seccionesOcultas, setSeccionesOcultas] = useState<Seccion[]>(
    () => leerListaSecciones(CLAVE_SIDEBAR_OCULTAS) ?? [],
  );

  const seccionesOrdenadas = useMemo(() => {
    const porId = new Map(SECCIONES.map((seccion) => [seccion.id, seccion]));
    const ordenadas = ordenSidebar
      .map((id) => porId.get(id))
      .filter((seccion): seccion is (typeof SECCIONES)[number] => seccion !== undefined);
    // Cubre secciones nuevas agregadas al código después de que este navegador
    // ya guardó un orden: aparecen al final en vez de desaparecer del menú.
    const faltantes = SECCIONES.filter((seccion) => !ordenSidebar.includes(seccion.id));
    return [...ordenadas, ...faltantes];
  }, [ordenSidebar]);

  const primeraVisible = seccionesOrdenadas.find((seccion) => !seccionesOcultas.includes(seccion.id));

  function reordenarSidebar(orden: Seccion[]) {
    setOrdenSidebar(orden);
    guardarListaSecciones(CLAVE_SIDEBAR_ORDEN, orden);
  }

  function alternarVisibilidadSeccion(id: Seccion, visible: boolean) {
    setSeccionesOcultas((actual) => {
      const siguiente = visible ? actual.filter((x) => x !== id) : [...actual, id];
      // Nunca ocultar la última sección visible: dejaría el menú vacío y sin
      // forma de deshacerlo desde la interfaz.
      if (siguiente.length >= SECCIONES.length) return actual;
      guardarListaSecciones(CLAVE_SIDEBAR_OCULTAS, siguiente);
      return siguiente;
    });
  }

  function restablecerSidebar() {
    const ordenPorDefecto = SECCIONES.map((seccion) => seccion.id);
    setOrdenSidebar(ordenPorDefecto);
    setSeccionesOcultas([]);
    guardarListaSecciones(CLAVE_SIDEBAR_ORDEN, ordenPorDefecto);
    guardarListaSecciones(CLAVE_SIDEBAR_OCULTAS, []);
  }

  // Si la sección activa se ocultó, hay que salir de ella: de lo contrario la
  // persona queda viendo una pantalla que ya no tiene entrada en el menú.
  useEffect(() => {
    if (seccionActual && seccionesOcultas.includes(seccionActual) && primeraVisible) {
      navigate(rutaSeccion(primeraVisible.id), { replace: true });
    }
  }, [seccionActual, seccionesOcultas, primeraVisible, navigate]);
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

  useEffect(() => {
    const etiqueta = SECCIONES.find((s) => s.id === seccionActual)?.etiqueta;
    document.title = etiqueta ? `${etiqueta} — Panel de Acceso` : "Panel de Acceso";
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
      <div className="flex h-full flex-col">
        <div className="flex flex-1 min-h-0">
          {menuMovilAbierto && (
            <div className="shell-sidebar-velo" onClick={() => setMenuMovilAbierto(false)} />
          )}

          <Sidebar
            secciones={seccionesOrdenadas}
            ocultas={seccionesOcultas}
            onReordenar={reordenarSidebar}
            onCambiarVisibilidad={alternarVisibilidadSeccion}
            onRestablecer={restablecerSidebar}
            // En mobile, elegir una sección cierra el cajón -- si no, tapa la
            // pantalla recién elegida hasta que la persona lo cierre a mano.
            onNavegar={() => setMenuMovilAbierto(false)}
            colapsado={colapsado}
            onToggleColapsado={alternarColapsado}
            abiertoEnMovil={menuMovilAbierto}
          />

          <main className="flex min-w-0 flex-1 flex-col">
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
            {!seccionActual && <Navigate to={rutaSeccion(primeraVisible?.id ?? "historial")} replace />}
            {visitadas.map((id) => (
              <div
                key={id}
                className={`min-h-0 flex-1 flex-col ${id === seccionActual ? "flex" : "hidden"}`}
              >
                {/* Suspense por sección, no uno compartido -- así la
                    primera visita a una sección NUEVA (bajando su chunk)
                    no vuelve a tapar con "Cargando pantalla…" las que ya
                    están montadas y visibles detrás. */}
                <Suspense fallback={<div className="pantalla-cuerpo" role="status">Cargando pantalla…</div>}>
                  {id === "historial" ? (
                    <Historial />
                  ) : id === "adentro" ? (
                    <AdentroAhora />
                  ) : id === "contratistas" ? (
                    <Contratistas />
                  ) : id === "usuarios" ? (
                    <Usuarios />
                  ) : id === "anfitriones" ? (
                    <Anfitriones />
                  ) : id === "sesiones" ? (
                    <Sesiones />
                  ) : id === "avisos" ? (
                    <Avisos />
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
