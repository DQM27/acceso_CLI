import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { LimiteErrores } from "./componentes/Comunes";
import "./index.css";

const raiz = document.getElementById("root");
if (!raiz) {
  throw new Error("No se encontró el elemento #root en index.html");
}

const clienteConsultas = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 15_000,
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(raiz).render(
  <StrictMode>
    <LimiteErrores>
      <QueryClientProvider client={clienteConsultas}>
        <App />
      </QueryClientProvider>
    </LimiteErrores>
  </StrictMode>,
);
