# Phase 4 – Bedienbarer MVP

Freigegeben am 2026-10-18 mit den Architekturvorschlägen aus `PROTOTYP.md` (Unterklassen,
Blueprint für Dialoge, GSettings + Dateien, serde, Meson, GPL-3.0-or-later, Ptyxis-Palette prüfen).

## Schritte

| # | Schritt | Stand |
|---|---|---|
| 1 | **Tabs und Splits** (dieses Dokument, Abschnitt unten) | fertig, in VM getestet |
| 2 | Meson-Build, GSettings-Schema, LICENSE, Arch-Paket | fertig, in VM getestet |
| 3 | Profile und Farbschemata (Ptyxis-Format, Import, Kacheln mit Live-Vorschau) | fertig, in VM getestet |
| 4 | Einstellungen (AdwPreferencesDialog): Allgemein, Profile, Tastenkürzel | fertig |
| 5 | Suche im Terminal, Links öffnen (Strg+Klick), änderbare Tastenkürzel | fertig, in VM getestet |
| 6 | Befehlspalette, Layout-Modus, Vorlagen (A+) | offen, eventuell nach MVP |
| 7 | Rückgängig für Schließen, Kontextfarbe je Bereich (Vorschläge 1, 4) | offen |

## Schritt 1: Tabs und Splits

### Umgesetzt

- **Unterklassen** `ParkettWindow` (AdwApplicationWindow), `ParkettTab` und `ParkettPane`
  (AdwBin). Der Tab hält das Layout-Modell aus `parkett-core` und baut daraus den GtkPaned-Baum;
  Bereiche werden beim Umbau nur umgehängt, Terminals laufen weiter.
- **Tabs:** AdwTabView, AdwTabBar (bei einem Tab ausgeblendet), AdwTabOverview mit Suche
  (Titel und Verzeichnis), AdwTabButton mit Anzahl, Split-Knopf „Neuer Tab“ mit Menü.
  Neuer Tab und neue Bereiche übernehmen das Verzeichnis des fokussierten Bereichs (OSC 7).
  Tab umbenennen (F2, Menü), leerer Name = Titel des Terminals.
- **Splits:** rechts, unten, automatisch (längere Seite); Fokus nach Richtung; Bereich 1–9;
  Größe per Tastatur (3 % je Schritt) und Maus; angleichen; maximieren mit Hinweis
  „Maximiert · n Bereiche ausgeblendet · Alle zeigen“ und „(1/n)“ im Titel (Vorschlag 3).
  Mindestgröße 20×5 Zellen; Teilen darunter wird mit einer Einblendung abgelehnt.
- **Darstellung (A+):** schwebender Titel je Bereich nur bei geteilten Tabs, Fokuslinie in
  Akzentfarbe, inaktive Bereiche abgeblendet.
- **Kontextmenü** um Teilen, Maximieren, Angleichen, Schließen erweitert; Rechtsklick
  fokussiert vorher den Bereich.
- **Schließen mit Nachfrage**, wenn ein Vordergrundprozess läuft (tcgetpgrp wie Ptyxis) –
  für Bereich, Tab und Fenster. Letzter Bereich schließt den Tab, letzter Tab das Fenster.
- **Prozessende:** Leiste bleibt; Tab bekommt Warnsymbol und „braucht Aufmerksamkeit“, wenn
  ein Bereich mit Fehler endet und der Tab im Hintergrund ist.
- **Tastenkürzel-Übersicht** (AdwShortcutsDialog, Strg+?, Menü). libadwaita-Mindestversion
  dafür jetzt 1.8.

### Kürzel

| Aktion | Kürzel |
|---|---|
| Neuer Tab / Tab schließen | Strg+Umschalt+T / Strg+Umschalt+Q |
| Tab wechseln | Strg+Bild ab / Strg+Bild auf, Strg+Alt+1…9 |
| Tab umbenennen / Übersicht | F2 / Strg+Umschalt+O |
| Rechts / unten / automatisch teilen | Strg+Alt+R / D / A |
| Bereich schließen / maximieren / angleichen | Strg+Umschalt+W / Strg+Umschalt+X / Strg+Alt+= |
| Fokus / Größe | Alt+Pfeil / Umschalt+Alt+Pfeil, Bereich n: Alt+1…9 |

