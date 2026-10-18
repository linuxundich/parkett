# Icon

App-Icon für Parkett: Fischgrät-Parkett mit eingelassenem Terminal-Bereich
(GNOME-HIG, 128-px-Raster, Kasten 104 × 100 bei (12, 12), 4 px Vorderkante).

| Datei | Inhalt |
|---|---|
| `parkett.svg` | App-Icon, **erzeugt** von `gen_icon.py` – dort ändern |
| `parkett-symbolic.svg` | symbolisches Icon 16 px, von Hand |
| `gen_icon.py` | Generator; `--no-grain` lässt die Maserung weg |
| `vorschau.png` | 256 … 16 px plus symbolisch, hell und dunkel |
| `entwuerfe/` | verworfene Varianten A (Korbflecht) und B (Split) |

Neu erzeugen:

    python3 gen_icon.py parkett.svg

Sobald die App-ID feststeht, beide Dateien unter `data/icons/hicolor/` nach der
App-ID umbenennen.
