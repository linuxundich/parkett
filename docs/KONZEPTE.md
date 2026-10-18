# Phase 2 – Designkonzepte und Bedienkonzept

Stand: 2026-10-18. Mockups: `mockups/` (Start: `mockups/index.html`, lokal über
`python3 -m http.server --directory mockups 8751` oder die Vorschau-Konfiguration `terminal-mockups`).

Alle drei Konzepte zeigen **dieselbe Fenstergröße (1280×800, zusätzlich 720×540), dieselben
fünf Sitzungen, dieselben Terminal-Inhalte und dasselbe Split-Layout** (lui-theme: links Shell,
rechts oben `npm run dev`, rechts unten `ssh bender`). Farben stammen 1:1 aus dem Stylesheet von
libadwaita 1.10, Icons aus Adwaita. Jede Seite hat Umschalter für Ansicht, Hell/Dunkel und die
nummerierten Erläuterungen.

Was die Mockups sind und was nicht: Es sind **lauffähige HTML/CSS-Prototypen** der Oberfläche,
keine GTK-Anwendung. Abstände und Größen orientieren sich an libadwaita (Headerbar 47 px,
Tab-Leiste 40 px, Zeilen 52 px), Schrift ist Adwaita Sans/Mono. Interaktionen (Klicken, Ziehen)
sind nicht programmiert – die Ansichten zeigen Zustände.

---

## Messung der Terminalfläche (im Mockup gemessen)

Gemessen per Skript (`shared/content.js: measure()`), Schrift Adwaita Mono 13 px, Zeilenhöhe 17 px.
Spalten×Zeilen je Bereich: links | rechts oben | rechts unten.

| | Fenster 1280×800 | Bereiche | Fenster 720×540 | Bereiche |
|---|---|---|---|---|
| **A GNOME Native** | 89,0 % | 89×41 · 70×20 · 70×20 | 83,7 % | 49×25 · 38×12 · 38×12 |
| **B Arbeitsbereiche** | 70,9 % (Seitenleiste offen) | 71×41 · 55×19 · 55×19 | 83,6 % (Leiste zu) | 49×26 · 38×12 · 38×12 |
| **C Terminalzentriert** | 95,6 % | 89×44 · 70×21 · 70×21 | 93,5 % | 49×29 · 38×14 · 38×14 |

Einordnung: A blendet die Tab-Leiste bei nur einem Tab aus (dann ca. 94 %). B mit eingeklappter
Seitenleiste liegt bei großen Fenstern ungefähr bei A minus die Titelzeilen je Bereich (28 px).

---

## Konzept A – GNOME Native

**Grundidee:** So, wie eine Kern-App von GNOME aussähe, wenn sie Splits könnte. Headerbar mit
Split-Knopf (neuer Tab | Profil/Vorlage), darunter `AdwTabBar`, darunter die Splits. Jeder Tab ist
eine Tilix-„Sitzung“ mit eigenem Split-Baum. Fokus über eine Akzentlinie oben am Bereich plus
leichtes Abblenden der anderen. Über jedem Bereich schwebt – nur wenn der Tab geteilt ist – ein
kleiner Titel mit Maximieren/Schließen, der zugleich Griff zum Ziehen ist. Splits verwaltet man über
das Kontextmenü (mit Layout-Kacheln) und Kürzel; Navigation über viele Tabs per `AdwTabOverview`.
Einstellungen im `AdwPreferencesDialog`.

| Kriterium | Bewertung |
|---|---|
| Vorteile | Sofort vertraut für GNOME-Nutzer (wie Console/Ptyxis); fast alles aus Standard-Widgets; Tab-Übersicht gratis; gute Mausbedienung |
| Nachteile | Zwei Leisten oben (87 px) bei mehreren Tabs; Splits sind „nur“ im Kontextmenü sichtbar; wenig eigenes Profil gegenüber Ptyxis |
| Platz | 89 % (94 % bei einem Tab) |
| Maus/Tastatur | Maus sehr gut (Tabs, Menüs, Ziehen); Tastatur gut über Kürzel, aber ohne zentrale Suche |
| Komplexe Splits | gut bis 4–6 Bereiche; schwebende Titel werden bei sehr schmalen Bereichen eng |
| GNOME-Integration | sehr hoch |
| Aufwand | **niedrig–mittel** – Hauptaufwand ist der Split-Baum, der in allen Konzepten gleich ist |
| Kleine Fenster | Tab-Leiste scrollt, Übersicht hilft; ab 720 px knapp für drei Bereiche |
| Große Fenster | gut; Tab-Leiste bleibt schmal relativ zur Fläche |

