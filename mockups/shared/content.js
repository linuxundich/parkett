// Gemeinsamer Beispielinhalt und Hilfsfunktionen für alle Konzepte.
// Jedes Konzept zeigt dieselben Sitzungen, dieselben Terminals und dasselbe Split-Layout,
// damit sich die Entwürfe direkt vergleichen lassen.

const P = (s) => s; // Lesbarkeit: Zeilen bleiben HTML-Strings mit Farbklassen f0–f14

const prompt = (dir, branch) =>
  `<span class="f2 b">toff@smokki</span> <span class="f4 b">${dir}</span>` +
  (branch ? ` <span class="f5">(${branch})</span>` : "") + " $ ";
const promptBender = `<span class="f6 b">toff@bender</span>:<span class="f4 b">~</span>$ `;
const cur = (f) => `<span class="cur"> </span>`;

export const TERMS = {
  git: {
    title: "zsh", cwd: "~/05_Projekte/web/linuxundich/theme", state: "idle", profile: "Standard",
    lines: [
      prompt("theme", "redesign-c2a") + "git status -sb",
      `<span class="f2">## redesign-c2a</span>`,
      ` <span class="f1">M</span> assets/css/main.css`,
      ` <span class="f1">M</span> templates/single.php`,
      `<span class="f1">??</span> assets/img/hero-dark.webp`,
      prompt("theme", "redesign-c2a") + "git log --oneline -7",
      `<span class="f11">a41c9e2</span> <span class="f6">(HEAD -&gt; redesign-c2a)</span> Kopfzeile: Suche als Popover`,
      `<span class="f11">7d03b11</span> Kartenraster für Archivseiten`,
      `<span class="f11">c19e0aa</span> Dunkles Farbschema: Kontraste nach WCAG AA`,
      `<span class="f11">58f2d7e</span> Fußzeile neu gesetzt`,
      `<span class="f11">e0b6c41</span> Schriften lokal einbinden (Adwaita Sans)`,
      `<span class="f11">9a3c2f0</span> Lesezeit im Artikelkopf`,
      `<span class="f11">4410bd8</span> <span class="f6">(tag: 2.0.0)</span> Release 2.0.0`,
      prompt("theme", "redesign-c2a") + "rg -n 'hero' templates/ | head -4",
      `<span class="f5">templates/single.php</span>:<span class="f2">12</span>:  &lt;figure class="<span class="f1 b">hero</span>"&gt;`,
      `<span class="f5">templates/single.php</span>:<span class="f2">18</span>:  &lt;img class="<span class="f1 b">hero</span>__img" src="…"&gt;`,
      `<span class="f5">templates/archive.php</span>:<span class="f2">9</span>:   &lt;?php get_template_part('<span class="f1 b">hero</span>'); ?&gt;`,
      `<span class="f5">templates/parts/hero.php</span>:<span class="f2">1</span>: &lt;?php // <span class="f1 b">Hero</span>-Bereich`,
      prompt("theme", "redesign-c2a") + cur(),
    ],
  },
  dev: {
    title: "npm run dev", cwd: "~/05_Projekte/web/linuxundich/theme", state: "run", profile: "Standard",
    lines: [
      prompt("theme", "redesign-c2a") + "npm run dev",
      "",
      "&gt; lui-theme@2.1.0 dev",
      "&gt; vite --host 127.0.0.1",
      "",
      `  <span class="f2 b">VITE</span> <span class="f2">v7.1.4</span>  ready in <span class="b">412</span> ms`,
      "",
      `  <span class="f2">➜</span>  <span class="b">Local</span>:   <a class="link f6">http://127.0.0.1:5173/</a>`,
      `  <span class="f2">➜</span>  <span class="f8">press <span class="b">h + enter</span> to show help</span>`,
      `<span class="f8">14:02:11</span> <span class="f6 b">[vite]</span> <span class="f2">hmr update</span> <span class="f8">/assets/css/main.css</span>`,
      `<span class="f8">14:02:38</span> <span class="f6 b">[vite]</span> <span class="f2">hmr update</span> <span class="f8">/assets/css/main.css</span>`,
      `<span class="f8">14:03:05</span> <span class="f6 b">[vite]</span> <span class="f2">page reload</span> <span class="f8">templates/single.php</span>`,
      cur(),
    ],
  },
  bender: {
    title: "ssh bender", cwd: "bender:~", state: "idle", profile: "Server (bender)", remote: true,
    lines: [
      promptBender + "docker ps --format 'table {{.Names}}\\t{{.Status}}'",
      "NAMES        STATUS",
      `npm          Up 3 days <span class="f2">(healthy)</span>`,
      `authelia     Up 3 days <span class="f2">(healthy)</span>`,
      "homarr       Up 2 days",
      "gatus        Up 2 days",
      `crowdsec     Up 2 days <span class="f2">(healthy)</span>`,
      promptBender + cur(),
    ],
  },
  journal: {
    title: "journalctl -f", cwd: "bender:~", state: "act", profile: "Server (bender)", remote: true,
    lines: [
      promptBender + "journalctl -fu borgmatic -u backup-health",
      `<span class="f8">Okt 18 03:00:01</span> bender systemd[1]: Starting borgmatic.service…`,
      `<span class="f8">Okt 18 03:00:04</span> bender borgmatic[90211]: local: Creating archive`,
      `<span class="f8">Okt 18 03:41:57</span> bender borgmatic[90211]: local: Pruning archives`,
      `<span class="f8">Okt 18 03:42:10</span> bender borgmatic[90211]: offsite: Creating archive`,
      `<span class="f8">Okt 18 04:26:33</span> bender systemd[1]: borgmatic.service: <span class="f2">Deactivated successfully.</span>`,
      `<span class="f8">Okt 18 06:00:00</span> bender backup-health[1201]: <span class="f2">OK</span> – letzte Sicherung 1 h 34 min alt`,
      cur(),
    ],
  },
  test: {
    title: "cargo test", cwd: "~/05_Projekte/apps/blocksatz", state: "err", exit: 101, profile: "Standard",
    lines: [
      prompt("blocksatz", "main") + "cargo test -q",
      "",
      "running 214 tests",
      `<span class="f2">.......................................................</span> 55/214`,
      `<span class="f2">...............................</span><span class="f1 b">F</span><span class="f2">.......................</span> 110/214`,
      "",
      `failures:`,
      `---- import::tables::colspan_roundtrip stdout ----`,
      `thread 'import::tables::colspan_roundtrip' panicked at src/import/tables.rs:188:9:`,
      `assertion \`left == right\` failed`,
      "",
      `test result: <span class="f1 b">FAILED</span>. 213 passed; 1 failed; 0 ignored`,
      `<span class="f1 b">error</span>: test failed, to rerun pass \`--lib\``,
    ],
  },
  build: {
    title: "cargo build", cwd: "~/05_Projekte/apps/blocksatz", state: "idle", profile: "Standard",
    lines: [
      prompt("blocksatz", "main") + "cargo build --release",
      `   <span class="f2 b">Compiling</span> blocksatz v0.75.0 (/home/toff/05_Projekte/apps/blocksatz)`,
      `    <span class="f2 b">Finished</span> \`release\` profile [optimized] target(s) in 48.21s`,
      prompt("blocksatz", "main") + cur(),
    ],
  },
  home: {
    title: "zsh", cwd: "~", state: "idle", profile: "Standard",
    lines: [prompt("~") + "uptime", " 14:03:41 up 2 days,  3:12,  1 user,  load average: 0,42, 0,51, 0,47", prompt("~") + cur()],
  },
  kulisse: {
    title: "cargo build", cwd: "~/05_Projekte/apps/kulisse", state: "run", profile: "Standard",
    lines: [
      prompt("kulisse", "main") + "cargo build",
      `   <span class="f2 b">Compiling</span> prospekt v0.4.1`,
      `   <span class="f2 b">Compiling</span> kulisse v0.6.1 (/home/toff/05_Projekte/apps/kulisse)`,
      `    <span class="f6 b">Building</span> [=======================&gt;   ] 412/437: kulisse`,
    ],
  },
};

