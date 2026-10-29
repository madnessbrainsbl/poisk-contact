"""Render source-informed product illustrations; never reads application data.

Requires Python 3.10+, CairoSVG and Pillow. The optional GIFBuilder dependency
is supplied with --gif-skill; it is used only for building the animation.
"""
from __future__ import annotations

import argparse
import io
import re
import sys
from html import escape
from pathlib import Path

import cairosvg
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs" / "images"
SOURCE = (ROOT / "src" / "main.rs").read_text(encoding="utf-8")


def palette(name: str) -> str:
    match = re.search(rf"const {name}: Color32 = Color32::from_rgb\((\d+), (\d+), (\d+)\)", SOURCE)
    if not match:
        raise ValueError(f"UI palette constant missing: {name}")
    return "#" + "".join(f"{int(v):02x}" for v in match.groups())


BG, CARD, SIDE, BORDER, TEXT, MUTED, GREEN = (
    palette(n) for n in ("BG_COLOR", "CARD_BG", "SIDEBAR_BG", "BORDER_COLOR", "TEXT_COLOR", "TEXT_MUTED", "ACCENT_TEAL")
)


def rect(x, y, w, h, fill=CARD, stroke=BORDER, r=10):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}"/>'


def text(x, y, value, size=18, fill=TEXT, weight=400):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{fill}" font-weight="{weight}">{escape(str(value))}</text>'


def button(x, y, w, label, primary=False):
    return rect(x, y, w, 38, TEXT if primary else CARD) + text(x+13, y+25, label, 15, BG if primary else TEXT, 600)


def line(x1, y1, x2, y2, color=BORDER):
    return f'<path d="M{x1} {y1}H{x2}" stroke="{color}"/>' if y1 == y2 else f'<path d="M{x1} {y1}L{x2} {y2}" stroke="{color}"/>'


def shell(scene):
    labels = ["Find contacts", "Contacts", "Email template", "Mail and sending", "Activity log"]
    active = {"search": 0, "contacts": 1, "template": 2, "export": 1}[scene]
    title = labels[active]
    parts = [rect(62, 188, 1476, 718, BG, BORDER, 16), rect(62, 188, 214, 718, SIDE, BORDER, 16)]
    parts += [text(88, 232, "PoiskContact", 23, TEXT, 700), text(88, 274, "WORKSPACE", 12, MUTED, 600)]
    for i, label in enumerate(labels):
        y = 294+i*48
        if i == active:
            parts += [rect(78, y, 182, 40, "#222226", "#38383d", 8), line(79, y+10, 79, y+30, GREEN)]
        parts += [text(93, y+26, label, 16, TEXT if i == active else MUTED, 600 if i == active else 400)]
    parts += [rect(82, 592, 174, 124), text(98, 622, "SENDER", 12, MUTED, 600), text(98, 656, "Configure your sender", 14), text(98, 686, "No email configured", 12, MUTED)]
    parts += [rect(82, 738, 174, 138), text(98, 770, "LOCAL WORKSPACE", 12, MUTED, 600), text(98, 807, "Review before use", 14), text(98, 846, "Illustrative demo", 13, GREEN)]
    parts += [text(310, 236, title, 28, TEXT, 600)]
    return parts