## Konzept B – Arbeitsbereiche

**Grundidee:** Statt Tabs eine Seitenleiste (`AdwOverlaySplitView`) mit **Arbeitsbereichen**: ein
Arbeitsbereich ist ein Projekt mit Verzeichnis, Profil und Split-Layout. Aufgeklappt zeigt er seine
Terminals einzeln mit Status (läuft, Fehler mit Exit-Code). Darunter lose Terminals und
**Vorlagen**, die ein fertiges Layout in einem Verzeichnis öffnen. Jeder Bereich hat eine flache
Titelzeile (Tilix-Erbe): Prozess, Host-Chip, Verzeichnis, Teilen/Maximieren/Schließen. Splits
lassen sich in einem grafischen **Layout-Editor** umbauen, mit Eigenschaften je Bereich
(Profil, Startbefehl, Verhalten bei Prozessende). Einstellungen im Fenster mit eigener Navigation.

| Kriterium | Bewertung |
|---|---|
| Vorteile | Beste Übersicht bei vielen parallelen Projekten; jedes Terminal direkt anspringbar; Vorlagen und Startbefehle machen wiederkehrende Layouts zu einem Klick; Titelzeilen sind klare DnD-Griffe; Fokus durch eingefärbte Titelzeile sehr deutlich |
| Nachteile | Seitenleiste kostet 260 px (29 % weniger Fläche als A bei offener Leiste); Titelzeilen kosten 28 px je Bereich; neues Datenmodell „Arbeitsbereich“ muss man verstehen; weicher Übergang zwischen „Arbeitsbereich“ und „Tab“ nötig |
| Platz | 71 % offen, ca. 84–86 % eingeklappt |
| Maus/Tastatur | Maus sehr gut; Tastatur gut (Strg+1…9, F9 für Leiste), Liste per Pfeiltasten |
| Komplexe Splits | sehr gut – Editor zeigt Struktur, Anteile und Eigenschaften |
| GNOME-Integration | hoch (GNOME-Settings-artig), aber unüblich für Terminals |
| Aufwand | **hoch** – Seitenleiste mit Baum, Layout-Editor, Vorlagen-/Startbefehl-Modell, Einstellungen im Fenster |
| Kleine Fenster | Leiste klappt ein (Overlay); Titelzeilen fallen ins Gewicht |
| Große Fenster | am besten von allen – nutzt Breite sinnvoll |

## Konzept C – Terminalzentriert

**Grundidee:** Keine Headerbar. Oben 8 px Ziehrand, Fenstersteuerung schwebt ein, wenn der Zeiger
oben rechts ist (Muster Black Box `floating-controls`). Unten eine abschaltbare **Statuszeile**
(26 px) mit nummerierten Sitzungen, Status-Punkten und Hinweis auf die Befehlspalette. Navigation
über die **Befehlspalette** (Strg+Umschalt+P): Sitzungen, einzelne Bereiche, Aktionen, Vorlagen,
Profile in einer Liste. Splits verwaltet ein **Layout-Modus** (Strg+Umschalt+L): Bereiche bekommen
Nummern, danach genügen Einzeltasten (Pfeile, R/D teilen, Z zoomen, = angleichen, X schließen), die
Hilfe ist eingeblendet. Einstellungen als **Seitenpanel neben dem echten Terminal** (Live-Vorschau),
dazu Zugang zur Konfigurationsdatei.

| Kriterium | Bewertung |
|---|---|
| Vorteile | Maximale Fläche (96 %); sehr schnell für Tastaturnutzer; Palette macht alles auffindbar, ohne Knöpfe; Live-Vorschau der Einstellungen |
| Nachteile | Funktionen sind unsichtbar, bis man sie kennt (Entdeckbarkeit); Fenster ohne Headerbar ist unter GNOME ungewohnt (Ziehen, Doppelklick zum Maximieren nur am Rand); modaler Layout-Modus ist für Mausnutzer fremd; Barrierefreiheit der schwebenden Steuerung muss eigens gelöst werden |
| Platz | 95,6 % (ohne Statuszeile ca. 99 %) |
| Maus/Tastatur | Tastatur hervorragend; Maus ausreichend (Kontextmenü, Trenner, Statuszeile klickbar) |
| Komplexe Splits | gut – Nummern machen Bereiche ansprechbar; Struktur sieht man aber nur im Modus |
| GNOME-Integration | mittel – nutzt Adw-Bausteine, bricht aber mit der Headerbar-Konvention |
| Aufwand | **mittel** – Palette (Suche, Ranking) und Layout-Modus sind Eigenbau, dafür entfallen Seitenleiste und Editor |
| Kleine Fenster | am besten – fast alles bleibt Terminal |
| Große Fenster | gut; Statuszeile wirkt bei sehr breiten Fenstern verloren |

