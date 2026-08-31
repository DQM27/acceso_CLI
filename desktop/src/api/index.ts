// Barrel: cada pantalla sigue importando de "../api" sin saber que por
// dentro está separado por dominio (mismo criterio que comandos/ del lado
// Rust — un archivo por área de negocio, nada de archivo único creciendo sin
// límite).

export * from "./actualizaciones";
export * from "./auditoria";
export * from "./autenticacion";
export * from "./contratistas";
export * from "./empresas";
export * from "./gafetes";
export * from "./historial";
export * from "./ingresos";
export * from "./usuarios";
