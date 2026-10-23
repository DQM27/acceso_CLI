import type { ReactNode } from "react";
import { trazoEngranaje } from "./Engranaje.logica";

const TRAZO = trazoEngranaje(8, 47, 37);

/** Un engranaje con `children` (una inicial) en el centro. Los colores los
 * pone quien lo usa por CSS (`.engranaje`, ver `index.css`). */
export default function Engranaje({ children }: { children: ReactNode }) {
  return (
    <span className="engranaje" aria-hidden="true">
      <svg viewBox="0 0 100 100" focusable="false">
        <path d={TRAZO} />
      </svg>
      <span className="engranaje-centro">{children}</span>
    </span>
  );
}
