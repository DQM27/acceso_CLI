# Módulo "Control T1"

Estado: **dominio casi cerrado — sin código todavía** (2026-11-23). Este
documento se fue cerrando por rondas con el dueño. Lo marcado como
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
| Un dato en la columna equivocada | Carreta `996137`, que es el marchamo `996137-38` | Lectura por cámara y validación de cada campo |
| Destino mal escrito | `PREDO` en vez de `PREDIO` | Catálogo de lugares |
| Horas fuera de orden | 03:55, 03:35, 03:58 | La hora la pone el reloj de la aplicación |
| Carreta distinta entre ingreso y salida (error de digitación confirmado por el dueño) | `36509` → `36209` (filas 1076 y 1082) | La salida reutiliza la unidad de arrastre del ingreso (R13) |

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
| **Lugar** | De dónde viene (en el ingreso) o a dónde va (en la salida). Sale del catálogo, o es **Otro** con el lugar escrito para un caso puntual |
| **Predio** | El lugar de donde vienen y a donde van los cabezales **vacíos** |
| **Ciclo T1** | Lo que va del reporte de un cabezal a su salida |
| **Reporte** | El chofer llega al puesto de control y se anuncia; espera a que lo llamen |
| **Anulación** | Cierre de un reporte que nunca entró. No borra nada: el registro queda, con el motivo |
| **Ingreso / Salida** | Los dos únicos movimientos |
| **Vacío / Cargado** | Estado de la carga en cada movimiento. Se registran solo los documentos (SAP y marchamos), **no** qué producto se transporta |
| `SM` / `SD` | "Sin marchamo" / "Sin documento": cómo se muestra un movimiento **vacío** |

## Ciclo de vida

```
                 ┌──(lo llaman)──► Adentro ──► Salió
Reportado ───────┤
                 └──(no entra)───► Anulado (con motivo)
```

1. **Reporte.** El chofer se reporta. El guarda captura chofer, cabezal,
   unidad de arrastre, carga de ingreso y origen mientras el chofer espera, y
   se abre el ciclo **sin hora de ingreso**.
2. **Ingreso.** Cuando lo llaman y entra, el guarda lo marca: en ese momento
   se coloca la **hora de ingreso** (la que hoy va en la columna `HORA`). Desde
   ahí el cabezal aparece en la lista de **cabezales adentro**.
3. **Anulación.** Si se reportó y no entró, el guarda **anula** el reporte
   indicando el motivo. Nunca se borra: queda el registro de que se reportó,
   que no entró y por qué.
4. **Salida.** El guarda elige el cabezal en la lista de adentro y responde
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

Lugar = Catalogo(lugar_id)          (Predio es una entrada fija del catálogo)
      | Otro(texto)                 (caso puntual, texto obligatorio)

Ciclo T1
  chofer            contratista del catálogo          ┐ se capturan una vez
  cabezal           placa                             │ (en el reporte) y la
  unidad_arrastre   { tipo: Carreta | Furgón, placa } ┘ salida los reutiliza
  reporte           { fecha_hora, guarda, carga: Carga, origen: Lugar }
  estado
    = Esperando                                   (reportado, todavía afuera)
    | Anulado  { fecha_hora, guarda, motivo }
    | Adentro  { ingreso: { fecha_hora, guarda } }
    | Salió    { ingreso: { fecha_hora, guarda },
                 salida:  { fecha_hora, guarda, carga: Carga, destino: Lugar } }