def search():
    p = shell("search")
    p += [text(310, 271, "Discover businesses and save their published contact details.", 17, MUTED), button(310, 295, 152, "Start collection", True), button(476, 295, 145, "Open contacts")]
    p += [rect(310, 360, 575, 510), text(334, 401, "Search criteria", 20, TEXT, 600), text(334, 446, "Target location", 15, MUTED)]
    p += [rect(334, 460, 527, 43, BG), text(350, 488, "Berlin", 17), text(334, 536, "Collection mode", 15, MUTED), text(334, 570, "Business websites", 17), text(334, 613, "Search across industries", 17), text(334, 657, ">  Quick presets", 17), text(334, 710, "Business or service", 15, MUTED), rect(334, 726, 527, 43, BG), text(350, 754, "Photography", 17), text(334, 813, "Companies per run", 15, MUTED), button(603, 787, 80, "100")]
    p += [rect(909, 360, 595, 510), text(935, 402, "Collection pipeline", 20, TEXT, 600)]
    steps = [("Find businesses", "OpenStreetMap and web search."), ("Collect contacts", "Public business contact details."), ("Merge duplicates", "Match domains, emails and phones."), ("Check email domains", "Optional MX lookup.")]
    for i, (label, desc) in enumerate(steps):
        y = 449+i*85
        p += [text(935, y, f"0{i+1}", 14, GREEN, 600), text(974, y, label, 19, TEXT, 500), text(974, y+28, desc, 16, MUTED)]
    p += [line(935, 793, 1477, 793), text(935, 832, "Progress is saved automatically to your contacts.", 15, MUTED)]
    return p


def contacts(export=False):
    p = shell("export" if export else "contacts")
    p += [text(310, 271, "Review company records, keep notes and inspect source evidence.", 17, MUTED), button(310, 295, 156, "File and export"), button(480, 295, 152, "Select contacts"), button(646, 295, 156, "Database tools")]
    p += [rect(310, 358, 1194, 68), text(333, 399, "Search company, email or name...", 16, MUTED), button(1080, 373, 140, "Priority: All"), button(1234, 373, 142, "Status: All")]
    p += [text(334, 472, "COMPANY", 13, MUTED, 600), text(766, 472, "EMAIL", 13, MUTED, 600), text(1210, 472, "CITY", 13, MUTED, 600)]
    rows = [("Northstar Studio", "hello@northstar.example.invalid", "Berlin"), ("Harbor Labs", "team@harbor.example.invalid", "Berlin"), ("Evergreen Works", "contact@evergreen.example.invalid", "Munich")]
    for i, (company, email, city) in enumerate(rows):
        y = 496+i*87
        p += [rect(310, y, 1194, 78, CARD), text(335, y+33, company+" (Demo)", 20, TEXT, 600), text(335, y+58, "Fictional company record", 14, MUTED), text(766, y+44, email, 17), text(1210, y+44, city, 17)]
    p += [text(334, 812, "All example domains are reserved .invalid addresses.", 16, MUTED), text(334, 846, "No collection, verification or email sending is simulated here.", 16, MUTED)]
    if export:
        p += [rect(310, 341, 325, 337, "#202024", "#606068", 10), text(333, 379, "Export reviewed records", 19, TEXT, 600)]
        for i, label in enumerate(["CSV", "vCard", "AmoCRM / Bitrix24", "Instantly / Lemlist"]):
            p += [rect(326, 397+i*61, 293, 47, CARD), text(341, 427+i*61, label, 17)]
    return p


def template():
    p = shell("template")
    p += [text(310, 271, "Write once, personalize for each contact. Preview before sending.", 17, MUTED), button(310, 295, 150, "Save template", True), button(474, 295, 194, "Load saved template"), button(682, 295, 178, "Built-in templates")]
    p += [rect(310, 360, 575, 510), text(335, 401, "Subject", 18, TEXT, 600), rect(334, 424, 527, 46, BG), text(350, 454, "Question for {company}", 18), text(335, 516, "Message", 18, TEXT, 600), rect(334, 537, 527, 290, BG)]
    for i, value in enumerate(["Hi {name},", "", "I would like to discuss a possible", "collaboration with {company}.", "Would a short introduction be useful?", "", "Best regards,", "{sender_name}"]):
        p += [text(350, 575+i*29, value, 17)]
    p += [rect(909, 360, 595, 510), text(935, 401, "Email preview", 20, TEXT, 600), text(935, 444, "Example for Northstar Studio (Demo)", 17, MUTED), button(935, 469, 165, "Refresh preview"), button(1115, 469, 164, "Shuffle variations")]
    for i, value in enumerate(["Subject: Question for Northstar Studio", "", "Hi Alex Example,", "", "I would like to discuss a possible", "collaboration with Northstar Studio.", "Would a short introduction be useful?"]):
        p += [text(935, 570+i*32, value, 17)]
    return p


