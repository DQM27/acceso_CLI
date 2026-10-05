import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { LimiteErrores } from "./componentes/Comunes";
import { Avisos } from "./componentes/Avisos";
import { iniciarReglas } from "./reglas";
import "./index.css";

const raiz = document.getElementById("root");
if (!raiz) {
  throw new Error("No se encontró el elemento #root en index.html");
}

// Tema antes del primer render: sin destello claro en modo oscuro.
try {
  const tema = localStorage.getItem("brisas:tema");
  if (tema === "light" || tema === "dark") document.documentElement.setAttribute("data-theme", tema);
} catch {
  /* Sin almacenamiento: manda el tema del sistema. */
}

const clienteConsultas = new QueryClient({
  defaultOptions: { queries: { staleTime: 15_000, retry: 1, refetchOnWindowFocus: false } },
});

// Las reglas del núcleo (WebAssembly) empiezan a cargar ya, sin frenar la
// primera pantalla: el formulario las espera si todavía no llegaron.
iniciarReglas().catch((error: unknown) => console.error("No se pudieron cargar las reglas", error));

createRoot(raiz).render(
  <StrictMode>
    <LimiteErrores>
      <QueryClientProvider client={clienteConsultas}>
        <App />
        <Avisos />
      </QueryClientProvider>
    </LimiteErrores>
  </StrictMode>,
);
