# Phase 2 – Design concepts and interaction concept

As of: 2026-10-18. Mockups: `mockups/` (start: `mockups/index.html`, locally via
`python3 -m http.server --directory mockups 8751` or the preview configuration `terminal-mockups`).

All three concepts show **the same window size (1280×800, plus 720×540), the same
five sessions, the same terminal contents and the same split layout** (lui-theme: shell on the left,
`npm run dev` top right, `ssh bender` bottom right). Colors are taken 1:1 from the libadwaita 1.10
stylesheet, icons from Adwaita. Each page has toggles for the view, light/dark and the
numbered explanations.

What the mockups are and what they are not: they are **working HTML/CSS prototypes** of the interface,
not a GTK application. Spacing and sizes follow libadwaita (header bar 47 px,
tab bar 40 px, rows 52 px), the fonts are Adwaita Sans/Mono. Interactions (clicking, dragging)
are not programmed – the views show states.

---

## Measuring the terminal area (measured in the mockup)

Measured by script (`shared/content.js: measure()`), font Adwaita Mono 13 px, line height 17 px.
Columns×rows per pane: left | top right | bottom right.

| | Window 1280×800 | Panes | Window 720×540 | Panes |
|---|---|---|---|---|
| **A GNOME Native** | 89.0 % | 89×41 · 70×20 · 70×20 | 83.7 % | 49×25 · 38×12 · 38×12 |
| **B Workspaces** | 70.9 % (sidebar open) | 71×41 · 55×19 · 55×19 | 83.6 % (sidebar closed) | 49×26 · 38×12 · 38×12 |
| **C Terminal-centric** | 95.6 % | 89×44 · 70×21 · 70×21 | 93.5 % | 49×29 · 38×14 · 38×14 |

Context: A hides the tab bar when there is only one tab (then approx. 94 %). B with the sidebar
collapsed is, in large windows, roughly at A minus the title rows per pane (28 px).

---

## Concept A – GNOME Native

**Basic idea:** What a GNOME core app would look like if it could do splits. Header bar with a
split button (new tab | profile/template), below it `AdwTabBar`, below that the splits. Each tab is
a Tilix "session" with its own split tree. Focus is shown by an accent line at the top of the pane plus
slight dimming of the others. Above each pane – only when the tab is split – floats a
small title with maximize/close, which also serves as a handle for dragging. Splits are managed via
the context menu (with layout tiles) and shortcuts; navigation across many tabs via `AdwTabOverview`.
Preferences in an `AdwPreferencesDialog`.

| Criterion | Assessment |
|---|---|
| Advantages | Immediately familiar to GNOME users (like Console/Ptyxis); almost everything from standard widgets; tab overview for free; good mouse operation |
| Disadvantages | Two bars at the top (87 px) with several tabs; splits are "only" visible in the context menu; little distinct identity compared to Ptyxis |
| Space | 89 % (94 % with one tab) |
| Mouse/keyboard | mouse very good (tabs, menus, dragging); keyboard good via shortcuts, but without central search |
| Complex splits | good up to 4–6 panes; floating titles get cramped in very narrow panes |
| GNOME integration | very high |
| Effort | **low–medium** – the main effort is the split tree, which is the same in all concepts |
| Small windows | tab bar scrolls, overview helps; from 720 px tight for three panes |
| Large windows | good; the tab bar stays narrow relative to the area |

## Concept B – Workspaces

**Basic idea:** Instead of tabs, a sidebar (`AdwOverlaySplitView`) with **workspaces**: a
workspace is a project with a directory, profile and split layout. When expanded, it shows its
terminals individually with status (running, error with exit code). Below that, loose terminals and
**templates** that open a ready-made layout in a directory. Each pane has a flat
title row (Tilix heritage): process, host chip, directory, split/maximize/close. Splits
can be rearranged in a graphical **layout editor**, with properties per pane
(profile, start command, behavior on process exit). Preferences in a window with its own navigation.