```

- Con `Carga` como tipo de dos variantes, es **imposible** registrar
  marchamos sin SAP, SAP sin marchamos o una lista de marchamos vacía. Es el
  mismo criterio que ya usa el proyecto con `SalidaRegistroIngreso`.
- Con `estado` como un solo tipo, es imposible un ciclo anulado que además
  tenga ingreso, una salida sin ingreso o una salida a medias (con hora pero
  sin destino, por ejemplo). Fecha y guarda de cada evento van juntos o no
  van.
- Cada evento guarda **su propio guarda** (id + nombre): quien recibe el
  reporte, quien marca el ingreso, quien anula y quien registra la salida
  pueden ser personas distintas (cambio de turno).

## Reglas

### Confirmadas por el dueño

| # | Regla |
|---|---|
| R1 | Solo hay dos movimientos: **ingreso** y **salida**. La fecha y hora las pone el reloj de la aplicación; no se editan. El guarda es el usuario de la sesión |
| R2 | El chofer **se reporta primero**: el ciclo se abre al reportarse y la **hora de ingreso** se coloca recién cuando entra |
| R3 | El chofer es un **contratista** de una empresa que ya existe en el catálogo |
| R4 | El cabezal se identifica **solo por su placa**. Los números de unidad que aparecen en la hoja (`1`, `10`, `11`, `46`) se ignoran |
| R5 | La unidad de arrastre es una **carreta** o un **furgón**, con su placa. **El guarda elige el tipo** en el reporte, porque de él depende el mínimo de marchamos |
| R6 | Una carga **cargada** lleva SAP y marchamos; una carga **vacía** no lleva ninguno de los dos (`SM`/`SD`). No se anota qué producto se transporta |
| R7 | Mínimo de marchamos de una carga cargada: **carreta ≥ 6**, **furgón ≥ 1** |
| R8 | **Vacío ⇔ Predio**: el vacío viene del predio o va al predio |
| R9 | El lugar sale de un **catálogo que administra el dueño**, más la opción **Otro** con el lugar escrito para un caso puntual |
| R10 | El SAP es el número **bajo el código de barras** del documento (hoy empieza con `90`) |
| R11 | **Sin longitud fija** para SAP ni marchamos: la cantidad de dígitos puede crecer o ser menor. Solo se exige que sean dígitos y que no estén vacíos |
| R12 | Cada marchamo se registra con su **número completo**, uno por uno. No se capturan rangos: hoy se anotan así por espacio en la hoja, pero los números pueden **saltarse** (no siempre son consecutivos) |
| R13 | La salida **reutiliza chofer, cabezal y unidad de arrastre** del ingreso |
| R14 | Al salir se pregunta **¿vacío o con carga?** Vacío: `SM`/`SD` y destino Predio, sin capturar nada más. Con carga: solo marchamos, SAP y destino |
| R15 | **No puede haber dos ciclos abiertos** (esperando o adentro) para la **misma placa de cabezal** ni para la **misma cédula** de chofer |
| R16 | El chofer **no lleva gafete** |
| R17 | Un reporte que no entra **se anula con un motivo**; nunca se borra |
| R18 | Las placas (cabezal y unidad de arrastre) son **placas de Costa Rica**; se **escanean** con el lector de placas o se **digitan a mano** |
| R19 | La exportación sigue **el mismo patrón** que las otras vistas de ingresos |

### Derivadas (revisar con el dueño)

| # | Regla | De dónde sale |
|---|---|---|
| D1 | Un SAP tecleado a mano que **no empiece con `90`** muestra un **aviso** ("no parece el número del código de barras, ¿es correcto?"), **no un bloqueo**. Atrapa el error de copiar el "Documento SAP" del mismo papel (ej. `7144455713`) sin volver rígido el formato | Lo más fácil de confundir en el documento de ejemplo; R11 |
| D2 | El SAP se lee del **código de barras**, no por OCR: es exacto | El documento lo trae en código de barras |
| D3 | El marchamo se guarda **tal como está impreso** (`0996362`) y se compara **sin ceros a la izquierda**, así `996362` y `0996362` son el mismo | La hoja anota sin el cero; el marchamo lo trae |
| D4 | La placa se guarda **normalizada** (mayúsculas, sin espacios ni guiones), tanto la escaneada como la digitada, para que R15 compare igual `C-179371`, `c 179371` y `C179371`. Sin formato rígido, igual que R11 | R15 y R18 |
| D5 | **Predio** es una entrada fija del catálogo (no se puede desactivar ni renombrar), porque R8 y R14 dependen de ella. Un movimiento con **Otro** siempre es cargado | R8, R9 |
| D6 | Al teclear varios marchamos, el siguiente campo puede **proponer el número anterior** para que solo se corrijan los últimos dígitos. Es ayuda de captura: el número se guarda completo igual | Los marchamos de una misma carga comparten el inicio (R12) |

### A futuro (fuera de alcance por ahora)

- **Cruce de envíos entre sitios**: lo que sale de un lugar debe llegar al
  otro con el mismo SAP y los mismos marchamos. Hoy T1 solo se usa en este
  sitio, así que no aplica todavía; el modelo no debe impedirlo.

## Reutilización técnica

| Necesidad | Componente existente |
|---|---|
| Chofer | Catálogo de contratistas y empresas; OCR de cédula (`mobile/rust-core/src/lectura_documentos/`) |
| SAP | Lector de códigos de barras de ML Kit, ya en la app (`barcode-scanning`, usado por `EscanerQr.kt`) |
| Placa de cabezal y de unidad de arrastre | Lector de placas de Costa Rica de `lectura_documentos/vehiculo.rs`. Hoy se llama "de ruta" por historia: se saca a un lector genérico para que T1 no dependa de nada de Rutas |
| Marchamo | **Nuevo**: OCR de solo dígitos, impreso en vertical sobre plástico curvo, con recuadro guía vertical |
| Exportación | El mismo mecanismo de las otras vistas de ingresos (selección de columnas y `.xlsx` del filtro vigente) |

## Pendientes

1. **PRAIND del chofer:** "lo tienen o deberían tenerlo". Si está **vencido o
   no lo tiene**, ¿se **bloquea** el reporte o solo se **advierte**? (Para un
   contratista normal, vencido bloquea y próximo a vencer advierte.)
2. **Acceso negado y empresa inactiva:** hoy todas las puertas respetan el
   acceso negado de un contratista (`plan-veto-por-persona.md`). Propuesta:
   T1 también, y también la empresa inactiva.
3. **Persona adentro por una sola vía:** propuesta: un chofer con un ciclo T1
   abierto no puede estar adentro a la vez como contratista, proveedor o por
   correo (T1 pasa a ser una vía más, igual que las otras).
4. **Salida sin ingreso registrado:** ¿qué se hace con un cabezal que sale
   sin haber entrado por la aplicación? Pasará al arrancar el módulo (los que
   ya están adentro) y si un día se olvida el ingreso.
5. **Motivo de anulación:** ¿texto libre o una lista corta (por ejemplo "se
   retiró", "no fue autorizado", "error de registro") más "Otro"?
