import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { LimiteErrores } from "./componentes/Comunes";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <LimiteErrores>
      <App />
    </LimiteErrores>
  </StrictMode>,
);
