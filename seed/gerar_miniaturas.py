#!/usr/bin/env python3
"""
Gera miniaturas 64×64 a partir de emoji (Twemoji CC-BY 4.0).
Fallback: inicial colorida por secção (como antes).
"""
from __future__ import annotations

import json
import urllib.request
from io import BytesIO
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "seed" / "catalogo_base.json"
OUT = ROOT / "data" / "imgs"
CACHE = ROOT / "seed" / "emoji_cache"

# fundo suave por secção
FUNDOS = {
    0: (232, 232, 232),
    1: (220, 236, 214),
    2: (245, 220, 220),
    3: (214, 230, 245),
    4: (250, 242, 210),
    5: (242, 230, 210),
    6: (218, 234, 248),
    7: (220, 232, 238),
    8: (234, 224, 242),
    9: (220, 232, 245),
    10: (228, 228, 220),
    11: (252, 232, 210),
}

# nome exacto do seed → emoji
EMOJI: dict[str, str] = {
    # Fruta
    "Banana": "🍌",
    "Maçã": "🍎",
    "Laranja": "🍊",
    "Morango": "🍓",
    "Uva": "🍇",
    "Pêra": "🍐",
    "Limão": "🍋",
    # Frescos / legumes
    "Tomate": "🍅",
    "Alface": "🥬",
    "Cenoura": "🥕",
    "Batata": "🥔",
    "Cebola": "🧅",
    "Alho": "🧄",
    "Pepino": "🥒",
    "Pimento": "🫑",
    "Couve": "🥦",
    "Espinafres": "🥬",
    "Abóbora": "🎃",
    "Brócolos": "🥦",
    "Courgete": "🥒",
    # Talho
    "Frango": "🍗",
    "Bifes de vaca": "🥩",
    "Carne picada": "🥩",
    "Costeleta porco": "🥩",
    "Fiambre": "🍖",
    "Chouriço": "🌭",
    "Bacon": "🥓",
    "Salsichas": "🌭",
    "Peru": "🦃",
    "Entrecosto": "🍖",
    # Peixaria
    "Pescada": "🐟",
    "Salmão": "🐟",
    "Atum fresco": "🐟",
    "Bacalhau": "🐟",
    "Camarão": "🦐",
    "Polvo": "🐙",
    "Dourada": "🐠",
    "Sardinhas": "🐟",
    # Laticínios
    "Leite": "🥛",
    "Iogurte natural": "🥛",
    "Queijo fresco": "🧀",
    "Queijo flamengo": "🧀",
    "Manteiga": "🧈",
    "Natas": "🥛",
    "Ovos": "🥚",
    "Queijo ralado": "🧀",
    "Bebida de soja": "🧃",
    # Mercearia
    "Pão de forma": "🍞",
    "Arroz": "🍚",
    "Massa esparguete": "🍝",
    "Azeite": "🫒",
    "Óleo alimentar": "🫙",
    "Açúcar": "🧂",
    "Farinha": "🌾",
    "Sal": "🧂",
    "Café moído": "☕",
    "Chá": "🍵",
    "Cereais": "🥣",
    "Bolachas": "🍪",
    "Atum em lata": "🐟",
    "Feijão": "🫘",
    "Grão de bico": "🫘",
    "Milho em lata": "🌽",
    "Molho de tomate": "🍅",
    "Vinagre": "🫙",
    "Mel": "🍯",
    "Compota": "🍓",
    "Chocolate": "🍫",
    "Amêndoas": "🥜",
    # Bebidas
    "Água 1,5L": "💧",
    "Sumo de laranja": "🧃",
    "Refrigerante": "🥤",
    "Cerveja": "🍺",
    "Vinho tinto": "🍷",
    "Vinho branco": "🥂",
    # Limpeza
    "Detergente loiça": "🍽️",
    "Detergente roupa": "👕",
    "Amaciador": "🫧",
    "Lixívia": "🧴",
    "Limpa-vidros": "🪟",
    "Sacos do lixo": "🗑️",
    "Esponjas": "🧽",
    # Higiene
    "Pasta de dentes": "🪥",
    "Champô": "🧴",
    "Gel de banho": "🛁",
    "Sabonete": "🧼",
    "Papel higiénico": "🧻",
    "Guardanapos": "🧻",
    "Desodorizante": "💨",
    # Congelados
    "Pizza congelada": "🍕",
    "Peixe panado": "🐟",
    "Legumes congel.": "❄️",
    "Gelado": "🍦",
    "Batata frita cong.": "🍟",
    # Outros
    "Ração para cão": "🐕",
    "Ração para gato": "🐈",
    "Pilhas": "🔋",
}

