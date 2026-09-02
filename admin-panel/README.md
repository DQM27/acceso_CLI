# Panel de dispositivos

Herramienta de administración para el receptor en la nube
(`control-acceso-nube`, ver `docs/plan-persistencia-nube.md`). Da de alta,
lista y revoca las credenciales de los dispositivos (PC/celular) de cada
sitio.

Deliberadamente separada de las apps de escritorio y móvil — ningún
dispositivo de sitio tiene, ni debe tener, acceso a estas funciones.

## Uso

Abrí `panel-dispositivos.html` con doble clic (corre en el navegador,
sin instalar nada). Te va a pedir el código de administrador — está en
`supabase_admin_key.txt` que quedó en el escritorio al crearlo. Habla
directo con las Edge Functions del proyecto de Supabase por HTTPS.

Por ahora corre solo como archivo local: no hay hosting público. Si en
algún momento hace falta que otro administrador lo use desde otra PC,
ahí conviene subirlo a un hosting estático (Netlify/Vercel, gratis) en
vez de copiar el archivo a mano.
