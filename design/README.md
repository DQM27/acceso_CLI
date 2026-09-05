# Sistema de diseño Brisas

Brisas utiliza superficies en grafito y un resaltado azul marino. El panel lateral se diferencia del cuerpo de las tablas y los encabezados tienen un nivel más claro. La fuente de verdad es **[brisas.json](brisas.json)**. Las reglas de los botones web están en **[controles.css](controles.css)**. No editar los archivos marcados como generados.

Abrir **[catalogo.html](catalogo.html)** en el navegador para comparar temas, colores, botones, campos, estados, tablas y superficies elevadas. Funciona sin servidor ni conexión. Tab permite revisar el foco; los botones Claro y Oscuro cambian la muestra completa.

## Cambiar el diseño

1. Editar los valores de `brisas.json` y, si cambia el comportamiento visual de los botones, `controles.css`.
2. Desde la raíz ejecutar `node design/generar.mjs`.
3. Ejecutar `node design/generar.mjs --check` y `node design/verificar.mjs`.
4. Revisar el catálogo en ambos temas, compilar las plataformas afectadas y guardar también las salidas generadas en el mismo cambio.

En escritorio están disponibles `npm run design:generate` y `npm run design:check`. CI comprueba que las salidas coincidan con la fuente y verifica contraste. Cambiar el archivo maestro requiere regenerar y reconstruir la aplicación correspondiente; no es una configuración remota ni se modifica una instalación ya distribuida.

## Colores y superficies

| Rol | Uso |
| --- | --- |
| `fondo` | Plano inferior de la aplicación |
| `panel` | Tarjetas y cuerpo de tablas |
| `panel-lateral` | Navegación lateral, diferenciada de las tablas |
| `panel-suave` | Encabezados y agrupaciones secundarias |
| `elevado` | Menús y modales, con `sombra-panel` |
| `campo-fondo` | Controles de entrada y filas alternas |
| `borde` | Separación decorativa entre superficies |
| `borde-fuerte` | Controles que necesitan un contorno reconocible |
| `texto` / `muted` | Texto principal y complementario |
| `acento` | Texto e indicadores de foco: marino en claro, azul claro desaturado en oscuro para legibilidad |
| `acento-relleno` | Azul marino para botones principales y selección de navegación, con texto `sobre-acento` |
| `sobre-acento-indicador` | Contraste inverso del acento para componentes Material |
| `acento-suave` | Fondo de selección |
| `exito` / `advertencia` / `error` / `info` | Resultado o estado explícito |
| `sobre-*` | Texto sobre un relleno sólido de ese rol |
| `*-suave` | Fondo de un mensaje o indicador de estado |

No usar el acento como sustituto de éxito. Los estados llevan etiqueta o icono además del color. `borde` puede ser sutil porque no identifica por sí solo un control; para eso se utiliza `borde-fuerte`. La prueba de contraste cubre texto normal, botones primarios y mensajes en sus fondos previstos; no certifica toda la accesibilidad de una pantalla.

## Botones y formas

| Variante web | Uso |
| --- | --- |
| `boton boton-primario` / `btn-primary` | Acción principal del contexto, relleno de acento |
| `boton` | Acción secundaria, superficie neutra y borde |
| `boton boton-discreto` / `btn-ghost` | Acción de menor jerarquía |
| `boton boton-peligro` / `btn-danger-ghost` | Acción destructiva, texto explícito y color de error |
| `boton boton-compacto` | Acción dentro de una tabla |

Todas las variantes tienen estados hover, pulsación, foco visible y deshabilitado. Usar `disabled` real. No eliminar el foco. Respetar reducción de movimiento. Los botones primarios tienen texto `sobre-acento`, también al pasar el cursor y al presionarlos.

- Controles: radio de **8 px**; tarjetas, menús y modales: **12 px**.
- Cápsulas: indicadores e interruptores, no botones de acción generales.
- Alturas mínimas: **36 px** normal, **28 px** compacto, **48 px/dp** táctil.
- Iconos de botón web: **18 px**; separación del texto: **8 px**.
- Escala de espacio: **4, 8, 12, 16, 24, 32**.
- Texto de control: **14 px/sp**, peso **600**; título de referencia: **20 px/sp**.

La familia es sans serif del sistema: Segoe UI en Windows, Roboto en Android y alternativas del navegador. Esto comparte categoría y jerarquía sin descargar fuentes. Los tamaños de lectura de pantallas existentes pueden tener ajustes locales por densidad; las medidas de controles y las formas comunes salen del maestro. La terminal utiliza la fuente monoespaciada configurada por el usuario y mide en celdas, no en píxeles.

## Adaptaciones

| Plataforma | Integración |
| --- | --- |
| Escritorio | `desktop/src/diseno.css` y `controles.css`; `index.css` mantiene estructura y componentes específicos |
| Android | `DisenoGenerado.kt` define ambos esquemas Material, formas y tipografía; `ControlesBrisas.kt` unifica botones |
| Panel web | Bloque `brisas-generado` incrustado en el HTML para conservar su distribución como archivo único |
| TUI | `src/diseno_generado.rs`; preferencias históricas `classic` → claro, `brisas` → oscuro, `negro` → oscuro con pestañas |
| CLI | Misma paleta RGB; oscuro predeterminado, `BRISAS_THEME=light` antes del arranque para claro |

La variante TUI con pestañas conserva su navegación, no una tercera paleta. La variable de CLI se lee una vez al iniciar su representación. Los colores RGB requieren una terminal con soporte de color verdadero; terminales limitadas pueden aproximarlos.

Los selectores segmentados e iconos táctiles conservan la interacción de Material. Los botones de acción utilizan los envoltorios Brisas. Al añadir una variante nueva, incorporarla a los componentes compartidos y al catálogo antes de utilizarla en pantallas. No introducir colores hexadecimales ni radios propios en una pantalla nueva.
