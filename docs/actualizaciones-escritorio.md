# Actualizaciones de la app de escritorio

El updater de Tauri busca la versión nueva en una lista de direcciones
(`plugins.updater.endpoints` de `desktop/src-tauri/tauri.conf.json`) y usa la
primera que responde:

1. `https://lt.megabrisas.com/escritorio/latest.json` (Cloudflare R2).
2. El `latest.json` del último Release de GitHub, como respaldo mientras el
   repositorio siga público. Con el repo privado esa segunda dirección deja de
   servir a equipos sin sesión y se puede quitar.

## Cómo llega la versión a R2

El job `build-gui` de `release.yml` (push de un tag `v*`):

1. `tauri-action` compila, firma y publica el Release de GitHub con los
   instaladores, sus `.sig` y un `latest.json` que apunta a esos assets.
2. El paso **Publicar el escritorio en el almacenamiento de descargas** baja
   esos archivos del Release, sube cada instalador a
   `escritorio/<nombre>-<12 primeros del SHA-256>.<ext>` (inmutable) y reescribe
   cada dirección del `latest.json` encontrando el instalador por su firma.
   El manifiesto se sube al final a `escritorio/latest.json`, sin caché.

Las firmas de Tauri no cambian: la app sigue verificando cada instalador contra
`plugins.updater.pubkey`, venga de GitHub o de R2.

Reglas del paso:

- Un disparo manual (`workflow_dispatch`) deja el Release en borrador y **no**
  publica en R2.
- Sin los secretos `R2_*` o la variable `R2_BUCKET` se salta con un aviso. Si
  falla, el release sigue (`continue-on-error`): el equipo todavía tiene GitHub
  como segundo `endpoints`. El error queda a la vista en el log.

## Primera vez

Las versiones anteriores solo conocen el `endpoints` de GitHub. La primera
versión con R2 como primer `endpoints` se instala a mano en cada equipo; de ahí
en adelante se actualizan solas desde R2.

## Comprobar una publicación

```sh
curl -s https://lt.megabrisas.com/escritorio/latest.json
```

Tiene que mostrar la `version` recién publicada y, en cada plataforma, una `url`
que empiece por `https://lt.megabrisas.com/escritorio/`.