Fenster- und Terminalkürzel hängen als `GtkShortcutController` (Capture-Phase) an Fenster bzw.
Terminal, nicht als App-Accels (siehe PROTOTYP.md, gefundener Fehler).

### Tests in der VM fedora44-wor (GNOME 50.5, Wayland, 1920×1033)

| Test | Ergebnis |
|---|---|
| `cargo test` (13 Kern-Tests inkl. Verhältnis per Pfad, Zoom aufheben, 1 App-Test), `cargo clippy` | grün, keine Befunde |
| Rechts teilen, dann unten teilen: 50 % / 50 % | ok **nach Korrektur** (erst 35 %) – siehe Funde 1, 2 |
| Alt+Links fokussiert linken Bereich; Umschalt+Alt+Rechts ×3 → 59 % | ok (`mvp-splits.png`) |
| Strg+Umschalt+X: maximiert mit Hinweis und „(1/3)“, Eingabe kommt an; nochmals → zurück | ok (`mvp-maximiert.png`) |
| Strg+Alt+= gleicht an | ok |
| Fensterbreite 960 → maximiert 1920 → zurück: Verhältnis bleibt 40,9 % | ok (Protokoll `PARKETT_DEBUG_LAYOUT`) |
| Neuer Tab, Tab-Leiste erscheint, Strg+Bild auf wechselt | ok |
| F2 → „Server“ → Eingabe benennt den Tab um | ok **nach Korrektur** (Fund 3) |
| Tab-Übersicht mit Vorschau beider Tabs | ok (`mvp-uebersicht.png`) |
| `sleep 300`, Strg+Umschalt+W → Nachfrage nennt „sleep“; Bestätigen beendet Shell und sleep | ok **nach Korrektur** (Fund 4) (`mvp-nachfrage.png`) |
| Tab mit `sleep` schließen: Esc bricht ab, Bestätigen beendet alles | ok **nach Korrektur** (Fund 5) |
| Fenster schließen mit laufendem `sleep` → Nachfrage | ok |
| Rechtsklick im inaktiven Bereich: Fokus wechselt, Menü mit Kürzeln; „Unten teilen“ wirkt dort | ok |
| `exit` → Leiste, Strg+Umschalt+W schließt den beendeten Bereich ohne Nachfrage | ok |

### Gefunden und behoben

1. Trenner-Abgleich rechnete mit `max-position` statt Gesamtlänge (GTK zieht dort die
   Mindestgröße des zweiten Kindes ab) → linke Hälfte nur 35 %.
2. Zugeklappter GtkRevealer (Leiste „Prozess beendet“) forderte weiter ~300 px Breite an →
   Bereiche konnten nicht schmaler werden. Jetzt unsichtbar, solange zugeklappt.
3. AdwAlertDialog fokussiert die Standard-Antwort; das Eingabefeld zum Umbenennen bekam
   keine Tastatureingaben → Feld wird fokussiert.
4. **Prozesse liefen nach dem Schließen weiter:** Closures hielten das VTE-Widget stark
   (u. a. am globalen StyleManager), es wurde nie freigegeben. Jetzt schwache Referenzen,
   Abmelden beim Zerstören, zusätzlich ausdrückliches SIGHUP an die Shell.
5. AdwAlertDialog sendet `closed` vor `response` → Bestätigung kam zu spät an, der Tab blieb
   offen. Jetzt wird nur `response` ausgewertet.

### Offen / beobachtet

- Cursor war nach Schließen der Tab-Übersicht bis zur nächsten Eingabe unsichtbar (vier
  Aufnahmen im Abstand von 0,35 s). Ursache noch unklar (VTE-Blinklogik nach Fokuswechsel?).
- Auswahlfarbe im Dunkelmodus ist VTE-Standard.
- **Begriff:** entschieden am 2026-10-18 – durchgehend „Reiter“ wie libadwaita (Oberfläche,
  Kürzel-Übersicht, Schema-Texte). Code und ältere Doku sagen weiter „Tab“.
