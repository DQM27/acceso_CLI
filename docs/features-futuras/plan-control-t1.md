# Módulo "Control T1"

Estado: **en definición — dominio y reglas, sin código todavía** (2026-11-23).
Este documento se va cerrando por rondas con el dueño. Lo marcado como
**pendiente** todavía no es regla.

## Contexto

**T1** es como se llama a los cabezales que transportan producto entre la
planta de producción y las bodegas, internas y externas.

Hoy cada movimiento se registra en **dos pasos**: el guarda llena a mano una
hoja en el puesto de control y después alguien pasa esos datos a un Excel. El
objetivo del módulo es **dejarlo en un solo paso**: el guarda registra desde el
teléfono, en el momento, con lectura de placa, código de barras y marchamos.
El escritorio da paridad operativa como respaldo.

### Lo que hoy produce el doble paso

Columnas de la hoja actual: `FECHA`, `HORA`, `OPERADOR`, `CABEZAL`,
`CARRETA`, `MARCHAMOS`, `MOVIMIENTO`, `DESTINO`, `TRANSPORTE`, `GUARDIA`.

Errores que se ven en una muestra real (filas 1076–1087):

| Error | Ejemplo | Qué lo evita en el módulo |
|---|---|---|
| Un dato en la columna equivocada | Carreta `996137`, que es el marchamo `996137-38` | Lectura por cámara y formato validado de cada campo |
| Destino mal escrito | `PREDO` en vez de `PREDIO` | Catálogo de lugares |
| Horas fuera de orden | 03:55, 03:35, 03:58 | La hora la pone el reloj de la aplicación |
| Carreta distinta entre ingreso y salida | `36509` → `36209` | La salida reutiliza la unidad de arrastre del ingreso (R13) |

## T1 es un módulo propio

T1 **no comparte dominio** con Rutas, Proveedores, Visitas ni Por correo:
tiene su propia tabla, sus reglas y su pantalla. Del resto de la aplicación
solo usa el **catálogo de contratistas y empresas** (para el chofer) y
**componentes técnicos** de lectura (ver "Reutilización técnica").

## Lenguaje del dominio

| Término | Significado |
|---|---|
| **T1** | Cabezal que transporta producto entre la planta y las bodegas |
| **Chofer** | Quien maneja el cabezal. Es un contratista de una empresa que ya existe en el catálogo |
| **Cabezal** | Tractocamión. Se identifica **solo por su placa** |
| **Unidad de arrastre** | **Carreta** o **furgón**, con su placa |
| **Marchamo** | Sello físico que asegura la carga para que no se manipule. Lleva un número impreso (ej. `0996362`) |
| **SAP** | El número que está **bajo el código de barras** del documento de transporte; hoy empieza con `90` (ej. `901025393`). **No** confundir con el campo "Documento SAP" impreso en el mismo papel, que es otro número |
| **Ciclo T1** | Lo que va del ingreso de un cabezal a su salida. Se abre con el reporte y se cierra con la salida |
| **Lugar** | De dónde viene (en el ingreso) o a dónde va (en la salida) |
| **Predio** | El lugar de donde vienen y a donde van los cabezales **vacíos** |
| **Reporte** | El chofer llega al puesto de control y se anuncia; espera a que lo llamen |
| **Ingreso / Salida** | Los dos únicos movimientos |
| **Vacío / Cargado** | Estado de la carga en cada movimiento. Se registran solo los documentos (SAP y marchamos), **no** qué producto se transporta |
| `SM` / `SD` | "Sin marchamo" / "Sin documento": cómo la hoja muestra un movimiento **vacío** |

## Ciclo de vida

```
Reportado ──(lo llaman)──► Adentro ──► Salió
```

1. **Reporte.** El chofer se reporta. El guarda captura chofer, cabezal,
   unidad de arrastre, carga de ingreso y origen mientras el chofer espera, y
   se abre el ciclo **sin hora de ingreso**.