// Split-Layout als Baum: h = nebeneinander, v = übereinander, r = Anteil des ersten Kindes.
export const SESSIONS = [
  { id: "theme", name: "lui-theme", icon: "folder-projects", workspace: true,
    layout: { d: "h", r: 0.56, a: "git", b: { d: "v", r: 0.5, a: "dev", b: "bender" } }, focus: "git" },
  { id: "bender", name: "bender", icon: "network-server", workspace: true, remote: true,
    layout: "journal", focus: "journal", attention: true },
  { id: "blocksatz", name: "blocksatz", icon: "folder-projects", workspace: true,
    layout: { d: "v", r: 0.55, a: "test", b: "build" }, focus: "test" },
  { id: "kulisse", name: "kulisse", icon: "folder-projects", workspace: true,
    layout: "kulisse", focus: "kulisse" },
  { id: "home", name: "~", icon: "user-home", layout: "home", focus: "home" },
];

export const sessionTerms = (layout) =>
  typeof layout === "string" ? [layout] : [...sessionTerms(layout.a), ...sessionTerms(layout.b)];

export const sessionState = (s) => {
  const st = sessionTerms(s.layout).map((t) => TERMS[t].state);
  if (st.includes("err")) return "err";
  if (st.includes("run")) return "run";
  if (st.includes("act")) return "act";
  return "idle";
};