- Nicht getestet: Ziehen von Tabs in ein neues Fenster, Maus-Ziehen der Trenner (nur über
  Tastatur und Fenstergröße geprüft), Host-Start, Orca.
- Drag&Drop von Bereichen (P1) und Rückgängig (Vorschlag 4) kommen später.

## Schritt 2: Meson, GSettings, LICENSE, Arch-Paket

### Umgesetzt

- **Begriff „Reiter“** in allen Texten der Oberfläche (Entscheidung Christoph).
- **Meson** wie bei Kulisse: `meson.build` ruft `build-aux/cargo-build.sh` (setzt
  `PARKETT_INSTALLED`), installiert Programm, `.desktop` (Exec mit vollem Pfad aus `bindir`),
  Metainfo, GSettings-Schema und Icons; `gnome.post_install` kompiliert Schema und Caches.
  `meson test`: desktop-file-validate, appstreamcli, glib-compile-schemas --strict, cargo test.
- **`cargo run`** bleibt möglich: `app/build.rs` kompiliert das Schema nach `$OUT_DIR/schemas`,
  `settings::init()` setzt dann `GSETTINGS_SCHEMA_DIR` (nur ohne `PARKETT_INSTALLED`).
- **GSettings** `de.linuxundich.Parkett` mit Schlüsseln, die schon wirken:
  `window-width/-height/-maximized`, `confirm-close`, `dim-unfocused` (live),
  `autohide-tab-bar` (live, per Bindung), `new-terminal-directory` (current/home),
  `scrollback-lines` (live, -1 = unbegrenzt). Profile und Vorlagen kommen als Dateien.
- **LICENSE** GPL-3.0 (Volltext).
- **VM-Skript** `build-aux/vm/deploy.sh` baut und installiert jetzt mit Meson nach `~/.local`.
- **Arch-Paket** `parkett` im pacman-Repo (`services/pacman-repo/pkgbuilds/parkett`).

### Tests (VM fedora44-wor)

| Test | Ergebnis |
|---|---|
| `meson setup/compile/test/install` lokal (Prefix `_install`) | 4/4 Tests ok, alle Dateien an der richtigen Stelle |
| `meson install` nach `~/.local` in der VM, Start | ok |
| `gsettings list-recursively de.linuxundich.Parkett` in der VM | alle 8 Schlüssel mit Standardwerten |
| `dim-unfocused false` / `autohide-tab-bar false` zur Laufzeit | wirken sofort (keine Abblendung, Reiterleiste bei einem Reiter sichtbar) |
| Fenster maximieren, letzten Reiter schließen → `window-maximized true`, Neustart maximiert | ok **nach Korrektur**: beim Schließen über den letzten Reiter wurde zuerst nicht gespeichert |

Nicht getestet: `new-terminal-directory home`, `scrollback-lines` zur Laufzeit, gespeicherte
Fensterbreite nach Größenänderung per Maus.

## Schritt 3: Profile und Farbschemata

Entscheidung Christoph (2026-10-18): Ptyxis-Paletten-Format mit Hell/Dunkel-Paaren und
root-/SSH-Farben, kuratierte Auswahl von 10–15, Import von Ptyxis- und Tilix-Schemata,
Auswahl als Kacheln mit Live-Vorschau.

### Umgesetzt

- **Kern `palette`:** Parser für Ptyxis-Paletten (`[Palette]`, `[Light]`, `[Dark]`; ältere
  Dateien mit Farben nur in `[Palette]` gelten für beide Fassungen), Ableitung fehlender
  root-/SSH-Hintergründe nach den Regeln von Ptyxis, Import von Tilix-JSON, Rückschreiben im
  Ptyxis-Format. 9 Tests, u. a.: alle eingebauten Schemata haben echte helle und dunkle Fassung,
  Rundreise Lesen → Schreiben → Lesen ist verlustfrei.
- **14 eingebaute Schemata** (`data/palettes`, Herkunft in `data/palettes/README.md`): GNOME,
  Hoher Kontrast, Tango, Solarized, Selenized, Nord, Dracula, Catppuccin, Gruvbox, Everforest,
  One, GitHub, Papercolor, VS Code – alle aus Ptyxis (GPL-3.0-or-later), Farben unverändert.
