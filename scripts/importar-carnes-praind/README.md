# Importar carnés PRAIND desde una carpeta

Fase 5 de [plan-fotos-praind.md](../../docs/features-futuras/plan-fotos-praind.md).
Sirve para la carpeta de carnés que se van juntando (PNG, JPEG o WebP). Se
puede correr las veces que haga falta: lo que ya quedó bien no se repite.

## Qué hace

1. **Lee cada carné** con el OCR de Windows en español (no instala nada): el
   nombre, la cédula, la empresa y las fechas de inducción y vencimiento.
2. **Repetidos.** Si una persona tiene varios carnés (el sitio da un enlace
   nuevo en cada inducción), queda el que vence más tarde.
3. **Busca a la persona** en la nube por cédula. Si no la encuentra, la busca
   por nombre (por si la cédula del carné o la guardada están mal).
4. **Empresas.** Si la del carné ya existe (sin importar mayúsculas, tildes
   ni el "S.A."), se usa esa. Si no, se crea.
5. **Contratista existente:**
   - Si el nombre guardado está mal escrito, queda el del carné. Si sólo le
     falta un apellido al del carné, queda el guardado.
   - La fecha de vencimiento sólo se corre hacia adelante, y nunca a una ya
     vencida.
   - Cambia la empresa si la del carné es otra.
   - Carga el carné, o reemplaza el anterior si es distinto.
6. **Contratista nuevo:** se registra con tipo PRAIND y acceso, y se le carga
   el carné.
7. **Carné vencido:** la persona queda registrada con su foto y con
   vencimiento al **31 de diciembre** del año en curso. Así se vence igual,
   pero queda el dato. Se cambia con `--vencidos-hasta AAAA-MM-DD`. A uno
   que ya está, esa fecha sólo le reemplaza una vacía o ya vencida.

La imagen se sube **tal cual**, sin recomprimir. El WebP se convierte a PNG,
que no pierde calidad, porque el Worker sólo acepta PNG o JPEG.

Todo pasa por los mismos caminos que el panel, con la sesión del
administrador: `panel_crear_empresa`, las Edge Functions
`admin-crear-contratista` y `admin-editar-contratista`, y el Worker
`praind-brisas`. Nada salta la RLS ni las reglas.

## Cómo se usa

Desde la raíz del repo, en Windows:

```powershell
# 1. Simulación: arma el informe y no cambia nada.
node scripts/importar-carnes-praind/importar.mjs --carpeta "C:\ruta\a\los\carnes"

# 2. Si el informe está bien, aplicarlo (--sin-ocr reusa la lectura anterior).
node scripts/importar-carnes-praind/importar.mjs --carpeta "C:\ruta\a\los\carnes" --sin-ocr --aplicar
```

Pide el correo de un administrador del panel y manda un código a ese correo.
Sin terminal interactiva se hace en dos pasos: primero `--correo <correo>
--pedir-codigo`, que manda el código, y después `--correo <correo> --codigo
<código>`. La sesión queda en `importacion-praind/.sesion.json`, así no hay
que pedir otro código en cada corrida, y se borra cuando la importación
termina sin errores.

Los informes quedan en una carpeta `importacion-praind`, al lado de la de los
carnés (se cambia con `--salida`):

| Archivo | Qué es |
| --- | --- |
| `informe-simulacion.md` | El informe de la simulación |
| `informe-aplicado.md` | El informe de lo que se aplicó |
| `resultado.json` | Cada persona, con el error si lo hubo |
| `ocr.json` | Lo que leyó el OCR |
| `correcciones.json` | Arreglos a mano. Se edita y se vuelve a correr |

```json
{
  "archivo.png": { "cedula": "304440412" },
  "otro.png": { "nombre": "Nombre Bien Escrito" },
  "no-es-carne.png": { "omitir": true, "motivo": "no es un carné" }
}
```

Para apuntar a staging, crear `web/.env.local` con los valores de staging
(ver `docs/recuperacion-sitio-staging.md`). Si no, va a producción.

Pruebas: `node --test scripts/importar-carnes-praind/importar.test.mjs`.