---

## Gegenüberstellung

| | A GNOME Native | B Arbeitsbereiche | C Terminalzentriert |
|---|---|---|---|
| Hauptnavigation | Tab-Leiste + Übersicht | Seitenleiste (Projekte → Terminals) | Befehlspalette + Statuszeile |
| Splits bedienen | Kontextmenü, Kürzel, Ziehen am schwebenden Titel | Knöpfe in Titelzeilen, Layout-Editor | Layout-Modus, Kürzel, Kontextmenü |
| Fokus-Kennzeichnung | Akzentlinie + Abblenden | eingefärbte Titelzeile | Akzentrahmen + Abblenden |
| Terminalfläche 1280×800 | 89 % | 71 % / ~85 % | 96 % |
| Entdeckbarkeit | hoch | sehr hoch | niedrig |
| Tastatur-Effizienz | gut | gut | sehr hoch |
| GNOME-Konformität | sehr hoch | hoch | mittel |
| Aufwand UI | niedrig–mittel | hoch | mittel |
| Tilix-Nähe | mittel (Tabs-Modus von Tilix) | hoch (Seitenleiste, Titelzeilen) | gering |

### Bausteine, die sich kombinieren lassen

Diese Elemente sind weitgehend unabhängig vom Grundkonzept:

1. **Befehlspalette** (aus C) – passt in A und B als Zusatz (P1).
2. **Layout-Modus** (aus C) – als Tastatur-Alternative in A.
3. **Vorlagen mit Startbefehlen** (aus B) – in A als Seite „Layouts“ und im Neu-Knopf-Menü.
4. **Kontextfarbe je Bereich** (root/ssh/Container, Farben aus dem Schema wie Ptyxis) – in allen.
5. **Kopfloser Modus** (aus C) – als Einstellung in A („Headerbar ausblenden“), wie Black Box.
6. **Titelzeile je Bereich** (aus B) als Option in A statt schwebendem Titel.
7. **Einstellungen mit Live-Vorschau** (aus C) – für den Profil-Editor in A oder B.

---

## Bedienkonzept: Abläufe

Gemeinsame Kürzel (Vorschlag, an Tilix angelehnt, ohne Kollision mit GNOME-Standards):

| Aktion | Kürzel | Maus |
|---|---|---|
| Neuer Tab/Sitzung (Standardprofil) | Strg+Umschalt+T | + in Headerbar (A), + in Leiste (B), schwebender + (C) |
| Neuer Tab mit Profil/Vorlage | Strg+Umschalt+Alt+T → Auswahl | Pfeil am Split-Knopf (A), Vorlagen (B), Palette (C) |
| Rechts / unten / automatisch teilen | Strg+Alt+R / D / A | Kontextmenü; Knöpfe in Titelzeile (B) |
| Fokus wechseln | Alt+Pfeil, Alt+1…9 (Bereich), Strg+Tab (zuletzt) | Klick |
| Größe ändern | Umschalt+Alt+Pfeil | Trenner ziehen, Doppelklick = angleichen |
| Maximieren/Zoom | Strg+Umschalt+X | Knopf im Titel |
| Bereich schließen | Strg+Umschalt+W | × im Titel, Mittelklick auf Titel |
| Sitzung wechseln | Strg+Bild auf/ab, Strg+Alt+1…9 | Tab/Leiste/Statuszeile |
| Umbenennen | F2 (Sitzung), Umschalt+F2 (Bereich) | Doppelklick auf Tab/Titel |
| Übersicht / Palette | Strg+Umschalt+O / Strg+Umschalt+P | Knopf |
| Suchen | Strg+Umschalt+F | Lupe |

### Szenario 1 – Morgens das Theme-Projekt öffnen
- **A:** Pfeil am Neu-Knopf → *Vorlage „Web-Projekt“* → Verzeichnis wählen → Tab mit drei Bereichen
  öffnet sich, `npm run dev` startet. Tastatur: Strg+Umschalt+Alt+T, Pfeiltasten, Eingabe.
