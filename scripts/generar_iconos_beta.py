"""Genera los iconos de las builds de prueba (debug y telemetria).

Toma los iconos de la app real y les pone una cinta diagonal naranja con la
palabra TEST en la esquina superior derecha, para que en el escritorio o en el
telefono se distinga a simple vista de la version de produccion.

Uso (desde la raiz del repo, requiere `pip install pillow`):

    python scripts/generar_iconos_beta.py

Escribe:
  - desktop/src-tauri/icons/diagnostico/   (32x32, 128x128, 128x128@2x, icon.png,
    icon.ico, icon.icns) -- los usa tauri.diagnostico.conf.json
  - mobile/android/app/src/beta/res/mipmap-*/ic_launcher(.png|_round.png) --
    los usan los tipos de build `debug` y `diagnostico` (build.gradle.kts)

Los originales no se tocan. El resultado es determinista: volver a correrlo
no cambia nada si el logo original no cambio.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

RAIZ = Path(__file__).resolve().parent.parent
ICONOS_ESCRITORIO = RAIZ / "desktop" / "src-tauri" / "icons"
RES_ANDROID = RAIZ / "mobile" / "android" / "app" / "src" / "main" / "res"
SALIDA_ESCRITORIO = ICONOS_ESCRITORIO / "diagnostico"
SALIDA_ANDROID = RAIZ / "mobile" / "android" / "app" / "src" / "beta" / "res"

NARANJA = (245, 124, 0, 255)
BORDE = (255, 255, 255, 255)
TEXTO = "TEST"
FUENTE = "C:/Windows/Fonts/arialbd.ttf"


def con_cinta(original: Image.Image) -> Image.Image:
    """El mismo icono con la cinta puesta; conserva la forma (alfa) original."""
    lado = original.width
    base = original.convert("RGBA")

    # Se dibuja a 4x y se reduce: bordes suaves sin depender del antialias.
    factor = 4
    grande = lado * factor
    capa = Image.new("RGBA", (grande, grande), (0, 0, 0, 0))
    dibujo = ImageDraw.Draw(capa)

    # Cinta: franja entre dos rectas paralelas a la diagonal secundaria.
    # Cubre la esquina superior derecha: x + (grande - y) entre dos valores.
    ancho = grande * 0.17
    centro = grande * 0.44
    # Puntos de la franja (esquina superior derecha, a 45 grados).
    a = centro - ancho / 2
    b = centro + ancho / 2
    franja = [
        (grande - a, 0),
        (grande - b, 0),
        (grande, b),
        (grande, a),
    ]
    dibujo.polygon(franja, fill=NARANJA)
    dibujo.line([franja[0], franja[3]], fill=BORDE, width=max(2, grande // 170))
    dibujo.line([franja[1], franja[2]], fill=BORDE, width=max(2, grande // 170))

    # El texto va en una capa propia, se rota 45 grados y se centra en la franja.
    fuente = ImageFont.truetype(FUENTE, int(ancho * 0.62))
    caja = dibujo.textbbox((0, 0), TEXTO, font=fuente)
    texto_w, texto_h = caja[2] - caja[0], caja[3] - caja[1]
    capa_texto = Image.new("RGBA", (texto_w + 20, texto_h + 20), (0, 0, 0, 0))
    ImageDraw.Draw(capa_texto).text(
        (10 - caja[0], 10 - caja[1]), TEXTO, font=fuente, fill=(255, 255, 255, 255)
    )
    capa_texto = capa_texto.rotate(-45, expand=True, resample=Image.BICUBIC)
    # Centro de la franja: punto medio de su recta central.
    cx = grande - centro / 2
    cy = centro / 2
    capa.alpha_composite(
        capa_texto,
        (int(cx - capa_texto.width / 2), int(cy - capa_texto.height / 2)),
    )

    capa = capa.resize((lado, lado), Image.LANCZOS)

    # La cinta no puede salirse de la forma del icono (esquinas redondeadas o
    # circulo): se recorta con el alfa original.
    mascara = base.getchannel("A")
    recorte = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    recorte.paste(capa, (0, 0), mascara)

    resultado = base.copy()
    resultado.alpha_composite(recorte)
    return resultado


def escritorio() -> None:
    SALIDA_ESCRITORIO.mkdir(parents=True, exist_ok=True)
    original = Image.open(ICONOS_ESCRITORIO / "icon.png").convert("RGBA")
    marcado = con_cinta(original)
    marcado.save(SALIDA_ESCRITORIO / "icon.png")

    for nombre, lado in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256)):
        marcado.resize((lado, lado), Image.LANCZOS).save(SALIDA_ESCRITORIO / nombre)

    marcado.save(
        SALIDA_ESCRITORIO / "icon.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    marcado.save(SALIDA_ESCRITORIO / "icon.icns")


def android() -> None:
    for carpeta in sorted(RES_ANDROID.glob("mipmap-*dpi")):
        destino = SALIDA_ANDROID / carpeta.name
        destino.mkdir(parents=True, exist_ok=True)
        for nombre in ("ic_launcher.png", "ic_launcher_round.png"):
            origen = carpeta / nombre
            if origen.exists():
                con_cinta(Image.open(origen)).save(destino / nombre)


if __name__ == "__main__":
    escritorio()
    android()
    print("iconos de prueba generados")
