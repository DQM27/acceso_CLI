# Kiosco con Alacritty

Reemplaza la consola de Windows por [Alacritty](https://alacritty.org/)
(terminal acelerado por GPU) para correr `control_acceso.exe` — sin fusionar
ningún código: Alacritty sigue siendo el binario que descargaste, `main.rs`
sólo lo relanza a sí mismo adentro de esa ventana la primera vez que arranca.

## Cómo funciona

Al iniciar, `control_acceso.exe` busca `Alacritty.exe` en su misma carpeta.
Si lo encuentra (y todavía no se relanzó), lanza Alacritty con este archivo
de config (si también está al lado) y con sí mismo como comando (`-e`), y el
proceso original termina. El que queda corriendo es el hijo, ya dentro de la
ventana GPU de Alacritty — el usuario ve un solo doble clic, un solo
programa. Sin `Alacritty.exe` al lado, arranca exactamente igual que hoy, en
la consola que Windows ofrezca.

Este flujo NO se activa con `--reset-root`: ese flag es de
consola/recuperación (prompts de contraseña en línea), no la experiencia de
kiosco.

## Armar la carpeta

1. Descargar el `.zip` portable de Alacritty para Windows desde
   [su página de releases](https://github.com/alacritty/alacritty/releases)
   y extraer `Alacritty.exe` (así, con mayúscula inicial — Windows no
   distingue mayúsculas para encontrarlo, pero así viene el `.zip` oficial).
2. Compilar el release:
   ```powershell
   cargo build --release
   ```
3. Juntar los tres archivos en una misma carpeta (podés reusar
   `target\release\` mientras probás, o una carpeta de distribución aparte):
   ```powershell
   Copy-Item packaging\alacritty\alacritty.toml target\release\
   Copy-Item <ruta-al-zip-extraido>\Alacritty.exe target\release\
   ```
4. Ejecutar `target\release\control_acceso.exe` (o el acceso directo que ya
   tengas apuntando ahí) — debería abrirse dentro de Alacritty, en una
   ventana normal (con barra de título y bordes).

## Ajustar la config

`alacritty.toml` ya trae la paleta calcada de `FADE_*` en
`src/cli/render/estilos.rs` (mismos colores que usan los fundidos de la app).
Con `general.live_config_reload = true`, cualquier cambio al archivo se
aplica sin reiniciar Alacritty — cómodo para seguir ajustando fuente o
colores a ojo.

Si en algún momento querés el look de kiosco (portería, sin que el usuario
pueda mover/cerrar la ventana con la barra de título) en vez de ventana
normal, cambiá `[window] decorations = "None"` y `startup_mode =
"Maximized"` — o más rápido: **F11 o Alt+Enter alternan pantalla completa
en caliente**, sin editar el archivo (ya vienen mapeados en
`[[keyboard.bindings]]`).

Otros ajustes que ya trae este archivo:
- `font.builtin_box_drawing = true` — las líneas de tabla (`─│├┤`) se
  dibujan parejas sin depender de la fuente.
- `mouse.hide_when_typing = true` — el cursor de flecha no queda flotando
  en medio de la pantalla mientras se teclea.
- `selection.save_to_clipboard = false` — seleccionar texto (hay cédulas y
  nombres a la vista) no lo copia solo al portapapeles. Ojo: Alacritty no
  tiene forma de desactivar la selección en sí, esto sólo evita el copiado
  automático — no reemplaza controlar quién tiene acceso físico a la
  máquina.

Alacritty no tiene pestañas ni múltiples ventanas propias en ningún modo —
no hay nada que desactivar ahí. Que sólo pueda haber una instancia corriendo
ya lo garantiza `InstanciaGuard` dentro de la propia app (un candado de
archivo independiente de Alacritty): si alguien intenta abrir una segunda,
falla al arrancar en vez de convivir con la primera.

## Ícono de la barra de tareas

Por defecto el ícono que aparece en la barra de tareas es el de Alacritty —
Windows lo toma del recurso de ícono embebido en `Alacritty.exe`, así que no
es algo que se cambie desde `alacritty.toml`. Para que muestre el ícono de
Brisas en su lugar hay que reescribir ese recurso directamente en el `.exe`
descargado (no toca nada de Alacritty en sí, sólo su ícono):

1. Bajar [`rcedit`](https://github.com/electron/rcedit/releases) (una sola
   herramienta de línea de comandos, la usa el propio proyecto Electron para
   lo mismo) y ponerlo en el `PATH` o al lado del `.exe`.
2. Generar el `.ico` de Brisas si todavía no existe uno suelto (el script ya
   lo arma junto al resto de recursos MSIX):
   ```powershell
   & .\packaging\generate-icons.ps1
   ```
3. Aplicarlo sobre la copia de Alacritty que vas a distribuir (no toca la
   instalación "oficial" si tenés otra en el sistema) — es el mismo
   `assets\icon.ico` que ya usa `build.rs` para el ícono de
   `control_acceso.exe`:
   ```powershell
   rcedit Alacritty.exe --set-icon assets\icon.ico
   ```

Repetir el paso 3 cada vez que descargues una versión nueva de Alacritty.