SCENES = {"search": search, "contacts": contacts, "template": template, "export": lambda: contacts(True)}
CAPTIONS = {"search": "Find public business contacts.", "contacts": "Review the evidence. Keep the context.", "template": "Personalize with a preview.", "export": "Export the records you choose."}


def svg(scene, progress=None):
    p = [rect(0, 0, 1600, 1000, "#101014", "none", 0), text(64, 55, "POISKCONTACT PRO  /  WINDOWS", 17, GREEN, 700), text(62, 117, CAPTIONS[scene], 44, TEXT, 600), text(64, 154, "Native desktop research. Local contact management. Built with Rust and egui.", 19, MUTED)]
    p += SCENES[scene]()
    p += [text(64, 949, "SOURCE-INFORMED ILLUSTRATION  ·  FICTIONAL DEMO DATA", 14, MUTED, 600), text(1110, 949, "github.com/madnessbrainsbl/poisk-contact", 14, MUTED)]
    if progress is not None:
        p += [rect(64, 976, 1472*progress, 4, GREEN, "none", 2)]
    return '<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1000" viewBox="0 0 1600 1000" role="img"><title>PoiskContact product illustration</title><desc>Source-informed illustration with fictional data; not an application screenshot.</desc><g font-family="Segoe UI, DejaVu Sans, sans-serif">'+''.join(p)+'</g></svg>'


def render():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("--gif-skill", type=Path)
    opts = args.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    for scene in SCENES:
        data = svg(scene).encode()
        (OUT/f"poisk-contact-{scene}.svg").write_bytes(data)
        cairosvg.svg2png(bytestring=data, write_to=str(OUT/f"poisk-contact-{scene}.png"), output_width=2400, output_height=1500)
    cover = '<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="640"><rect width="1280" height="640" fill="#101014"/><g font-family="Segoe UI, DejaVu Sans, sans-serif">'+text(70, 96, "POISKCONTACT PRO", 22, GREEN, 700)+text(67, 203, "Business contacts.", 68, TEXT, 700)+text(67, 285, "Research to review.", 68, TEXT, 700)+text(70, 359, "Find public contacts. Inspect sources. Export locally.", 26, MUTED)+rect(70, 417, 300, 54, TEXT, "none", 10)+text(90, 452, "Download for Windows", 22, BG, 600)+text(70, 556, "Rust + egui  /  Windows x64  /  Local workspace", 21, MUTED)+text(70, 600, "github.com/madnessbrainsbl/poisk-contact", 20, GREEN)+'</g></svg>'
    (OUT/"poisk-contact-social.svg").write_text(cover, encoding="utf-8")
    cairosvg.svg2png(bytestring=cover.encode(), write_to=str(OUT/"poisk-contact-social.png"))
    if opts.gif_skill:
        sys.path.insert(0, str(opts.gif_skill))
        from core.gif_builder import GIFBuilder
        from core.validators import validate_gif
        builder = GIFBuilder(width=960, height=600, fps=10)
        scenes = list(SCENES)
        for index, scene in enumerate(scenes):
            for frame in range(25):
                data = cairosvg.svg2png(bytestring=svg(scene, (index*25+frame+1)/100).encode(), output_width=960, output_height=600)
                builder.add_frame(Image.open(io.BytesIO(data)))
        builder.save(OUT/"poisk-contact-workflow.gif", num_colors=128, remove_duplicates=False)
        valid, details = validate_gif(OUT/"poisk-contact-workflow.gif", is_emoji=False, verbose=False)
        if not valid:
            raise ValueError(f"GIF validation failed: {details}")
        print(f"Validated GIF: {details['width']}x{details['height']}")
    print("Rendered source-informed illustrations and social cover.")


if __name__ == "__main__":
    render()