| Criterion | Assessment |
|---|---|
| Advantages | Best overview with many parallel projects; every terminal can be jumped to directly; templates and start commands turn recurring layouts into a single click; title rows are clear DnD handles; focus very clear thanks to the colored title row |
| Disadvantages | Sidebar costs 260 px (29 % less area than A with the sidebar open); title rows cost 28 px per pane; the new data model "workspace" has to be understood; a smooth transition between "workspace" and "tab" is needed |
| Space | 71 % open, approx. 84–86 % collapsed |
| Mouse/keyboard | mouse very good; keyboard good (Ctrl+1…9, F9 for the sidebar), list via arrow keys |
| Complex splits | very good – the editor shows structure, proportions and properties |
| GNOME integration | high (GNOME Settings-like), but unusual for terminals |
| Effort | **high** – sidebar with a tree, layout editor, template/start command model, preferences in a window |
| Small windows | sidebar collapses (overlay); title rows weigh heavily |
| Large windows | best of all – uses the width sensibly |

## Concept C – Terminal-centric

**Basic idea:** No header bar. An 8 px drag border at the top, window controls float in when the pointer
is at the top right (Black Box `floating-controls` pattern). At the bottom an optional **status bar**
(26 px) with numbered sessions, status dots and a hint about the command palette. Navigation
via the **command palette** (Ctrl+Shift+P): sessions, individual panes, actions, templates,
profiles in one list. Splits are managed by a **layout mode** (Ctrl+Shift+L): panes get
numbers, after which single keys are enough (arrows, R/D to split, Z to zoom, = to equalize, X to close), and the
help is shown on screen. Preferences as a **side panel next to the real terminal** (live preview),
plus access to the configuration file.

| Criterion | Assessment |
|---|---|
| Advantages | Maximum area (96 %); very fast for keyboard users; the palette makes everything findable without buttons; live preview of the preferences |
| Disadvantages | Features are invisible until you know them (discoverability); a window without a header bar is unusual on GNOME (dragging, double-click to maximize only at the edge); the modal layout mode is alien to mouse users; accessibility of the floating controls has to be solved separately |
| Space | 95.6 % (approx. 99 % without the status bar) |
| Mouse/keyboard | keyboard excellent; mouse adequate (context menu, separators, status bar clickable) |
| Complex splits | good – numbers make panes addressable; but the structure is only visible in the mode |
| GNOME integration | medium – uses Adw building blocks but breaks with the header bar convention |
| Effort | **medium** – palette (search, ranking) and layout mode are custom-built, but sidebar and editor are not needed |
| Small windows | best – almost everything remains terminal |
| Large windows | good; the status bar looks lost in very wide windows |

---

## Comparison

