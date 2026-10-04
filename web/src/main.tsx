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

// Las reglas del núcleo (WebAssembly, ver src/reglas) se cargan antes de
// mostrar el panel: los formularios las consultan de forma sincrónica. Si no
// cargan (navegador sin WebAssembly), el panel igual se muestra y el error
// queda en la consola; guardar sigue validándose en el servidor.
iniciarReglas()
  .catch((error: unknown) => console.error("No se pudieron cargar las reglas del núcleo:", error))
  .finally(() => mostrarPanel(raiz));

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