- **B:** Arbeitsbereich *lui-theme* steht in der Leiste (wiederhergestellt) → ein Klick; oder
  *Vorlagen → Web-Projekt*.
- **C:** Strg+Umschalt+P, „web“ tippen, Eingabe.

### Szenario 2 – Nebenbei auf dem Server nachsehen
- **A:** Rechtsklick im rechten Bereich → *Unten teilen*, Profil „Server (bender)“ wechselt beim
  `ssh` automatisch; Bereich bekommt Kontextfarbe.
- **B:** Knopf *Unten teilen* in der Titelzeile; Host-Chip „bender“ erscheint.
- **C:** Strg+Alt+D, `ssh bender`; oder Palette „bender“ → *Rechts teilen mit Profil Server*.

### Szenario 3 – Test schlägt fehl, Prozess endet
- Alle: Bereich bleibt stehen, unten Leiste *Prozess beendet · Exit-Code 101* mit
  *Neu starten*/*Schließen*; Tab/Leiste/Statuszeile zeigt einen roten Punkt bzw. Warnsymbol.
  Läuft das Fenster im Hintergrund und dauerte der Befehl > 10 s, optional Desktop-Benachrichtigung.

### Szenario 4 – Viele Tabs (z. B. 12)
- **A:** Tab-Leiste scrollt; Strg+Umschalt+O öffnet die Übersicht mit Suche (Titel, Verzeichnis, Befehl).
- **B:** Leiste listet alle; zugeklappte Arbeitsbereiche halten sie kurz.
- **C:** Statuszeile kürzt auf Nummern + „+n“; Palette ist der Weg.

### Szenario 5 – Layout umbauen, ohne Terminals zu verlieren
- **A:** Bereich am schwebenden Titel ziehen, Ablagezone zeigt „unten teilen“; oder Kontextmenü
  → Layout-Kachel.
- **B:** *Layout bearbeiten* → Editor, Trenner ziehen, Bereiche tauschen, *Fertig*.
- **C:** Strg+Umschalt+L → Nummern → M (tauschen) / Umschalt+Pfeil → Esc.

### Command Palette vs. Kontextmenü vs. Werkzeugleiste

| Variante | Stärke | Schwäche | Einschätzung |
|---|---|---|---|
| Befehlspalette | findet alles, auch selten Genutztes; keine Knöpfe; Tastatur | unsichtbar für Gelegenheitsnutzer; Ranking/Suche ist Eigenbau | als Ergänzung in jedem Konzept sinnvoll, als einzige Navigation nur in C |
| Kontextmenü am Bereich | ortsbezogen (gilt für *diesen* Bereich); GNOME-üblich; Menütaste | Rechtsklick kollidiert mit Programmen, die Maus nutzen (mc, vim mit mouse=a) → dann Umschalt+Rechtsklick | in allen Konzepten nötig |
| Kompakte Werkzeugleiste | sichtbar, entdeckbar | kostet Fläche in jedem Bereich; wiederholt sich bei vielen Splits | nur als Titelzeile (B) oder schwebend (A), nicht als zusätzliche Leiste |

---

## Fachliche Empfehlung (keine Entscheidung)

**S:** Ich empfehle **A als Grundgerüst, ergänzt um die Befehlspalette und den Layout-Modus aus C
sowie die Vorlagen mit Startbefehlen aus B.**

Begründung:
- A hat das beste Verhältnis aus Vertrautheit, Fläche (89–94 %) und Aufwand; die
  Standard-Widgets (`AdwTabView`, `AdwTabOverview`, `AdwPreferencesDialog`) tragen viel.
- Die Stärke von C (Tastatur, Palette) lässt sich in A nachrüsten, ohne dass Gelegenheitsnutzer sie
  kennen müssen. Umgekehrt lässt sich die fehlende Entdeckbarkeit von C kaum nachrüsten.
- B ist das eigenständigste Konzept und für viele parallele Projekte am stärksten, kostet aber
  dauerhaft Fläche und am meisten Entwicklungszeit. Sein wertvollster Teil – Vorlagen, die ein Layout
  mit Startbefehlen in einem Verzeichnis öffnen – funktioniert auch in A.
- Offen bleibt in A, ob der Bereichstitel schwebend (wie gezeigt) oder als flache Titelzeile (B)
  kommt. Schwebend spart Fläche; die Titelzeile ist der bessere Griff zum Ziehen und zeigt den Fokus
  deutlicher.

---

## Entscheidung und Konzept A+ (2026-10-18)

**Entscheidung (Christoph):** A als Grundgerüst, ergänzt um Befehlspalette und Layout-Modus aus C
und die Vorlagen mit Startbefehlen aus B. Mockup: `mockups/combined.html` (elf Ansichten, Schalter
für die offene Frage „Bereichstitel“).

### Was A+ gegenüber A ändert

| Baustein | Herkunft | Umsetzung im Mockup |
|---|---|---|
| Befehlspalette (Strg+Umschalt+P) | C | über Headerbar und Tabs; Gruppen Sitzungen/Bereiche, Aktionen, Vorlagen; zeigt zu jeder Aktion das Kürzel; Präfixe `>` `@` `#` |
| Layout-Modus (Strg+Umschalt+L) | C | nummerierte Bereiche, Einzeltasten, Hilfeleiste unten, Pille „LAYOUT“ im Fenstertitel; alle Kürzel bleiben auch ohne Modus nutzbar |
| Vorlagen mit Startbefehlen | B | im Menü am Neu-Knopf neben den Profilen; Dialog „Vorlage öffnen“ (Verzeichnis, Startbefehle ein/aus, zuletzt benutzte Verzeichnisse); „Aktuellen Tab als Vorlage speichern“; Editor als Unterseite der Einstellungen |
| Layout-Editor | B | **nur für Vorlagen** – im laufenden Tab genügen Kontextmenü, Ziehen und Layout-Modus |

### Neue Vorschläge in A+ (übernommen am 2026-10-18)

1. **Kontextfarbe je Bereich:** SSH/Root/Container-Bereiche bekommen eine farbige Linie oben,
   einen Host-Chip und einen leicht getönten Grund. Farbe aus dem Farbschema (Muster Ptyxis),
   abschaltbar. Kein Vorbild hat das je Split, weil keines außer Ghostty Splits hat.
2. **Fenstertitel als Palette-Auslöser:** Klick auf Titel/Untertitel der Headerbar öffnet die Palette.
   Macht die Palette ohne zusätzlichen Knopf entdeckbar.
3. **Maximieren sichtbar:** Hinweis „Maximiert · 2 Bereiche ausgeblendet · Alle zeigen“ und „1/3“ am Tab.
4. **Rückgängig** für geschlossene Bereiche und Tabs per `AdwToast` (Muster Ghostty). Offen ist,
   wie lange der Prozess dabei weiterläuft (Vorschlag: bis der Toast verschwindet).
5. **Startbefehl wird in die Shell getippt** (`feed_child`), statt die Shell zu ersetzen: Nach
   Strg+C bleibt der Bereich als Shell nutzbar, und der Befehl steht im Verlauf. Alternative:
   Befehl direkt starten, Bereich hält bei Ende. Beides ist technisch einfach.
6. **Startbefehle nie still ausführen:** Schalter im Öffnen-Dialog, Standard „an“ nur für eigene
   Vorlagen; importierte Vorlagen zeigen ihre Befehle vor dem ersten Lauf.

### Gestaltungsfrage Bereichstitel (entschieden: schwebend)

Im Mockup per Schalter „Titel schwebend / Titel als Zeile“ vergleichbar (gilt nur bei geteilten Tabs).

| | Schwebend | Als Zeile |
|---|---|---|
| Terminalfläche 1280×800 (gemessen) | 89,0 % · 89×41, 70×20, 70×20 | 84,0 % · 89×39, 70×18, 70×19 |
| Fokus | Akzentlinie + Abblenden | eingefärbte Zeile (deutlicher) |
| Ziehen | am kleinen Titel | an der ganzen Zeile (leichter zu treffen) |
| Information | Prozess, Host | Prozess, Host, Verzeichnis, Status-Punkt |
| Risiko | verdeckt rechts oben Terminalinhalt (z. B. htop-Uhr) | wiederholt Info aus Tab/Headerbar |

**S:** Beide sind gleich teuer umzusetzen; als Einstellung lässt sich die andere Variante anbieten.
Zu entscheiden ist der Standard.

### Freigabe A+ (2026-10-18)

Christoph hat A+ freigegeben: **Bereichstitel schwebend** als Standard („als Zeile“ als
Einstellung), **Vorschläge 1–6 übernommen**. Weiter mit Phase 3, siehe `docs/PROTOTYP.md`.
