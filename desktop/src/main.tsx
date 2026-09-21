import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import ErrorBoundary from "./componentes/ErrorBoundary";
import "./index.css";

// El menú contextual nativo del WebView (Atrás/Actualizar/Guardar como/
// Imprimir/Más herramientas) es el de un navegador -- no aplica a esta app
// de escritorio y confundía al usuario (pedido explícito, 2026-09-21). Se
// apaga acá, a nivel documento, en vez de en cada pantalla. El único menú
// contextual real de la app (reordenar/ocultar íconos del sidebar,
// `Sidebar.tsx`) sigue funcionando -- su propio `onContextMenu` llama
// `preventDefault()` igual y abre su menú antes de que este listener
// corra en la fase de burbujeo, así que no compiten.
document.addEventListener("contextmenu", (evento) => evento.preventDefault());

const raiz = document.getElementById("root");
if (!raiz) {
  throw new Error("No se encontró el elemento #root en index.html");
}

createRoot(raiz).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
