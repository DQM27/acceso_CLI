# Muestras OCR aisladas

Estas muestras se conservaron como referencia, pero no forman parte del lector
activo de la app móvil.

## Carnet KOF rojo

Fecha de muestra: 2026-09-08.

Motivo de aislamiento:
- Pertenece a otro proyecto.
- El número visible de siete dígitos es un código interno, no la cédula que
  debe usar esta app para buscar contratistas.
- El reverso puede traer la cédula, pero no debe activarse este perfil en el
  flujo actual hasta definir el uso correcto en el proyecto correspondiente.

Señales observadas en la foto:
- Portacarnet rojo.
- Nombre en dos líneas.
- Código interno visible: `5040017`.
- Texto de emergencia compartido con otros carnets: `CENTRAL COSTA RICA DE
  ALERTA Y RESPUESTA` y `800-2256327`.
- Código lateral largo impreso por el proveedor del plástico.

Decisión:
- No agregar `KOF` como `TipoDocumento` del lector móvil actual.
- No usar el código interno como `numeroDocumento` en esta app.
- Mantener esta nota para retomar el perfil fuera de este proyecto.

## Carnet Coca-Cola FEMSA frontal

Fecha de muestra: 2026-09-08.

Motivo de aislamiento:
- Pertenece al mismo perfil KOF/FEMSA anterior y queda fuera del lector activo
  de esta app.
- El frente no muestra código de empleado; muestra nombre y marca.
- Para el proyecto que sí use este perfil, el frente debe priorizar búsqueda
  por nombre.
- El código de empleado debe leerse sólo del reverso cuando esté disponible.

Señales observadas en la foto:
- Marca `Coca-Cola FEMSA` visible en la camisa y en la parte inferior.
- Foto de la persona con camisa roja.
- Bandera de Costa Rica.
- Nombre visible en dos líneas: `Brasly Daniel Chaves Bonilla`.
- Texto decorativo de antigüedad: `5 AÑOS`.

Decisión:
- No activar este perfil en el OCR móvil actual.
- Documentarlo como perfil aislado para un proyecto futuro donde el flujo use
  nombre del frente y código de empleado del reverso.
