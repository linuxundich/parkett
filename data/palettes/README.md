# Farbschemata

Mitgelieferte Farbschemata im Paletten-Format von Ptyxis (Abschnitte `[Palette]`, `[Light]`,
`[Dark]`; optional `SuperuserBackground`, `RemoteBackground` u. a.). Jede Datei enthält eine
helle und eine dunkle Fassung; Parkett wählt sie nach dem Systemstil.

Herkunft: [Ptyxis](https://gitlab.gnome.org/chergert/ptyxis) `src/palettes/`, Stand Commit
`ecc97d39` (2026-10-11), GPL-3.0-or-later. Viele davon stammen ursprünglich aus
[Gogh](https://github.com/Gogh-Co/Gogh) (MIT, Hinweis am Dateiende). Dateinamen wurden
vereinheitlicht, drei Anzeigenamen eingedeutscht bzw. gekürzt (Hoher Kontrast, Catppuccin,
GitHub); die Farben sind unverändert.

| Datei | Ptyxis-Datei |
|---|---|
| gnome.palette | gnome.palette |
| gnome-high-contrast.palette | gnome-high-contrast.palette |
| tango.palette | tango.palette |
| solarized.palette | solarized.palette |
| nord.palette | nord.palette |
| dracula.palette | dracula.palette |
| catppuccin.palette | Catppuccin Mocha.palette |
| gruvbox.palette | Gruvbox.palette |
| one.palette | One.palette |
| everforest.palette | Everforest.palette |
| github.palette | Github.palette |
| selenized.palette | Selenized.palette |
| papercolor.palette | Papercolor.palette |
| vs-code.palette | Vs Code.palette |

Eigene oder importierte Schemata legt Parkett unter `$XDG_DATA_HOME/parkett/palettes/` ab.
