"""Icono navideño de Lattis: sombrero rojo sobre la L y nieve al pie.

Mismo diseño para escritorio y móvil. Se pone en el parche de la última
semana de noviembre y se quita en el de la primera de enero; las dos
versiones quedan guardadas en `assets/iconos/` para el año siguiente.

Uso (desde la raíz del repo, requiere `pip install pillow` y las
dependencias de `desktop/` instaladas para `tauri icon`):

    python scripts/icono_navideno.py poner     # copia la versión navideña
    python scripts/icono_navideno.py quitar    # vuelve a la normal
    python scripts/icono_navideno.py generar   # rehace assets/iconos/navidad
                                               # desde assets/iconos/normal

`poner` y `quitar` también regeneran los iconos de prueba (beta/diagnóstico)
con `generar_iconos_beta.py`, para que lleven el mismo dibujo con su cinta.
Si cambia el logo, actualizar `assets/iconos/normal/` y correr `generar`.
"""

import math
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

RAIZ = Path(__file__).resolve().parent.parent
ICONOS_ESCRITORIO = RAIZ / "desktop" / "src-tauri" / "icons"
RES_ANDROID = RAIZ / "mobile" / "android" / "app" / "src" / "main" / "res"
NORMAL = RAIZ / "assets" / "iconos" / "normal"
NAVIDAD = RAIZ / "assets" / "iconos" / "navidad"
LOGO = RAIZ / "assets" / "iconoL.png"
LOGO_NAVIDAD = RAIZ / "assets" / "iconoL-navidad.png"

ANDROID = ("ic_launcher.png", "ic_launcher_round.png")
# Lo que `tauri icon` genera y está en git (el resto, android/ e ios/, no se usa).
ESCRITORIO = (
    "32x32.png", "64x64.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico", "icon.icns",
    "Square30x30Logo.png", "Square44x44Logo.png", "Square71x71Logo.png", "Square89x89Logo.png",
    "Square107x107Logo.png", "Square142x142Logo.png", "Square150x150Logo.png",
    "Square284x284Logo.png", "Square310x310Logo.png", "StoreLogo.png",
)

ROJO = (206, 32, 52, 255)
ROJO_SOMBRA = (150, 20, 38, 255)
BLANCO = (255, 255, 255, 255)
CONTORNO = (178, 198, 222, 255)
NIEVE_SOMBRA = (212, 226, 243, 255)

# Medidas tomadas sobre assets/iconoL.png (1254 px): caja del logo (L + texto)
# y el borde de arriba de la L, que sube hacia la derecha.
CAJA_REFERENCIA = (219, 183, 1039, 1046)
CENTRO_ALA = (490, 214)
ANGULO_ALA = math.atan2(-41, 105)


def caja_logo(imagen: Image.Image) -> tuple[int, int, int, int]:
    """Caja de los píxeles oscuros del logo (la L y "Lattis")."""
    gris = imagen.convert("RGBA")
    ancho, alto = gris.size
    datos = gris.load()
    xs, ys = [], []
    for y in range(alto):
        for x in range(ancho):
            r, g, b, a = datos[x, y]
            if a > 200 and (r + g + b) / 3 < 110:
                xs.append(x)
                ys.append(y)
    return min(xs), min(ys), max(xs), max(ys)


def bezier(p0, p1, p2, p3, pasos=40):
    puntos = []
    for i in range(pasos + 1):
        t = i / pasos
        m = 1 - t
        puntos.append((
            m ** 3 * p0[0] + 3 * m * m * t * p1[0] + 3 * m * t * t * p2[0] + t ** 3 * p3[0],
            m ** 3 * p0[1] + 3 * m * m * t * p1[1] + 3 * m * t * t * p2[1] + t ** 3 * p3[1],
        ))
    return puntos


