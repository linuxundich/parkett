# Color schemes

Bundled color schemes in the Ptyxis palette format (sections `[Palette]`, `[Light]`,
`[Dark]`; optionally `SuperuserBackground`, `RemoteBackground` and others). Each file contains a
light and a dark variant; Parkett picks one according to the system style.

Origin: [Ptyxis](https://gitlab.gnome.org/chergert/ptyxis) `src/palettes/`, as of commit
`ecc97d39` (2026-10-11), GPL-3.0-or-later. Many of them originally come from
[Gogh](https://github.com/Gogh-Co/Gogh) (MIT, notice at the end of the file). File names were
normalized, three display names were translated into German or shortened (*Hoher Kontrast*
(High Contrast), Catppuccin, GitHub); the colors are unchanged, except in *Hoher Kontrast*:
there the ANSI colors 1–6 and 9–14 were darkened (light) or lightened (dark) toward black or
white until they reach 7:1 or 4.5:1 against the background (QA audit 2026-10-19, QA-10; rule in
the file header, checked by a unit test).

| File | Ptyxis file |
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

Parkett stores custom or imported schemes under `$XDG_DATA_HOME/parkett/palettes/`.
