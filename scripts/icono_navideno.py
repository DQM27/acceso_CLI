"""Icono navideño de Lattis: sombrero rojo sobre la L y nieve al pie.

Mismo diseño para escritorio y móvil. Se pone en el parche de la última
semana de noviembre y se quita en el de la primera de enero; las dos
versiones quedan guardadas en `assets/iconos/` para el año siguiente.

Uso (desde la raíz del repo, requiere `pip install pillow resvg-py` y las
dependencias de `desktop/` instaladas para `tauri icon`):

    python scripts/icono_navideno.py poner     # copia la versión navideña
    python scripts/icono_navideno.py quitar    # vuelve a la normal
    python scripts/icono_navideno.py generar   # rehace assets/iconos/navidad
                                               # desde assets/lattis-navidad.svg

`generar` dibuja cada icono desde el SVG (nítido en cualquier tamaño), con
la forma (esquinas redondeadas o círculo) y el encuadre del icono normal
del mismo tamaño. `poner` y `quitar` también regeneran los iconos de
prueba (beta/diagnóstico) con `generar_iconos_beta.py`, para que lleven el
mismo dibujo con su cinta.
"""

import io
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import resvg_py
from PIL import Image

RAIZ = Path(__file__).resolve().parent.parent
ICONOS_ESCRITORIO = RAIZ / "desktop" / "src-tauri" / "icons"
RES_ANDROID = RAIZ / "mobile" / "android" / "app" / "src" / "main" / "res"
NORMAL = RAIZ / "assets" / "iconos" / "normal"
NAVIDAD = RAIZ / "assets" / "iconos" / "navidad"
SVG_NAVIDAD = RAIZ / "assets" / "lattis-navidad.svg"
LOGO_NAVIDAD = RAIZ / "assets" / "iconoL-navidad.png"

ANDROID = ("ic_launcher.png", "ic_launcher_round.png")
# Lo que `tauri icon` genera y está en git (el resto, android/ e ios/, no se usa).
ESCRITORIO = (
    "32x32.png", "64x64.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico", "icon.icns",
    "Square30x30Logo.png", "Square44x44Logo.png", "Square71x71Logo.png", "Square89x89Logo.png",
    "Square107x107Logo.png", "Square142x142Logo.png", "Square150x150Logo.png",
    "Square284x284Logo.png", "Square310x310Logo.png", "StoreLogo.png",
)

# Lienzo del SVG y caja del logo (la L y "Lattis") dentro de él.
LIENZO_SVG = 1254
CAJA_SVG = (219, 183, 1039, 1046)


def caja_logo(imagen: Image.Image) -> tuple[int, int, int, int]:
    """Caja de los píxeles oscuros del logo en un icono normal."""
    datos = imagen.convert("RGBA").load()
    ancho, alto = imagen.size
    xs, ys = [], []
    for y in range(alto):
        for x in range(ancho):
            r, g, b, a = datos[x, y]
            if a > 200 and (r + g + b) / 3 < 110:
                xs.append(x)
                ys.append(y)
    return min(xs), min(ys), max(xs), max(ys)


def renderizar(lado: int) -> Image.Image:
    png = resvg_py.svg_to_bytes(svg_path=str(SVG_NAVIDAD), width=lado, height=lado)
    return Image.open(io.BytesIO(bytes(png))).convert("RGBA")


def desde_svg(normal: Image.Image) -> Image.Image:
    """El icono navideño con la forma y el encuadre de `normal`."""
    lado = normal.width
    x0, _, x1, _ = caja_logo(normal)
    escala = (x1 - x0) / (CAJA_SVG[2] - CAJA_SVG[0])
    # A 4x y reducido: los bordes del logo encajan al píxel en los chicos.
    factor = 4
    dibujo = renderizar(round(LIENZO_SVG * escala * factor))
    lienzo = Image.new("RGBA", (lado * factor, lado * factor), (255, 255, 255, 255))
    desplazamiento = round((x0 - CAJA_SVG[0] * escala) * factor)
    _, y0, _, _ = caja_logo(normal)
    desplazamiento_y = round((y0 - CAJA_SVG[1] * escala) * factor)
    lienzo.alpha_composite(dibujo, (desplazamiento, desplazamiento_y))
    resultado = lienzo.resize((lado, lado), Image.LANCZOS)
    resultado.putalpha(normal.convert("RGBA").getchannel("A"))
    return resultado


def generar() -> None:
    (NAVIDAD / "escritorio").mkdir(parents=True, exist_ok=True)
    desde_svg(Image.open(NORMAL / "escritorio" / "icon.png")).save(NAVIDAD / "escritorio" / "icon.png")
    for carpeta in sorted((NORMAL / "android").glob("mipmap-*dpi")):
        destino = NAVIDAD / "android" / carpeta.name
        destino.mkdir(parents=True, exist_ok=True)
        for nombre in ANDROID:
            desde_svg(Image.open(carpeta / nombre)).save(destino / nombre)
    grande = Image.new("RGBA", (LIENZO_SVG, LIENZO_SVG), (255, 255, 255, 255))
    grande.alpha_composite(renderizar(LIENZO_SVG))
    grande.convert("RGB").save(LOGO_NAVIDAD)
    print("assets/iconos/navidad generado")


def aplicar(origen: Path) -> None:
    for carpeta in sorted((origen / "android").glob("mipmap-*dpi")):
        for nombre in ANDROID:
            shutil.copyfile(carpeta / nombre, RES_ANDROID / carpeta.name / nombre)

    # `tauri icon` saca todos los tamaños del escritorio desde un solo PNG.
    with tempfile.TemporaryDirectory() as temporal:
        subprocess.run(
            ["npx", "--no-install", "tauri", "icon", str(origen / "escritorio" / "icon.png"), "-o", temporal],
            cwd=RAIZ / "desktop", check=True, shell=sys.platform == "win32",
        )
        for nombre in ESCRITORIO:
            shutil.copyfile(Path(temporal) / nombre, ICONOS_ESCRITORIO / nombre)
    # El icon.png de 512 va tal cual, sin pasar por el redimensionado.
    shutil.copyfile(origen / "escritorio" / "icon.png", ICONOS_ESCRITORIO / "icon.png")

    subprocess.run([sys.executable, str(RAIZ / "scripts" / "generar_iconos_beta.py")], check=True)
    print(f"iconos de {origen.name} aplicados")


if __name__ == "__main__":
    accion = sys.argv[1] if len(sys.argv) > 1 else ""
    if accion == "generar":
        generar()
    elif accion == "poner":
        aplicar(NAVIDAD)
    elif accion == "quitar":
        aplicar(NORMAL)
    else:
        sys.exit("uso: python scripts/icono_navideno.py generar|poner|quitar")