2. **Ingreso.** Cuando lo llaman y entra, el guarda lo marca: en ese momento
   se coloca la **hora de ingreso** (la que hoy va en la columna `HORA`). Desde
   ahí el cabezal aparece en la lista de **cabezales adentro**.
3. **Salida.** El guarda elige el cabezal en la lista de adentro y responde
   **una sola pregunta: ¿sale vacío o con carga?** Chofer, cabezal y unidad de
   arrastre se reutilizan del ingreso; no se vuelven a capturar.
   - **Vacío:** no se captura nada más. Queda `SM`/`SD` y el destino es
     **Predio** automáticamente.
   - **Con carga:** se capturan solo **marchamos, SAP y destino**.

   En los dos casos la fecha, la hora y el guarda que registra los pone la
   aplicación. La salida **cierra el ciclo**.

Un cabezal puede **entrar vacío o cargado** y **salir con otra carga o
vacío**: la carga de ingreso y la de salida son independientes.

## Modelo conceptual

```
Carga = Vacía
      | Cargada { sap, marchamos: [al menos uno] }

Ciclo T1
  chofer            contratista del catálogo          ┐ se capturan una vez
  cabezal           placa                             │ (en el reporte) y la
  unidad_arrastre   { tipo: Carreta | Furgón, placa } ┘ salida los reutiliza
  ingreso
    reportado_en    hora del reporte (la pone el reloj)
    guarda_reporte  usuario de la sesión (id + nombre)
    ingreso         None mientras espera | { fecha_hora, guarda }
    carga           Carga
    origen          catálogo de lugares
  salida            None mientras está adentro
                  | { fecha_hora, guarda, carga: Carga, destino: lugar }
```

- Con `Carga` como tipo de dos variantes, es **imposible** registrar
  marchamos sin SAP, SAP sin marchamos o una lista de marchamos vacía. Es el
  mismo criterio que ya usa el proyecto con `SalidaRegistroIngreso`.
- Que la salida sea **una sola parte opcional** con todos sus datos juntos
  hace imposible una salida a medias (con hora pero sin destino, por
  ejemplo). Lo mismo con el ingreso: hora y guarda van juntos o no van.
- Cada evento guarda **su propio guarda**: quien recibe el reporte, quien
  marca el ingreso y quien registra la salida pueden ser personas distintas
  (cambio de turno).

## Reglas

### Confirmadas por el dueño

| # | Regla |
|---|---|
| R1 | Solo hay dos movimientos: **ingreso** y **salida**. La fecha y hora las pone el reloj de la aplicación; no se editan. El guarda es el usuario de la sesión |
| R2 | El chofer **se reporta primero**: el registro se crea al reportarse y la **hora de ingreso** se coloca recién cuando entra |
| R3 | El chofer es un **contratista** de una empresa que ya existe en el catálogo |
| R4 | El cabezal se identifica **solo por su placa**. Los números de unidad que aparecen en la hoja (`1`, `10`, `11`, `46`) se ignoran |
| R5 | La unidad de arrastre es una **carreta** o un **furgón**, con su placa |
| R6 | Una carga **cargada** lleva SAP y marchamos; una carga **vacía** no lleva ninguno de los dos (`SM`/`SD`). No se anota qué producto se transporta |
| R7 | Mínimo de marchamos de una carga cargada: **carreta ≥ 6**, **furgón ≥ 1** |
| R8 | **Vacío ⇔ Predio**: el vacío viene del predio o va al predio |
| R9 | El lugar sale de un **catálogo casi cerrado**: muy pocas veces se va a otro lado |
| R10 | El SAP es el número **bajo el código de barras** del documento (hoy empieza con `90`) |
| R11 | **Sin longitud fija** para SAP ni marchamos: la cantidad de dígitos puede crecer o ser menor. Solo se exige que sean dígitos y que no estén vacíos |
| R12 | Cada marchamo se registra con su **número completo**, uno por uno. No se capturan rangos: hoy se anotan así por espacio en la hoja, pero los números pueden **saltarse** (no siempre son consecutivos) |
| R13 | El ciclo se abre en el ingreso y se cierra con la salida. La salida **reutiliza chofer, cabezal y unidad de arrastre** del ingreso |
| R14 | Al salir se pregunta **¿vacío o con carga?** Vacío: `SM`/`SD` y destino Predio, sin capturar nada más. Con carga: solo marchamos, SAP y destino |