- **Kern `profile`:** Profil als TOML-Datei (`~/.config/parkett/profiles/<id>.toml`) mit Name,
  Farbschema, Befehl statt Shell, Login-Shell, Startverzeichnis, Umgebungsvariablen, Verhalten
  bei Prozessende (Leiste / schließen bei Erfolg / neu starten). Unbekannte Felder stören nicht,
  kaputte Dateien werden gemeldet statt gelöscht. 3 Tests.
- **App:** Farben je Bereich aus der Palette seines Profils, Fassung nach Systemstil, live bei
  Wechsel von Stil, Schema oder Profil. Abblendung in der Hintergrundfarbe des Schemas.
  Neue Bereiche erben das Profil des aktiven Bereichs; „Neuer Reiter mit Profil“ im Menü am
  Neu-Knopf (Standardprofil zuerst). Neustart-Schutz: endet ein Prozess innerhalb von 2 s,
  startet er nicht automatisch neu. Reiter ohne OSC-Titel heißen nach dem Befehl des Profils.
- **Einstellungen** (Strg+, und Menü): AdwPreferencesDialog mit „Allgemein“ (alle
  GSettings-Schlüssel) und „Profile“ (Liste, neues Profil, Editor als Unterseite). Farbschemata
  als Kacheln (links hell, rechts dunkel), Auswahl gilt sofort; „Importieren …“ mit Dateifilter.
  Neue GSettings-Schlüssel `default-profile`.

### Tests (VM fedora44-wor)

| Test | Ergebnis |
|---|---|
| `cargo test`: 24 Kern-Tests + 1 App-Test, `cargo clippy` | grün, keine Befunde |
| Erster Start legt `standard.toml` mit Schema „gnome“ an | ok |
| Einstellungen: Seite „Allgemein“, Seite „Profile“, Editor mit Kacheln | ok (`mvp-farbschemata.png`) |
| Kachel „Tango“ anklicken → beide laufenden Bereiche sofort Tango, Profil gespeichert | ok |
| Dunkelmodus → dunkle Tango-Fassung, Abblendung passend | ok (`mvp-tango-hell-dunkel.png`) |
| Import Tilix `monokai.json` → `monokai-dark.palette`, Hinweis „nur eine Fassung“, Profil nutzt es | ok (`mvp-import.png`) – Kachelauswahl folgte **erst nach Korrektur** |
| Import Ptyxis-Palette mit nur `[Palette]` (Afterglow) → Kachel ausgewählt | ok |
| „+“ legt neues Profil an, Umbenennen per Eingabefeld wird gespeichert | ok |
| Profil mit Befehl `sleep 3` + „Neu starten“: neuer Prozess alle 3 s | ok |
| „Bereich schließen (bei Erfolg)“: Reiter schließt nach Ende von `sleep 3`, keine Prozesse übrig | ok |
| Menü „Neuer Reiter mit Profil“ öffnet Reiter mit gewähltem Profil, Titel „sleep 3“ | ok (**nach Korrektur**: erst „Standard (Standard)“ und „Shell“) |

Nicht getestet: Umgebungsvariablen, festes Startverzeichnis, Login-Shell, Profil löschen,
Fehlermeldung bei kaputter Importdatei, Neustart-Schutz (< 2 s). Befehl und Verhalten bei Ende
wurden für den Test in der TOML-Datei gesetzt, nicht über die Eingabefelder.

### Offen / beobachtet

- Die Id eines Profils entsteht aus dem ersten Namen („neues-profil“) und bleibt beim
  Umbenennen gleich – unsichtbar in der Oberfläche, aber in Dateinamen sichtbar.
- Importierte Schemata erscheinen bis zum nächsten Start am Ende statt alphabetisch.
- Kontextfarbe je Bereich (root/SSH) ist in den Paletten vorhanden, wird aber erst mit
  Schritt 7 angewendet.

## Schritt 5: Suche, Links, Tastenkürzel

### Umgesetzt

