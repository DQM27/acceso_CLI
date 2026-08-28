import { useEffect, useState } from "react";
import Login from "./pantallas/Login";
import Contratistas from "./pantallas/Contratistas";
import Empresas from "./pantallas/Empresas";
import Usuarios from "./pantallas/Usuarios";
import { cerrarSesion, requiereConfiguracionInicial } from "./api";
import type { UsuarioSesion } from "./api";

type Pantalla =
  | { tipo: "cargando" }
  | { tipo: "requiere-configuracion-inicial" }
  | { tipo: "login" }
  | { tipo: "shell"; sesion: UsuarioSesion };

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

type Seccion = "contratistas" | "empresas" | "usuarios";

const SECCIONES: { id: Seccion; etiqueta: string }[] = [
  { id: "contratistas", etiqueta: "Contratistas" },
  { id: "empresas", etiqueta: "Empresas" },
  { id: "usuarios", etiqueta: "Usuarios" },
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
  const [seccion, setSeccion] = useState<Seccion>("contratistas");

  return (
    <div style={{ display: "flex", height: "100%" }}>
      <nav
        style={{
          width: "13rem",
          flexShrink: 0,
          background: "var(--panel)",
          borderRight: "1px solid var(--borde)",
          display: "flex",
          flexDirection: "column",
          padding: "1rem 0",
        }}
      >
        <div style={{ padding: "0 1rem 1rem", color: "var(--acento)", fontWeight: 600 }}>
          Brisas
        </div>

        {SECCIONES.map((item) => (
          <button
            key={item.id}
            onClick={() => setSeccion(item.id)}
            className={`nav-item ${seccion === item.id ? "nav-item-activo" : ""}`}
          >
            {item.etiqueta}
          </button>
        ))}

        <div style={{ flex: 1 }} />

        <div style={{ padding: "0 1rem" }}>
          <div style={{ color: "var(--muted)", fontSize: "0.8rem", marginBottom: "0.5rem" }}>
            {sesion.nombre} ({sesion.rol})
          </div>
          <button className="boton" style={{ width: "100%" }} onClick={onCerrarSesion}>
            Cerrar sesión
          </button>
        </div>
      </nav>

      <main style={{ flex: 1, minWidth: 0 }}>
        {seccion === "contratistas" && <Contratistas />}
        {seccion === "empresas" && <Empresas />}
        {seccion === "usuarios" && <Usuarios />}
      </main>
    </div>
  );
}