def navideno(original: Image.Image) -> Image.Image:
    """El mismo icono con sombrero y nieve; conserva su forma (alfa)."""
    base = original.convert("RGBA")
    lado = base.width
    x0, y0, x1, y1 = caja_logo(base)

    factor = 4
    grande = lado * factor
    capa = Image.new("RGBA", (grande, grande), (0, 0, 0, 0))
    dibujo = ImageDraw.Draw(capa)

    # De coordenadas de iconoL.png a las de esta imagen (a 4x).
    rx0, ry0, rx1, _ = CAJA_REFERENCIA
    escala = (x1 - x0) / (rx1 - rx0)

    def mundo(x, y):
        return ((x0 + (x - rx0) * escala) * factor, (y0 + (y - ry0) * escala) * factor)

    # Ejes del sombrero: `u` a lo largo del borde de la L, `v` hacia afuera.
    du = (math.cos(ANGULO_ALA), math.sin(ANGULO_ALA))
    dv = (math.sin(ANGULO_ALA), -math.cos(ANGULO_ALA))

    def local(u, v):
        cx, cy = CENTRO_ALA
        return mundo(cx + u * du[0] + v * dv[0], cy + u * du[1] + v * dv[1])

    def trazo(px):
        return max(1, round(px * escala * factor))

    # Cono: sube por la izquierda y cae doblado hacia la derecha.
    exterior = bezier((-100, 22), (-95, 175), (70, 222), (222, 52))
    interior = bezier((200, 40), (140, 110), (100, 75), (100, 22))
    dibujo.polygon([local(*p) for p in exterior + interior], fill=ROJO)
    # Pliegue: la cara de abajo de la punta que cae, más oscura.
    pliegue = bezier((60, 172), (150, 170), (205, 105), (222, 52)) + bezier((200, 40), (150, 112), (105, 140), (60, 172))
    dibujo.polygon([local(*p) for p in pliegue], fill=ROJO_SOMBRA)

    # Ala blanca de piel sobre el borde de la L.
    # Una cápsula: tramo recto y dos puntas redondeadas.
    radio_ala, recto, centro_v = 27, 95, 2
    ala = [(-recto + 2 * recto * i / 30, centro_v - radio_ala) for i in range(31)]
    ala += [(recto + radio_ala * math.sin(math.pi * i / 20), centro_v - radio_ala * math.cos(math.pi * i / 20)) for i in range(1, 20)]
    ala += [(recto - 2 * recto * i / 30, centro_v + radio_ala) for i in range(31)]
    ala += [(-recto - radio_ala * math.sin(math.pi * i / 20), centro_v + radio_ala * math.cos(math.pi * i / 20)) for i in range(1, 20)]
    dibujo.polygon([local(*p) for p in ala], fill=BLANCO, outline=CONTORNO, width=trazo(6))

    # Pompón.
    cx, cy = local(222, 44)
    r = 40 * escala * factor
    dibujo.ellipse((cx - r, cy - r, cx + r, cy + r), fill=BLANCO, outline=CONTORNO, width=trazo(6))

    # Nieve al pie: tapa apenas la base de las letras, como si el logo
    # estuviera medio enterrado, y se oscurece un poco hacia abajo.
    alto_letras = (1046 - 810) * escala
    tope = (y1 - alto_letras * 0.11) * factor

    def perfil(bajada: float) -> list[tuple[float, float]]:
        puntos = []
        for i in range(0, grande + 1, max(1, grande // 200)):
            t = i / grande
            y = tope + alto_letras * factor * (
                bajada + 0.07 * math.sin(t * 2 * math.pi * 1.6 + 0.6) + 0.035 * math.sin(t * 2 * math.pi * 3.7 + 1.9)
            )
            puntos.append((i, y))
        return puntos

    # Capas de abajo hacia arriba: cada una un poco más baja y más azul da el
    # volumen del montículo; el borde de arriba queda blanco con su contorno.
    borde = perfil(0)
    dibujo.polygon(borde + [(grande, grande), (0, grande)], fill=BLANCO)
    for paso in range(1, 7):
        t = paso / 6
        color = tuple(round(BLANCO[c] + (NIEVE_SOMBRA[c] - BLANCO[c]) * t) for c in range(3)) + (255,)
        capa_nieve = perfil(0.12 + 0.09 * paso)
        dibujo.polygon(capa_nieve + [(grande, grande), (0, grande)], fill=color)
    dibujo.line(borde, fill=CONTORNO, width=trazo(8), joint="curve")

    capa = capa.resize((lado, lado), Image.LANCZOS)
    mascara = base.getchannel("A")
    recorte = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    recorte.paste(capa, (0, 0), mascara)
    resultado = base.copy()
    resultado.alpha_composite(recorte)
    return resultado


def generar() -> None:
    (NAVIDAD / "escritorio").mkdir(parents=True, exist_ok=True)
    navideno(Image.open(NORMAL / "escritorio" / "icon.png")).save(NAVIDAD / "escritorio" / "icon.png")
    for carpeta in sorted((NORMAL / "android").glob("mipmap-*dpi")):
        destino = NAVIDAD / "android" / carpeta.name
        destino.mkdir(parents=True, exist_ok=True)
        for nombre in ANDROID:
            navideno(Image.open(carpeta / nombre)).save(destino / nombre)
    navideno(Image.open(LOGO).convert("RGBA")).convert("RGB").save(LOGO_NAVIDAD)
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