- **Suche je Bereich** (Strg+Umschalt+F, Knopf in der Kopfleiste, Kontextmenü): Leiste oben
  rechts im Bereich mit Suchfeld, „Aa“ (Groß/Klein), „.*“ (regulärer Ausdruck), Treffer
  darüber/darunter, Schließen. Sucht vom Ende her (neueste Ausgabe zuerst); Eingabe = Treffer
  darüber, Umschalt+Eingabe = darunter, Strg+Umschalt+G/H auch ohne offene Leiste. Kein
  Treffer oder ungültiger Ausdruck: Feld rot, Fehler als Tooltip. Markierter Text wird
  Suchbegriff. VTE-Suche mit PCRE2 (Flags wie Ptyxis), Text wird ohne „.*“ maskiert.
- **Links:** Adressen (http, https, ftp, file, mailto) per Regex erkannt, Satzzeichen am Ende
  gehören nicht dazu; OSC-8-Links von VTE. **Strg+Klick** öffnet über `GtkUriLauncher`, nur mit
  erlaubtem Schema (kein javascript:, data: …). Kontextmenü „Link öffnen“ / „Link kopieren“,
  nur sichtbar, wenn ein Link unter dem Zeiger liegt. **OSC-8-Links zeigen ihr wahres Ziel als
  Tooltip** (der sichtbare Text kann täuschen); das Signal bindet vte4-rs nicht, daher
  `connect_local`.
- **Tastenkürzel änderbar:** zentrale Liste `shortcuts.rs` (32 Aktionen mit Standardbelegung),
  eigene Belegungen in GSettings `custom-shortcuts` (`a{ss}`). Seite „Tastenkürzel“ in den
  Einstellungen: Zeile anklicken, Kombination drücken; Esc bricht ab, Rücktaste schaltet aus,
  „Standard wiederherstellen“, „Alle zurücksetzen“. Konflikte werden abgelehnt mit Hinweis auf
  die belegte Aktion; ohne Strg/Alt/Super sind nur F-Tasten erlaubt. Während der Aufnahme
  sind Fenster- und App-Kürzel abgeschaltet. Fenster, Terminals (inkl. Kontextmenü-Anzeige),
  App-Accels und die Kürzel-Übersicht folgen Änderungen sofort.

### Tests (VM fedora44-wor)

| Test | Ergebnis |
|---|---|
| Suche „apfel“: findet zuerst „APFEL“ (unten), Eingabe → „apfel“ darüber; „apfelbanane“ → rot | ok (`mvp-suche.png`) |
| Strg+Klick auf `https://linuxundich.de/artikel/test,` öffnet Firefox mit der Adresse ohne Komma | ok |
| Zeiger über OSC-8-Text „Hier klicken“ → Tooltip `https://example.org/wahres-ziel` | ok (`mvp-osc8-tooltip.png`) |
| Rechtsklick auf `mailto:…` → „Link öffnen/kopieren“; Kopieren → `wl-paste` = Adresse | ok (`mvp-link-menue.png`) |
| Rechtsklick auf `javascript:alert(1)` → keine Link-Einträge | ok |
| Kürzel-Seite listet alle Aktionen | ok (`mvp-kuerzel-seite.png`) |
| „Neuer Reiter“ → Strg+Umschalt+X → Hinweis „schon Bereich maximieren“, nichts ausgelöst | ok (`mvp-kuerzel-konflikt.png`) |
| Strg+Alt+E aufgenommen → GSettings `{'win.new-tab': '<Control><Alt>e'}`; Strg+Umschalt+T wirkungslos, Strg+Alt+E öffnet Reiter (live) | ok |
| Nach der Aufnahme wirkt Strg+, wieder; Zeile zeigt „Eigene Belegung“; „Alle zurücksetzen“ → `{}` | ok |
| Kürzel-Übersicht (Strg+?) zeigt aktuelle Belegung, keine GTK-Warnungen | ok |

### Gefunden und behoben

- **Absturz beim Start:** `glib::VariantDict` verlangt `a{sv}`, der Schlüssel ist `a{ss}` →
  Lesen und Schreiben als `HashMap<String, String>`.
- Ungültige Kürzel-Texte in der Übersicht („Strg+Klick“, „1…9“) ersetzt.

Nicht getestet: Regex- und Groß/Klein-Schalter der Suche, Änderung eines Terminal-Kürzels
(gleicher Mechanismus wie Fensterkürzel), `file://`-Links.