| | A GNOME Native | B Workspaces | C Terminal-centric |
|---|---|---|---|
| Main navigation | tab bar + overview | sidebar (projects → terminals) | command palette + status bar |
| Operating splits | context menu, shortcuts, dragging by the floating title | buttons in title rows, layout editor | layout mode, shortcuts, context menu |
| Focus indication | accent line + dimming | colored title row | accent border + dimming |
| Terminal area 1280×800 | 89 % | 71 % / ~85 % | 96 % |
| Discoverability | high | very high | low |
| Keyboard efficiency | good | good | very high |
| GNOME conformity | very high | high | medium |
| UI effort | low–medium | high | medium |
| Closeness to Tilix | medium (Tilix's tabs mode) | high (sidebar, title rows) | low |

### Building blocks that can be combined

These elements are largely independent of the basic concept:

1. **Command palette** (from C) – fits into A and B as an addition (P1).
2. **Layout mode** (from C) – as a keyboard alternative in A.
3. **Templates with start commands** (from B) – in A as a *Layouts* page and in the new-button menu.
4. **Context color per pane** (root/ssh/container, colors from the scheme like Ptyxis) – in all of them.
5. **Headerless mode** (from C) – as a setting in A (*Headerbar ausblenden* (Hide header bar)), like Black Box.
6. **Title row per pane** (from B) as an option in A instead of the floating title.
7. **Preferences with live preview** (from C) – for the profile editor in A or B.

---

## Interaction concept: workflows

Common shortcuts (proposal, based on Tilix, without collisions with GNOME defaults):

| Action | Shortcut | Mouse |
|---|---|---|
| New tab/session (default profile) | Ctrl+Shift+T | + in the header bar (A), + in the sidebar (B), floating + (C) |
| New tab with profile/template | Ctrl+Shift+Alt+T → selection | arrow on the split button (A), templates (B), palette (C) |
| Split right / down / automatically | Ctrl+Alt+R / D / A | context menu; buttons in the title row (B) |
| Switch focus | Alt+Arrow, Alt+1…9 (pane), Ctrl+Tab (last used) | click |
| Resize | Shift+Alt+Arrow | drag the separator, double-click = equalize |
| Maximize/zoom | Ctrl+Shift+X | button in the title |
| Close pane | Ctrl+Shift+W | × in the title, middle-click on the title |
| Switch session | Ctrl+Page Up/Down, Ctrl+Alt+1…9 | tab/sidebar/status bar |
| Rename | F2 (session), Shift+F2 (pane) | double-click on tab/title |
| Overview / palette | Ctrl+Shift+O / Ctrl+Shift+P | button |
| Search | Ctrl+Shift+F | magnifier |

### Scenario 1 – Opening the theme project in the morning
- **A:** Arrow on the new button → *Vorlage „Web-Projekt“* (template "Web project") → choose a directory → a tab with three panes
  opens, `npm run dev` starts. Keyboard: Ctrl+Shift+Alt+T, arrow keys, Enter.
- **B:** The workspace *lui-theme* is in the sidebar (restored) → one click; or
  *Vorlagen → Web-Projekt* (Templates → Web project).
- **C:** Ctrl+Shift+P, type "web", Enter.

### Scenario 2 – Quickly checking something on the server
- **A:** Right-click in the right pane → *Unten teilen* (Split down), the profile *Server (bender)* switches automatically on
  `ssh`; the pane gets a context color.
- **B:** Button *Unten teilen* (Split down) in the title row; the host chip "bender" appears.
- **C:** Ctrl+Alt+D, `ssh bender`; or palette "bender" → *Rechts teilen mit Profil Server* (Split right with profile Server).

### Scenario 3 – A test fails, the process exits
- All: The pane stays, at the bottom a bar *Prozess beendet · Exit-Code 101* (Process exited · exit code 101) with
  *Neu starten* (Restart)/*Schließen* (Close); the tab/sidebar/status bar shows a red dot or warning icon.
  If the window is in the background and the command took > 10 s, optionally a desktop notification.

### Scenario 4 – Many tabs (e.g. 12)
- **A:** The tab bar scrolls; Ctrl+Shift+O opens the overview with search (title, directory, command).
- **B:** The sidebar lists everything; collapsed workspaces keep it short.
- **C:** The status bar shortens to numbers + "+n"; the palette is the way.

### Scenario 5 – Restructuring the layout without losing terminals
- **A:** Drag the pane by its floating title, the drop zone shows *unten teilen* (split down); or context menu
  → layout tile.
- **B:** *Layout bearbeiten* (Edit layout) → editor, drag separators, swap panes, *Fertig* (Done).
- **C:** Ctrl+Shift+L → numbers → M (swap) / Shift+Arrow → Esc.

### Command palette vs. context menu vs. toolbar

| Variant | Strength | Weakness | Assessment |
|---|---|---|---|
| Command palette | finds everything, including rarely used things; no buttons; keyboard | invisible to occasional users; ranking/search is custom-built | sensible as an addition in every concept, as the only navigation only in C |
| Context menu on the pane | location-based (applies to *this* pane); customary on GNOME; Menu key | right-click collides with programs that use the mouse (mc, vim with mouse=a) → then Shift+right-click | needed in all concepts |
| Compact toolbar | visible, discoverable | costs area in every pane; repeats itself with many splits | only as a title row (B) or floating (A), not as an additional bar |

---

## Professional recommendation (not a decision)

**Conclusion:** I recommend **A as the basic framework, supplemented by the command palette and the layout mode from C
as well as the templates with start commands from B.**

Rationale:
- A has the best balance of familiarity, area (89–94 %) and effort; the
  standard widgets (`AdwTabView`, `AdwTabOverview`, `AdwPreferencesDialog`) carry a lot.
- The strength of C (keyboard, palette) can be retrofitted into A without occasional users
  having to know about it. Conversely, C's lack of discoverability can hardly be retrofitted.
- B is the most distinctive concept and the strongest for many parallel projects, but it permanently
  costs area and the most development time. Its most valuable part – templates that open a layout
  with start commands in a directory – also works in A.
- What remains open in A is whether the pane title is floating (as shown) or a flat title row (B).
  Floating saves area; the title row is the better drag handle and shows the focus
  more clearly.

---

## Decision and concept A+ (2026-10-18)

**Decision (Christoph):** A as the basic framework, supplemented by the command palette and layout mode from C
and the templates with start commands from B. Mockup: `mockups/combined.html` (eleven views, a toggle
for the open question "pane title").

### What A+ changes compared to A

| Building block | Origin | Implementation in the mockup |
|---|---|---|
| Command palette (Ctrl+Shift+P) | C | above the header bar and tabs; groups sessions/panes, actions, templates; shows the shortcut for every action; prefixes `>` `@` `#` |
| Layout mode (Ctrl+Shift+L) | C | numbered panes, single keys, help bar at the bottom, pill *LAYOUT* in the window title; all shortcuts remain usable without the mode as well |
| Templates with start commands | B | in the menu on the new button next to the profiles; dialog *Vorlage öffnen* (Open template) (directory, start commands on/off, recently used directories); *Aktuellen Tab als Vorlage speichern* (Save current tab as template); editor as a subpage of the preferences |
| Layout editor | B | **only for templates** – in a running tab, the context menu, dragging and layout mode are enough |

### New proposals in A+ (adopted on 2026-10-18)

1. **Context color per pane:** SSH/root/container panes get a colored line at the top,
   a host chip and a slightly tinted background. Color from the color scheme (Ptyxis pattern),
   can be switched off. No role model has this per split, because none except Ghostty has splits.
2. **Window title as palette trigger:** Clicking the title/subtitle of the header bar opens the palette.
   Makes the palette discoverable without an additional button.
3. **Maximizing made visible:** The hint *Maximiert · 2 Bereiche ausgeblendet · Alle zeigen* (Maximized · 2 panes hidden · Show all) and "1/3" on the tab.
4. **Undo** for closed panes and tabs via `AdwToast` (Ghostty pattern). Still open is
   how long the process keeps running meanwhile (proposal: until the toast disappears).
5. **The start command is typed into the shell** (`feed_child`) instead of replacing the shell: after
   Ctrl+C the pane remains usable as a shell, and the command is in the history. Alternative:
   start the command directly, the pane stays when it ends. Both are technically simple.
6. **Never run start commands silently:** A toggle in the open dialog, default "on" only for your own
   templates; imported templates show their commands before the first run.

### Design question pane title (decided: floating)

Comparable in the mockup via the toggle *Titel schwebend / Titel als Zeile* (Floating title / Title as row) (only applies to split tabs).

| | Floating | As a row |
|---|---|---|
| Terminal area 1280×800 (measured) | 89.0 % · 89×41, 70×20, 70×20 | 84.0 % · 89×39, 70×18, 70×19 |
| Focus | accent line + dimming | colored row (clearer) |
| Dragging | by the small title | by the whole row (easier to hit) |
| Information | process, host | process, host, directory, status dot |
| Risk | covers terminal content at the top right (e.g. the htop clock) | repeats info from the tab/header bar |

**Conclusion:** Both are equally expensive to implement; the other variant can be offered as a setting.
What has to be decided is the default.

### Approval of A+ (2026-10-18)

Christoph approved A+: **floating pane title** as the default ("as a row" as a
setting), **proposals 1–6 adopted**. Continue with phase 3, see `docs/PROTOTYPE.md`.
