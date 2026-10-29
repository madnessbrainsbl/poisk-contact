# PoiskContact Pro media kit

PoiskContact Pro is a native Windows application for finding public business
contacts, reviewing source evidence, and managing a local contact database.
It is built with Rust and egui and includes optional email outreach tools.

## Visual assets

All interface visuals in this kit are **source-informed illustrations**, not
screenshots or recordings. Their palette and labels follow `src/main.rs`.
Layouts are composed for readability. Fictional records use reserved `.invalid`
domains. The animation demonstrates the workflow; it does not show a live search,
successful delivery, verification results, or measured processing speed.

| Asset | Format and size | Intended use |
| --- | --- | --- |
| [Search illustration](images/poisk-contact-search.svg) | SVG, scalable | README and articles |
| [Search image](images/poisk-contact-search.png) | PNG, 2400 x 1500 | High-resolution publication |
| [Contact review](images/poisk-contact-contacts.png) | PNG, 2400 x 1500 | Product overview |
| [Email template](images/poisk-contact-template.png) | PNG, 2400 x 1500 | Template workflow |
| [Export illustration](images/poisk-contact-export.png) | PNG, 2400 x 1500 | Export workflow |
| [Workflow animation](images/poisk-contact-workflow.gif) | GIF, 960 x 600 | README and short previews |
| [Workflow video](images/poisk-contact-workflow.mp4) | MP4, 960 x 600 | Social posts |
| [Social cover](images/poisk-contact-social.png) | PNG, 1280 x 640 | Link previews and announcements |
| [Editable social cover](images/poisk-contact-social.svg) | SVG, scalable | Design adaptation |

## Suggested announcement

PoiskContact Pro brings public business contact research to a native Windows
workspace. Search by location and industry, review website evidence, keep notes,
and export the records you choose. The latest update improves the search layout,
sidebar, and email template workspace.

[Download the Windows release](https://github.com/madnessbrainsbl/poisk-contact/releases/latest)
and [browse the source](https://github.com/madnessbrainsbl/poisk-contact).

Short version: **Public business contact research, source review, and local
exports in a native Windows app built with Rust and egui.**

## Rebuilding the illustrations

Run `python -X utf8 docs/promo/render.py` from an environment with CairoSVG and Pillow.
The script reads UI palette constants from the source and writes SVG and PNG
files. It never reads saved contacts, mail settings, profiles, or credentials.
For the GIF, pass `--gif-skill` with the directory of the installed
`slack-gif-creator` skill, which provides its GIFBuilder and validator.
FFmpeg can convert the generated GIF to MP4 for social sharing.

These assets do not change the source licensing terms or any dependency license.
