# Paketierungswege

Stand 2026-10-19. Mindestversionen laut `meson.build`: GTK ≥ 4.18, libadwaita ≥ 1.8
(AdwShortcutsDialog), VTE (gtk4) ≥ 0.80 (Termprops, Kontextmenü-Modell). Kennzeichnung:
**F** belegt (selbst gebaut/getestet) · **S** Schlussfolgerung · **V** Vorschlag.

## Übersicht

| Weg | Stand | Aufwand | Einschätzung |
|---|---|---|---|
| **Arch, eigenes Repo `linuxundich`** | **F** läuft: `services/pacman-repo/pkgbuilds/parkett`, Meson + `check()`, auf bender veröffentlicht (r14) | erledigt | Hauptweg für die eigenen Rechner |
| Arch, AUR | – | gering | **S** braucht eine öffentliche Quelle (Git-Remote); PKGBUILD ist fast fertig |
| **Fedora** (RPM/COPR) | **F** baut und läuft in Fedora 44 (GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1) aus dem Quellbaum mit Meson | mittel | **V** Spec-Datei + COPR, sobald es eine öffentliche Quelle gibt |
| Debian/Ubuntu (.deb) | ungeprüft | mittel | **S** Debian 13 und Ubuntu 24.04 haben zu alte libadwaita/VTE; neuere (Ubuntu 26.04, Debian forky, TUXEDO OS) vermutlich passend – in den vorhandenen VMs (`ubuntu26.04`, `tuxedo-os-forky`) prüfen. Ein Absenken auf libadwaita 1.7 ginge, wenn die Kürzel-Übersicht ohne AdwShortcutsDialog auskommt |
| **Flatpak** | – | **hoch** | siehe unten |
| AppImage, Snap | – | – | **V** nicht verfolgen: Terminal braucht Host-Zugriff und Systemintegration |

## Flatpak im Detail

**F** Parkett nutzt heute direkt den Host: Es startet die Shell per `vte_terminal_spawn_async`
und liest `/proc` (Vordergrundprozess für Nachfrage beim Schließen, Kontextfarbe root/SSH,
„Als Vorlage speichern“). In der Flatpak-Sandbox sähe es nur den eigenen PID-Namensraum.

**S** Nötig wären:
- Shell auf dem Host starten: `flatpak-spawn --host` bzw. eine Host-PTY über
  `org.freedesktop.Flatpak.Development` (Berechtigung `--talk-name=org.freedesktop.Flatpak`).
- Ein **Host-Agent** wie bei Ptyxis (`agent/`, D-Bus über `socketpair`, Methoden u. a.
  `CreatePty`, `Spawn`, `HasForegroundProcess`, `GetWorkingDirectory`), gegen eine alte glibc
  gelinkt, damit er auf jedem Host läuft. Ohne ihn fallen Nachfrage beim Schließen,
  Kontextfarbe und Vorlagen-Befehle weg.
- VTE ist nicht in der GNOME-Runtime und müsste als Modul mitgebaut werden.

**Flathub:** Nach eigener Festlegung (Erinnerung „Flathub ohne KI“) reichst du KI-unterstützt
entstandene Apps dort nicht ein; ein eigenes Flatpak-Repo (wie bei anderen Projekten in
`apps/*`) bliebe möglich.

**V** Flatpak erst angehen, wenn es einen Grund gibt (andere Distributionen ohne passende
Pakete); dann als eigener Schritt mit Host-Agent.

## Empfehlung

**Entscheidung 2026-10-19:** vorerst nur lokal, Tag `v0.1.0`; einziger Weg ist das Arch-Repo
auf bender. Die übrigen Wege bleiben Vorschlag für später.

1. Arch-Repo bleibt der Hauptweg (läuft).
2. Für eine Veröffentlichung zuerst ein öffentliches Git-Repo; daraus AUR und Fedora COPR.
3. .deb nur bei Bedarf, nach Prüfung in den Ubuntu-/TUXEDO-VMs.
4. Flatpak nur mit Host-Agent, als eigenes Vorhaben.
