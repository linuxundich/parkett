# Icon

App icon for Parkett: herringbone parquet with an inset terminal area
(GNOME HIG, 128 px grid, box 104 × 100 at (12, 12), 4 px front edge).

| File | Content |
|---|---|
| `parkett.svg` | app icon, **generated** by `gen_icon.py` – make changes there |
| `parkett-symbolic.svg` | symbolic icon 16 px, made by hand |
| `gen_icon.py` | generator; `--no-grain` leaves out the wood grain |
| `vorschau.png` | 256 … 16 px plus symbolic, light and dark |
| `entwuerfe/` | rejected variants A (basket weave) and B (split) |

Regenerate:

    python3 gen_icon.py parkett.svg

Once the app ID is settled, rename both files under `data/icons/hicolor/` after the
app ID.
