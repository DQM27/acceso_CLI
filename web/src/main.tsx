import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import ErrorBoundary from "./componentes/ErrorBoundary";
import "./index.css";
import { iniciarReglas } from "./reglas";

const raiz = document.getElementById("root");
if (!raiz) {
  throw new Error("No se encontró el elemento #root en index.html");
}

// Un solo cliente para todo el panel -- Realtime invalida consultas en vez
// de volver a pedir listas completas a mano (ver auditoria-panel-web,
// D3/Cambios técnicos #1).
const clienteConsultas = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      refetchOnWindowFocus: false,
    },
  },
});

// Las reglas del núcleo (WebAssembly, ver src/reglas) empiezan a cargarse ya,
// sin frenar la primera pantalla: sólo las usan los formularios, que esperan
// si hace falta (`useEstadoReglas`).
iniciarReglas().catch((error: unknown) => console.error("No se pudieron cargar las reglas del núcleo:", error));
mostrarPanel(raiz);

function mostrarPanel(raiz: HTMLElement) {
  createRoot(raiz).render(
    <StrictMode>
      <ErrorBoundary>
        <QueryClientProvider client={clienteConsultas}>
          <App />
        </QueryClientProvider>
      </ErrorBoundary>
    </StrictMode>,
  );
}
