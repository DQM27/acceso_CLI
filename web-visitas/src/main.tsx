import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { LimiteErrores } from "./componentes/Comunes";
import "./index.css";

const raiz = document.getElementById("root");
if (!raiz) {
  throw new Error("No se encontró el elemento #root en index.html");
}

createRoot(raiz).render(
  <StrictMode>
    <LimiteErrores>
      <App />
    </LimiteErrores>
  </StrictMode>,
);