export const STATE_LABEL = { idle: "Bereit", run: "Läuft", act: "Neue Ausgabe", err: "Beendet mit Fehler" };

// Mini-Schema eines Layouts als CSS-Grid (für Seitenleisten, Tab-Übersicht, Palette)
export function glyph(layout, focus) {
  const cells = [];
  const walk = (n, x, y, w, h) => {
    if (typeof n === "string") return cells.push({ n, x, y, w, h });
    if (n.d === "h") { walk(n.a, x, y, w * n.r, h); walk(n.b, x + w * n.r, y, w * (1 - n.r), h); }
    else { walk(n.a, x, y, w, h * n.r); walk(n.b, x, y + h * n.r, w, h * (1 - n.r)); }
  };
  walk(layout, 0, 0, 12, 12);
  const g = document.createElement("span");
  g.className = "glyph";
  g.style.gridTemplateColumns = "repeat(12, 1fr)";
  g.style.gridTemplateRows = "repeat(12, 1fr)";
  for (const c of cells) {
    const i = document.createElement("i");
    i.style.gridColumn = `${Math.round(c.x) + 1} / span ${Math.max(1, Math.round(c.w))}`;
    i.style.gridRow = `${Math.round(c.y) + 1} / span ${Math.max(1, Math.round(c.h))}`;
    if (c.n === focus) i.className = "f";
    g.append(i);
  }
  return g;
}

export const el = (tag, cls, html) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (html != null) e.innerHTML = html;
  return e;
};
export const icon = (name, extra = "") => `<span class="icon i-${name} ${extra}"></span>`;

export function termEl(id, { focused = false, dimUnfocused = true } = {}) {
  const t = TERMS[id];
  const pane = el("div", "pane" + (focused ? " focused" : "") + (!focused && dimUnfocused ? " dimmed" : ""));
  pane.dataset.term = id;
  const term = el("div", "term", t.lines.join("\n"));
  pane.append(term);
  return pane;
}

// Baut den Split-Baum. chrome(pane, id) darf die Fläche dekorieren (Titelzeile, Overlay …).
export function renderLayout(layout, { focus, chrome = (p) => p, dimUnfocused = true } = {}) {
  if (typeof layout === "string") {
    return chrome(termEl(layout, { focused: layout === focus, dimUnfocused }), layout);
  }
  const box = el("div", `split ${layout.d}`);
  box.style.flex = "1";
  const a = renderLayout(layout.a, { focus, chrome, dimUnfocused });
  const b = renderLayout(layout.b, { focus, chrome, dimUnfocused });
  a.style.flex = `${layout.r} 1 0`;
  b.style.flex = `${1 - layout.r} 1 0`;
  box.append(a, el("div", "sep"), b);
  return box;
}

// ---------- Bühne: Ansichten, Hell/Dunkel, Hinweise, Messung ----------

let cellW = 0;
function measureCell() {
  const s = el("span", "", "M".repeat(100));
  Object.assign(s.style, { fontFamily: "var(--mono)", fontSize: "13px", position: "absolute", visibility: "hidden", whiteSpace: "pre" });
  document.body.append(s);
  cellW = s.getBoundingClientRect().width / 100;
  s.remove();
}