### Derivadas de los ejemplos (revisar con el dueño)

| # | Regla | De dónde sale |
|---|---|---|
| D1 | Un SAP tecleado a mano que **no empiece con `90`** muestra un **aviso** ("no parece el número del código de barras, ¿es correcto?"), **no un bloqueo**. Atrapa el error de copiar el "Documento SAP" del mismo papel (ej. `7144455713`) sin volver rígido el formato | Lo más fácil de confundir en el documento de ejemplo; R11 |
| D2 | El SAP se lee del **código de barras**, no por OCR: es exacto | El documento lo trae en código de barras |
| D3 | El marchamo se guarda **tal como está impreso** (`0996362`) y se compara **sin ceros a la izquierda**, así `996362` y `0996362` son el mismo | La hoja anota sin el cero; el marchamo lo trae |
| D4 | Un cabezal (por placa) y un chofer (por cédula) **no pueden tener dos ciclos abiertos** a la vez | Sin esto, la lista de "adentro" tendría duplicados y la salida no sabría qué ciclo cerrar |
| D5 | Al teclear varios marchamos, el siguiente campo puede **proponer el número anterior** para que solo se corrijan los últimos dígitos. Es ayuda de captura: el número se guarda completo igual | Los marchamos de una misma carga comparten el inicio (R12) |

### A futuro (fuera de alcance por ahora)

- **Cruce de envíos entre sitios**: lo que sale de un lugar debe llegar al
  otro con el mismo SAP y los mismos marchamos. Hoy T1 solo se usa en este
  sitio, así que no aplica todavía; el modelo no debe impedirlo.

## Reutilización técnica

| Necesidad | Componente existente |
|---|---|
| Chofer | Catálogo de contratistas y empresas; OCR de cédula (`mobile/rust-core/src/lectura_documentos/`) |
| SAP | Lector de códigos de barras de ML Kit, ya en la app (`barcode-scanning`, usado por `EscanerQr.kt`) |
| Placa de cabezal y de unidad de arrastre | Lector de placas de `lectura_documentos/vehiculo.rs`. Hoy se llama "de ruta" por historia: se saca a un lector genérico para que T1 no dependa de nada de Rutas |
| Marchamo | **Nuevo**: OCR de solo dígitos, impreso en vertical sobre plástico curvo, con recuadro guía vertical |

## Pendientes

1. **Reglas de acceso del chofer:** ¿se le aplican las mismas que a un
   contratista (PRAIND, acceso negado, empresa activa)? ¿Cuenta como persona
   "adentro"? (Hoy una misma cédula no puede estar adentro por dos vías a la
   vez.) ¿Lleva gafete?
2. **Salida sin ingreso registrado:** ¿qué se hace con un cabezal que sale
   sin haber entrado por la aplicación? Pasará al arrancar el módulo (los que
   ya están adentro) y si un día se olvida el ingreso.
3. **Placas:** formato impreso de la placa del cabezal y de la carreta o el
   furgón (la hoja guarda solo los dígitos). Hace falta una foto de cada una.
4. **Tipo de unidad de arrastre:** ¿lo elige el guarda en cada movimiento?
5. **Marchamo de un solo uso:** ¿un mismo número puede repetirse entre tipos
   de marchamo distintos? Faltan fotos de los otros tipos para calibrar el OCR.
6. **Catálogo de lugares:** lugares iniciales (Predio, Calle Blancos,
   Liberia, …) y quién da de alta uno nuevo.
7. **Reporte sin ingreso:** ¿qué pasa si alguien se reporta y nunca entra?
   ¿Se anula a mano o vence solo?
8. **Excel:** ¿se sigue necesitando una exportación con las mismas columnas
   de la hoja? Si es así: un ciclo serían dos filas (INGRESA y SALE) y los
   marchamos irían con su número completo, separados por coma.
