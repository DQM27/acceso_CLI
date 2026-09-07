/**
 * Aviso de "esta lista tiene más filas de las que se cargaron" -- ver el
 * campo `truncado` en `ResultadoHistorial`/`ResultadoContratistas`/
 * `ResultadoUsuarios` (`api/*.ts`). Un solo componente para las tres
 * pantallas que lo necesitan (antes iba a quedar copiado y pegado tres
 * veces, mismo estilo que ya usaba `desktop/src/pantallas/Historial.tsx`).
 */
export default function AvisoTruncado({ mensaje }: { mensaje: string }) {
  return (
    <p
      role="status"
      style={{
        margin: "0 0 0.5rem",
        padding: "0.5rem 0.75rem",
        borderRadius: "var(--radio-chico)",
        border: "1px solid var(--advertencia)",
        color: "var(--advertencia)",
        fontSize: "0.85rem",
      }}
    >
      {mensaje}
    </p>
  );
}