export function measure(win) {
  if (!cellW) measureCell();
  const scale = win.getBoundingClientRect().width / win.offsetWidth;
  if (!scale) return { pct: 0, dims: [] };
  const wa = win.offsetWidth * win.offsetHeight;
  let ta = 0;
  const dims = [];
  for (const t of win.querySelectorAll(".pane:not(.thumb) > .term")) {
    if (t.closest(".dialog-backdrop, .no-measure")) continue;
    const r = t.getBoundingClientRect();
    const w = r.width / scale, h = r.height / scale;
    if (w < 40 || h < 30) continue;
    ta += w * h;
    dims.push(`${Math.floor((w - 16) / cellW)}×${Math.floor((h - 12) / 17)}`);
  }
  return { pct: (100 * ta) / wa, dims };
}

export function stage({ concept, views, defaultView = "main" }) {
  const params = new URLSearchParams(location.hash.slice(1));
  let view = params.get("view") || defaultView;
  let theme = params.get("theme") || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
  let hints = params.get("hints") !== "0";

  const bar = document.querySelector(".stage-bar");
  const host = document.querySelector(".scaler");
  const caption = document.querySelector(".caption");
  const meter = bar.querySelector(".meter");

  const vseg = el("div", "seg");
  for (const v of views) {
    const b = el("button", "", v.label);
    b.onclick = () => { view = v.id; draw(); };
    b.dataset.v = v.id;
    vseg.append(b);
  }
  const tseg = el("div", "seg");
  for (const [k, l] of [["light", "Hell"], ["dark", "Dunkel"]]) {
    const b = el("button", "", l);
    b.dataset.t = k;
    b.onclick = () => { theme = k; draw(); };
    tseg.append(b);
  }
  const hseg = el("div", "seg");
  const hb = el("button", "", "Erläuterungen");
  hb.onclick = () => { hints = !hints; draw(); };
  hseg.append(hb);
  meter.before(vseg, tseg, hseg);

  function fit() {
    const win = host.querySelector(".window");
    if (!win) return;
    const s = Math.min(1, host.clientWidth / win.offsetWidth);
    win.style.transform = `scale(${s})`;
    host.style.height = win.offsetHeight * s + "px";
  }

  function draw() {
    document.documentElement.dataset.theme = theme;
    document.body.classList.toggle("no-hints", !hints);
    for (const b of vseg.children) b.setAttribute("aria-pressed", b.dataset.v === view);
    for (const b of tseg.children) b.setAttribute("aria-pressed", b.dataset.t === theme);
    hb.setAttribute("aria-pressed", hints);
    const v = views.find((x) => x.id === view) || views[0];
    host.innerHTML = "";
    const win = el("div", "window" + (v.small ? " small" : ""));
    host.append(win);
    v.render(win);
    caption.innerHTML = v.caption || "";
    history.replaceState(null, "", `#view=${v.id}&theme=${theme}${hints ? "" : "&hints=0"}`);
    requestAnimationFrame(() => {
      if (!win.isConnected) return;
      fit();
      const m = measure(win);
      meter.textContent = (v.id === "main" || v.small)
        ? `${concept} · Fenster ${win.offsetWidth}×${win.offsetHeight} px · Terminalfläche ${m.pct.toFixed(1)} % · Bereiche (Spalten×Zeilen bei 13 px): ${m.dims.join(", ") || "–"}`
        : `${concept} · Fenster ${win.offsetWidth}×${win.offsetHeight} px · Flächenmessung nur in „Hauptfenster“ und „Kleines Fenster“`;
    });
  }
  addEventListener("resize", fit);
  addEventListener("hashchange", () => {
    const q = new URLSearchParams(location.hash.slice(1));
    view = q.get("view") || view; theme = q.get("theme") || theme; hints = q.get("hints") !== "0";
    draw();
  });
  document.fonts.ready.then(() => { cellW = 0; draw(); });
  draw();
}

// Nummerierte Erläuterung im Mockup (per Schalter ausblendbar)
export function hint(win, n, text, style) {
  const h = el("div", "hint-callout", text);
  h.dataset.n = n;
  Object.assign(h.style, style);
  win.append(h);
}
