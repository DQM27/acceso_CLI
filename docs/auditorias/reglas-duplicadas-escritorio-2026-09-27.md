# Reglas de negocio duplicadas o faltantes en escritorio

Estado: **todo resuelto** en `claude/dominio-en-nucleo` (móvil) y
`claude/dominio-escritorio` (escritorio). El núcleo Rust decide las reglas
y los textos; escritorio (TS/Tauri) y móvil (Kotlin) sólo preguntan y
muestran. Queda como referencia de dónde vive cada regla.

| Regla | Dónde vive |
|---|---|
| Alta y edición de contratista (cédula, nombre, PRAIND, personal de ruta, alta con acceso) | `ContratistaService::armar`; el formulario pregunta a `reglas_formulario_contratista` |
| Ingreso de proveedor (cédula abierta en el sitio, activa en otro sitio, gafete en el otro dispositivo) | `application::registrar_ingreso_proveedor_verificado` (`src/application/con_nube.rs`) |
| Entrega de gafete provisional KOF (gafete prestado en el otro dispositivo) | `application::entregar_gafete_provisional_verificado` (`con_nube.rs`) |
| Placa según medio, placa y gafete obligatorios, bloqueo y aviso de PRAIND al ingresar | `domain::registro_ingreso::placa_segun_medio`, `RegistroIngresoService`, `PreparacionIngreso` |
| Aviso de la fila en listas ("ACCESO DENEGADO" / "PRAIND VENCIDO") | `domain::acceso::aviso_acceso_en_lista`, llenado en `AppCore::buscar_contratistas` (`ContratistaResumen::aviso_acceso`) |
