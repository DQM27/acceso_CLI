# Auditoría y reparación integral de Android

Fecha: 2026-09-09  
Base: `fix/ocr-arquitectura-concurrencia` (`208944f`)  
Rama: `fix/android-arquitectura-integral`

## Alcance

Se revisó la aplicación Android completa: arranque, sesión local, alcance de estado, SQLite/UniFFI, sincronización y Realtime, operaciones de ingreso/salida, OCR/CameraX, almacenamiento del secreto, historial, interfaz, manifiesto y pruebas. La reestructuración de autenticación coordinada con Supabase queda deliberadamente pendiente; se documenta al final.

## Reparaciones aplicadas

| Área | Riesgo encontrado | Decisión aplicada |
|---|---|---|
| Arranque | Abrir SQLite/UniFFI en el hilo principal podía congelar la primera pantalla; una rotación podía crear otro `Nucleo` mientras los ViewModel retenían el anterior. | `AplicacionViewModel` abre en `Dispatchers.IO`, publica carga/error y conserva una sola instancia hasta el cierre definitivo. |
| Sesiones | Los ViewModel de Activos, Historial y Nube pertenecían a la Activity y sobrevivían al cierre de sesión. | Cada sesión recibe su propio `ViewModelStore`; al salir se limpia antes de olvidar el actor. |
| Ciclo de vida | El observador podía instalarse después de `ON_START`; Realtime y el pulso no arrancaban hasta otro ciclo. En segundo plano la sesión quedaba abierta indefinidamente. | Se reconcilia el estado actual al instalar el observador, se detienen ambos servicios en `ON_STOP` y se cierra la sesión tras dos minutos fuera de primer plano. |
| Cámara/OCR | Al salir se liberaba `ImageAnalysis`, pero `Preview` seguía ligado a la Activity. | Se conservan y desligan ambos casos de uso; se cancelan callbacks/trabajos y Atrás cierra el escáner. |
| Mutaciones | Salida manual, salida masiva y escaneo automático podían escribir en paralelo; dobles toques iniciaban tareas repetidas. | Un `Mutex` común serializa las salidas y las banderas se levantan antes de lanzar corrutinas. Login, sincronización, activación e ingreso también rechazan reentrada. |
| Búsquedas | Durante el debounce se mostraban falsos “sin resultados”; una respuesta vieja podía interferir con el indicador de carga. | Estado explícito `cargando` y versión monotónica por búsqueda; sólo la versión vigente apaga el indicador. |
| Fechas remotas | Un RFC3339 malformado derribaba Historial o una fila activa. | Parseo tolerante: datos inválidos se muestran como “Fecha no disponible” y se ordenan al final. |
| Secreto | La escritura no era atómica y un archivo truncado se interpretaba como ausencia de configuración. | Archivo temporal, `fsync` y reemplazo atómico; un formato corrupto produce error recuperable visible. El secreto sigue cifrado con Android Keystore. |
| Activación | Primero se mutaba el estado remoto y luego se persistía el secreto, dejando una ventana difícil de recuperar. | Se persiste el secreto antes de la operación remota para permitir reintento seguro. |
| SQLite | Las conexiones secundarias no activaban `foreign_keys`, porque el PRAGMA es por conexión. | Toda conexión secundaria activa claves foráneas además de `busy_timeout`. |
| Privacidad | Capturas y miniatura de aplicaciones recientes podían mostrar cédulas e historial. | `FLAG_SECURE`; backups ya estaban deshabilitados y ahora también se prohíbe tráfico HTTP claro. |
| Dispositivos/UI | La cámara figuraba como requisito de instalación; formularios podían quedar ocultos por el teclado o desbordarse. | Cámara opcional en manifiesto y formularios desplazables con espacio para IME. |

## Efecto esperado en rendimiento y batería

Sí ayuda, especialmente durante uso repetido y al mandar la app a segundo plano:

- liberar `Preview` y `ImageAnalysis` evita que la cámara, el proveedor y el pipeline gráfico queden retenidos;
- detener Realtime y el pulso periódico en segundo plano elimina red, temporizadores y despertares sin utilidad visible;
- mantener un único `Nucleo` evita conexiones y estado nativo duplicados durante rotaciones;
- debounce y cancelación reducen consultas SQLite por tecla;
- serializar mutaciones evita contención y reintentos por carreras.

`FLAG_SECURE`, el parseo tolerante y la escritura atómica del secreto tienen costo despreciable en operación normal. No se declara un porcentaje: para cuantificarlo se debe comparar la base y esta rama en el Samsung A25, con igual guion, brillo y red, usando Energy Profiler o Perfetto/Battery Historian.

## Validación

La automatización del repositorio ejecuta en cada `push` a `fix/**`: núcleo Rust para host, tests Android/Kotlin contra el núcleo real, build/tests Rust generales y tests de escritorio/diseño. Se agregaron casos para fechas RFC3339 válidas/malformadas, aislamiento de sesión y doble intento de autenticación.

Validación manual recomendada: rotar durante login y búsqueda; abrir/cerrar OCR veinte veces; alternar fondo/primer plano antes y después de dos minutos; ejecutar salida manual y escaneada casi simultáneamente; cortar el proceso durante activación.

## Pendiente coordinado con Supabase: autenticación

### Problema

Hoy un usuario sincronizado llega con `SIN_PASSWORD_LOCAL`. Conocer la cédula permite fijar una contraseña nueva en ese teléfono, incluso para una cuenta privilegiada. Repararlo sólo en Android rompería compatibilidad con escritorio/panel y no resolvería identidad entre dispositivos.

### Operación propuesta

1. En Supabase, agregar una invitación de enrolamiento de un solo uso con `user_id`, `device_id`, expiración, estado y hash del token; nunca guardar el token en claro.
2. Hacer que Root/Admin autenticado emita la invitación desde el panel, y que el proceso controlado de activación entregue la invitación inicial del primer Root.
3. Cambiar Rust para que `fijarPasswordInicial` exija y consuma el token en una transacción remota antes de guardar el hash local.
4. Actualizar Android y escritorio juntos: cédula + token de enrolamiento + contraseña nueva. No revelar si una cédula privilegiada existe.
5. Revocar invitaciones al desactivar usuario/dispositivo, auditar emisión/consumo y limitar intentos.
6. Migrar instalaciones: permitir el flujo legado sólo durante una ventana explícita y con autorización administrativa; después eliminar `SIN_PASSWORD_LOCAL` como autorización suficiente.

También queda para esa intervención la reserva atómica de gafetes entre dispositivos. La consulta previa reduce errores, pero sólo una función RPC/transacción en Supabase puede garantizar que dos teléfonos no reserven el mismo gafete simultáneamente.

## Pendiente de endurecimiento adicional

- Cifrado integral de `control_acceso.db` (SQLCipher o equivalente), con migración y recuperación probadas. Keystore protege el secreto, pero SQLite contiene datos personales en reposo.
- Pruebas instrumentadas con Activity/CameraX en emulador o dispositivo; los tests JVM no demuestran liberación física de cámara ni comportamiento del sistema al capturar pantalla.
- Perfil de batería y tiempo de arranque en el Samsung A25 antes de fijar objetivos cuantitativos.