TWEMOJI = "https://cdn.jsdelivr.net/gh/jdecked/twemoji@15.1.0/assets/72x72/{code}.png"


def emoji_codepoint(emoji: str) -> str:
    """Twemoji file name:codepoints sem FE0F."""
    parts = []
    for ch in emoji:
        cp = ord(ch)
        if cp == 0xFE0F:  # variation selector
            continue
        parts.append(f"{cp:x}")
    return "-".join(parts)


def fetch_twemoji(emoji: str) -> Image.Image | None:
    code = emoji_codepoint(emoji)
    CACHE.mkdir(parents=True, exist_ok=True)
    cache_path = CACHE / f"{code}.png"
    if not cache_path.exists():
        url = TWEMOJI.format(code=code)
        try:
            with urllib.request.urlopen(url, timeout=20) as r:
                cache_path.write_bytes(r.read())
        except Exception as e:
            print(f"  ! twemoji {emoji} ({code}): {e}")
            return None
    try:
        return Image.open(cache_path).convert("RGBA")
    except Exception:
        return None


def letra(nome: str) -> str:
    for ch in nome:
        if ch.isalpha():
            return ch.upper()
    return "?"


def fallback(nome: str, secao: int) -> Image.Image:
    cor = {
        1: (76, 140, 74),
        2: (180, 72, 72),
        3: (56, 120, 168),
        4: (220, 190, 90),
        5: (180, 130, 70),
        6: (90, 150, 200),
        7: (90, 140, 160),
        8: (150, 110, 170),
        9: (100, 140, 190),
        10: (120, 120, 100),
        11: (210, 120, 50),
    }.get(secao, (120, 120, 120))
    img = Image.new("RGB", (64, 64), cor)
    d = ImageDraw.Draw(img)
    d.ellipse((6, 6, 58, 58), fill=tuple(min(255, c + 35) for c in cor))
    try:
        font = ImageFont.truetype(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 28
        )
    except OSError:
        font = ImageFont.load_default()
    L = letra(nome)
    bbox = d.textbbox((0, 0), L, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(((64 - tw) / 2, (64 - th) / 2 - 2), L, fill=(255, 255, 255), font=font)
    return img


def montar(emoji_img: Image.Image, secao: int) -> Image.Image:
    bg = FUNDOS.get(secao, (232, 232, 232))
    canvas = Image.new("RGBA", (64, 64), (*bg, 255))
    d = ImageDraw.Draw(canvas)
    d.rounded_rectangle((2, 2, 61, 61), radius=12, fill=(*bg, 255))
    icon = emoji_img.resize((48, 48), Image.Resampling.LANCZOS)
    canvas.alpha_composite(icon, (8, 8))
    return canvas.convert("RGB")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    itens = json.loads(SEED.read_text(encoding="utf-8"))
    ok = falhou = 0
    for it in itens:
        nome = it["nome"]
        secao = it.get("secao", 0)
        imag = it["imag"]
        emoji = EMOJI.get(nome)
        img = None
        if emoji:
            tw = fetch_twemoji(emoji)
            if tw is not None:
                img = montar(tw, secao)
                ok += 1
            else:
                falhou += 1
        if img is None:
            img = fallback(nome, secao)
            if emoji:
                pass
            else:
                falhou += 1
                print(f"  ? sem emoji: {nome}")
        img.save(OUT / imag, "PNG", optimize=True)
    print(f"{len(itens)} imagens → {OUT} (emoji OK={ok}, fallback/outros={falhou})")


if __name__ == "__main__":
    main()
