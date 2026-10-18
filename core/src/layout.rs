//! Split-Layout eines Tabs als unveränderlicher Binärbaum.
//!
//! Jede Operation liefert ein neues `Layout`; der alte Stand bleibt unverändert und kann
//! für Rückgängig aufbewahrt werden (Muster: Ghostty `split_tree.zig`). Die GTK-Seite baut
//! ihre `GtkPaned`-Bäume aus diesem Modell und liest nie umgekehrt aus den Widgets.

pub type PaneId = u32;

/// `Horizontal` = Bereiche nebeneinander (Trenner senkrecht), `Vertical` = übereinander.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn axis(self) -> Orientation {
        match self {
            Direction::Left | Direction::Right => Orientation::Horizontal,
            Direction::Up | Direction::Down => Orientation::Vertical,
        }
    }
}

/// Weg vom Wurzelknoten zu einem Split: je Ebene erstes oder zweites Kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    First,
    Second,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Pane(PaneId),
    Split {
        orientation: Orientation,
        /// Anteil des ersten Kindes, 0.0–1.0
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Kleinster erlaubter Anteil beim Verschieben eines Trenners.
pub const MIN_RATIO: f32 = 0.1;

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    root: Node,
    focus: PaneId,
    zoomed: Option<PaneId>,
    next_id: PaneId,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    /// Ein Tab mit einem einzigen Bereich (Id 1).
    pub fn new() -> Self {
        Layout { root: Node::Pane(1), focus: 1, zoomed: None, next_id: 2 }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    pub fn focus(&self) -> PaneId {
        self.focus
    }

    pub fn zoomed(&self) -> Option<PaneId> {
        self.zoomed
    }

    /// Alle Bereiche in Lesereihenfolge (links vor rechts, oben vor unten).
    pub fn panes(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        collect(&self.root, &mut out);
        out
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.panes().contains(&id)
    }

    pub fn with_focus(&self, id: PaneId) -> Option<Layout> {
        self.contains(id).then(|| Layout { focus: id, ..self.clone() })
    }

    /// Teilt `target`; der neue Bereich liegt rechts bzw. unten und bekommt den Fokus.
    pub fn split(&self, target: PaneId, orientation: Orientation) -> Option<(Layout, PaneId)> {
        let new_id = self.next_id;
        let root = replace(&self.root, target, &|_| Node::Split {
            orientation,
            ratio: 0.5,
            first: Box::new(Node::Pane(target)),
            second: Box::new(Node::Pane(new_id)),
        })?;
        Some((Layout { root, focus: new_id, zoomed: None, next_id: new_id + 1 }, new_id))
    }

    /// Wie Tilix „automatisch teilen“: entlang der längeren Seite des Bereichs.
    pub fn split_auto(&self, target: PaneId, width: f32, height: f32) -> Option<(Layout, PaneId)> {
        let r = self.rect_of(target, width, height)?;
        // Terminalzellen sind etwa doppelt so hoch wie breit, daher Breite halbiert vergleichen
        let o = if r.w / 2.0 >= r.h { Orientation::Horizontal } else { Orientation::Vertical };
        self.split(target, o)
    }

    /// Schließt `target`; das Geschwister rückt an seine Stelle. `None`, wenn es der letzte
    /// Bereich ist (dann schließt der Tab, nicht das Layout).
    pub fn close(&self, target: PaneId) -> Option<Layout> {
        if !self.contains(target) || matches!(self.root, Node::Pane(_)) {
            return None;
        }
        let (root, neighbour) = remove(&self.root, target)?;
        let focus = if self.focus == target { neighbour } else { self.focus };
        let zoomed = self.zoomed.filter(|z| *z != target);
        Some(Layout { root, focus, zoomed, next_id: self.next_id })
    }

    /// Verschiebt den nächstgelegenen passenden Trenner um `delta` (Anteil) in Richtung `dir`.
    pub fn resize(&self, target: PaneId, dir: Direction, delta: f32) -> Option<Layout> {
        let signed = match dir {
            Direction::Right | Direction::Down => delta,
            Direction::Left | Direction::Up => -delta,
        };
        let root = resize_node(&self.root, target, dir.axis(), signed)?.0;
        Some(Layout { root, ..self.clone() })
    }

    /// Setzt alle Anteile so, dass gleich ausgerichtete Bereiche gleich groß werden.
    pub fn equalize(&self) -> Layout {
        Layout { root: equalize_node(&self.root), ..self.clone() }
    }

    /// Setzt den Anteil des Splits am Pfad `path` (z. B. nach Ziehen des Trenners).
    /// `None`, wenn am Pfad kein Split liegt.
    pub fn set_ratio(&self, path: &[Side], ratio: f32) -> Option<Layout> {
        let root = set_ratio_node(&self.root, path, ratio.clamp(MIN_RATIO, 1.0 - MIN_RATIO))?;
        Some(Layout { root, ..self.clone() })
    }

    /// Anteil des Splits am Pfad.
    pub fn ratio_at(&self, path: &[Side]) -> Option<f32> {
        let mut n = &self.root;
        for side in path {
            match n {
                Node::Split { first, second, .. } => n = if *side == Side::First { first } else { second },
                Node::Pane(_) => return None,
            }
        }
        match n {
            Node::Split { ratio, .. } => Some(*ratio),
            Node::Pane(_) => None,
        }
    }

    /// Hebt einen Zoom auf (alle Bereiche wieder sichtbar).
    pub fn without_zoom(&self) -> Layout {
        Layout { zoomed: None, ..self.clone() }
    }

    pub fn toggle_zoom(&self, target: PaneId) -> Option<Layout> {
        if !self.contains(target) {
            return None;
        }
        let zoomed = if self.zoomed == Some(target) { None } else { Some(target) };
        Some(Layout { zoomed, focus: target, ..self.clone() })
    }

    /// Rechtecke aller Bereiche bei gegebener Fläche (Trennerbreite vernachlässigt).
    pub fn geometry(&self, width: f32, height: f32) -> Vec<(PaneId, Rect)> {
        let mut out = Vec::new();
        geom(&self.root, Rect { x: 0.0, y: 0.0, w: width, h: height }, &mut out);
        out
    }

    pub fn rect_of(&self, id: PaneId, width: f32, height: f32) -> Option<Rect> {
        self.geometry(width, height).into_iter().find(|(p, _)| *p == id).map(|(_, r)| r)
    }

    /// Darf `target` geteilt werden, ohne dass eine Hälfte kleiner als `min_w`×`min_h` wird?
    pub fn can_split(&self, target: PaneId, o: Orientation, width: f32, height: f32, min_w: f32, min_h: f32) -> bool {
        match self.rect_of(target, width, height) {
            Some(r) => match o {
                Orientation::Horizontal => r.w / 2.0 >= min_w && r.h >= min_h,
                Orientation::Vertical => r.h / 2.0 >= min_h && r.w >= min_w,
            },
            None => false,
        }
    }

    /// Nachbar in Richtung `dir`, geometrisch bestimmt: der nächste Bereich, der sich
    /// senkrecht zur Richtung mit dem aktuellen überlappt; bei Gleichstand der mit der
    /// größten Überlappung.
    pub fn neighbour(&self, from: PaneId, dir: Direction, width: f32, height: f32) -> Option<PaneId> {
        let rects = self.geometry(width, height);
        let me = rects.iter().find(|(p, _)| *p == from)?.1;
        const EPS: f32 = 0.5;
        rects
            .iter()
            .filter(|(p, _)| *p != from)
            .filter_map(|(p, r)| {
                let (dist, overlap) = match dir {
                    Direction::Right => (r.x - (me.x + me.w), span(me.y, me.h, r.y, r.h)),
                    Direction::Left => (me.x - (r.x + r.w), span(me.y, me.h, r.y, r.h)),
                    Direction::Down => (r.y - (me.y + me.h), span(me.x, me.w, r.x, r.w)),
                    Direction::Up => (me.y - (r.y + r.h), span(me.x, me.w, r.x, r.w)),
                };
                (dist > -EPS && overlap > EPS).then_some((*p, dist, overlap))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(b.2.total_cmp(&a.2)))
            .map(|(p, _, _)| p)
    }
}

fn span(a: f32, al: f32, b: f32, bl: f32) -> f32 {
    (a + al).min(b + bl) - a.max(b)
}

fn collect(n: &Node, out: &mut Vec<PaneId>) {
    match n {
        Node::Pane(id) => out.push(*id),
        Node::Split { first, second, .. } => {
            collect(first, out);
            collect(second, out);
        }
    }
}

fn first_pane(n: &Node) -> PaneId {
    match n {
        Node::Pane(id) => *id,
        Node::Split { first, .. } => first_pane(first),
    }
}

fn replace(n: &Node, target: PaneId, f: &dyn Fn(&Node) -> Node) -> Option<Node> {
    match n {
        Node::Pane(id) if *id == target => Some(f(n)),
        Node::Pane(_) => None,
        Node::Split { orientation, ratio, first, second } => {
            if let Some(a) = replace(first, target, f) {
                Some(Node::Split { orientation: *orientation, ratio: *ratio, first: Box::new(a), second: second.clone() })
            } else {
                replace(second, target, f).map(|b| Node::Split {
                    orientation: *orientation,
                    ratio: *ratio,
                    first: first.clone(),
                    second: Box::new(b),
                })
            }
        }
    }
}

/// Entfernt `target`, gibt neuen Baum und den Bereich zurück, der den Fokus erben soll.
fn remove(n: &Node, target: PaneId) -> Option<(Node, PaneId)> {
    let Node::Split { orientation, ratio, first, second } = n else { return None };
    if **first == Node::Pane(target) {
        return Some(((**second).clone(), first_pane(second)));
    }
    if **second == Node::Pane(target) {
        return Some(((**first).clone(), first_pane(first)));
    }
    if let Some((a, f)) = remove(first, target) {
        return Some((Node::Split { orientation: *orientation, ratio: *ratio, first: Box::new(a), second: second.clone() }, f));
    }
    remove(second, target).map(|(b, f)| {
        (Node::Split { orientation: *orientation, ratio: *ratio, first: first.clone(), second: Box::new(b) }, f)
    })
}

/// Rückgabe: (neuer Knoten, ob die Änderung schon angewendet wurde)
fn resize_node(n: &Node, target: PaneId, axis: Orientation, signed: f32) -> Option<(Node, bool)> {
    match n {
        Node::Pane(id) => (*id == target).then(|| (n.clone(), false)),
        Node::Split { orientation, ratio, first, second } => {
            let (in_first, (child, done)) = if let Some(r) = resize_node(first, target, axis, signed) {
                (true, r)
            } else {
                (false, resize_node(second, target, axis, signed)?)
            };
            let (new_ratio, done) = if !done && *orientation == axis {
                ((ratio + signed).clamp(MIN_RATIO, 1.0 - MIN_RATIO), true)
            } else {
                (*ratio, done)
            };
            let (a, b) = if in_first { (Box::new(child), second.clone()) } else { (first.clone(), Box::new(child)) };
            Some((Node::Split { orientation: *orientation, ratio: new_ratio, first: a, second: b }, done))
        }
    }
}

fn set_ratio_node(n: &Node, path: &[Side], ratio: f32) -> Option<Node> {
    let Node::Split { orientation, ratio: old, first, second } = n else { return None };
    match path.split_first() {
        None => Some(Node::Split { orientation: *orientation, ratio, first: first.clone(), second: second.clone() }),
        Some((Side::First, rest)) => Some(Node::Split {
            orientation: *orientation,
            ratio: *old,
            first: Box::new(set_ratio_node(first, rest, ratio)?),
            second: second.clone(),
        }),
        Some((Side::Second, rest)) => Some(Node::Split {
            orientation: *orientation,
            ratio: *old,
            first: first.clone(),
            second: Box::new(set_ratio_node(second, rest, ratio)?),
        }),
    }
}

fn weight(n: &Node, axis: Orientation) -> f32 {
    match n {
        Node::Split { orientation, first, second, .. } if *orientation == axis => weight(first, axis) + weight(second, axis),
        _ => 1.0,
    }
}

fn equalize_node(n: &Node) -> Node {
    match n {
        Node::Pane(_) => n.clone(),
        Node::Split { orientation, first, second, .. } => {
            let a = weight(first, *orientation);
            let b = weight(second, *orientation);
            Node::Split {
                orientation: *orientation,
                ratio: a / (a + b),
                first: Box::new(equalize_node(first)),
                second: Box::new(equalize_node(second)),
            }
        }
    }
}

fn geom(n: &Node, r: Rect, out: &mut Vec<(PaneId, Rect)>) {
    match n {
        Node::Pane(id) => out.push((*id, r)),
        Node::Split { orientation, ratio, first, second } => match orientation {
            Orientation::Horizontal => {
                let w1 = r.w * ratio;
                geom(first, Rect { w: w1, ..r }, out);
                geom(second, Rect { x: r.x + w1, w: r.w - w1, ..r }, out);
            }
            Orientation::Vertical => {
                let h1 = r.h * ratio;
                geom(first, Rect { h: h1, ..r }, out);
                geom(second, Rect { y: r.y + h1, h: r.h - h1, ..r }, out);
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::*;
    use Orientation::*;

    /// Das Beispiel-Layout der Mockups: links 1, rechts oben 2, rechts unten 3.
    fn three() -> Layout {
        let (l, two) = Layout::new().split(1, Horizontal).unwrap();
        let (l, three) = l.split(two, Vertical).unwrap();
        assert_eq!((two, three), (2, 3));
        l
    }

    #[test]
    fn split_places_new_pane_after_and_focuses_it() {
        let (l, id) = Layout::new().split(1, Horizontal).unwrap();
        assert_eq!(l.panes(), vec![1, id]);
        assert_eq!(l.focus(), id);
        assert!(Layout::new().split(99, Horizontal).is_none());
    }

    #[test]
    fn split_is_immutable() {
        let a = Layout::new();
        let _ = a.split(1, Vertical).unwrap();
        assert_eq!(a, Layout::new());
    }

    #[test]
    fn close_lets_sibling_take_over() {
        let l = three();
        let c = l.close(2).unwrap();
        assert_eq!(c.panes(), vec![1, 3]);
        assert_eq!(c.focus(), 3, "Fokus lag auf 3 und bleibt dort");
        let c = l.with_focus(2).unwrap().close(2).unwrap();
        assert_eq!(c.focus(), 3, "Fokus geht an das Geschwister");
        let only = c.close(3).unwrap();
        assert_eq!(only.root(), &Node::Pane(1));
        assert!(only.close(1).is_none(), "letzter Bereich schließt den Tab, nicht das Layout");
    }

    #[test]
    fn ids_are_never_reused() {
        let l = three().close(3).unwrap();
        let (_, id) = l.split(2, Vertical).unwrap();
        assert_eq!(id, 4);
    }

    #[test]
    fn geometry_matches_ratios() {
        let g = three().geometry(1000.0, 600.0);
        assert_eq!(g[0], (1, Rect { x: 0.0, y: 0.0, w: 500.0, h: 600.0 }));
        assert_eq!(g[1], (2, Rect { x: 500.0, y: 0.0, w: 500.0, h: 300.0 }));
        assert_eq!(g[2], (3, Rect { x: 500.0, y: 300.0, w: 500.0, h: 300.0 }));
    }

    #[test]
    fn neighbours_follow_geometry() {
        let l = three();
        assert_eq!(l.neighbour(1, Right, 1000.0, 600.0), Some(2), "bei völligem Gleichstand gewinnt der erste in Lesereihenfolge");
        assert_eq!(l.neighbour(3, Left, 1000.0, 600.0), Some(1));
        assert_eq!(l.neighbour(2, Down, 1000.0, 600.0), Some(3));
        assert_eq!(l.neighbour(3, Up, 1000.0, 600.0), Some(2));
        assert_eq!(l.neighbour(1, Left, 1000.0, 600.0), None);
    }

    #[test]
    fn neighbour_prefers_larger_overlap() {
        // rechte Spalte 70/30 geteilt: von links nach rechts landet man oben (mehr Überlappung)
        let l = three().resize(2, Down, 0.2).unwrap();
        assert_eq!(l.neighbour(1, Right, 1000.0, 600.0), Some(2));
        let l = three().resize(2, Up, 0.2).unwrap();
        assert_eq!(l.neighbour(1, Right, 1000.0, 600.0), Some(3));
    }

    #[test]
    fn resize_moves_nearest_matching_divider_and_clamps() {
        let l = three();
        let r = l.resize(3, Left, 0.1).unwrap();
        let g = r.geometry(1000.0, 600.0);
        assert!((g[0].1.w - 400.0).abs() < 0.01, "senkrechter Trenner wandert nach links");
        let r = l.resize(1, Right, 5.0).unwrap();
        assert!((r.geometry(1000.0, 600.0)[0].1.w - 900.0).abs() < 0.01, "auf 90 % begrenzt");
        let same = l.resize(1, Down, 0.1).unwrap();
        assert_eq!(same.root(), l.root(), "kein waagrechter Trenner über Bereich 1");
    }

    #[test]
    fn equalize_spreads_same_orientation_chains() {
        // drei nebeneinander: 1 | 2 | 3
        let (l, two) = Layout::new().split(1, Horizontal).unwrap();
        let (l, _) = l.split(two, Horizontal).unwrap();
        let g = l.equalize().geometry(900.0, 100.0);
        for (_, r) in g {
            assert!((r.w - 300.0).abs() < 0.01, "{r:?}");
        }
    }

    #[test]
    fn auto_split_uses_longer_side() {
        let (l, _) = Layout::new().split_auto(1, 1600.0, 600.0).unwrap();
        assert!(matches!(l.root(), Node::Split { orientation: Horizontal, .. }));
        let (l, _) = Layout::new().split_auto(1, 800.0, 600.0).unwrap();
        assert!(matches!(l.root(), Node::Split { orientation: Vertical, .. }));
    }

    #[test]
    fn min_size_blocks_split() {
        let l = three();
        // rechte Bereiche 500×300; Mindestgröße 300 breit
        assert!(!l.can_split(2, Horizontal, 1000.0, 600.0, 300.0, 50.0));
        assert!(l.can_split(2, Vertical, 1000.0, 600.0, 300.0, 50.0));
        assert!(!l.can_split(42, Vertical, 1000.0, 600.0, 1.0, 1.0));
    }

    #[test]
    fn set_ratio_by_path() {
        let l = three();
        let r = l.set_ratio(&[], 0.7).unwrap();
        assert_eq!(r.ratio_at(&[]), Some(0.7));
        let r = r.set_ratio(&[Side::Second], 0.25).unwrap();
        assert_eq!(r.ratio_at(&[Side::Second]), Some(0.25));
        assert_eq!(r.ratio_at(&[]), Some(0.7), "andere Splits unverändert");
        assert!(l.set_ratio(&[Side::First], 0.3).is_none(), "links liegt ein Bereich, kein Split");
        assert_eq!(l.set_ratio(&[], 0.0).unwrap().ratio_at(&[]), Some(MIN_RATIO), "begrenzt");
    }

    #[test]
    fn zoom_toggles_and_survives_unrelated_close() {
        let l = three().toggle_zoom(1).unwrap();
        assert_eq!(l.zoomed(), Some(1));
        assert_eq!(l.close(2).unwrap().zoomed(), Some(1));
        assert_eq!(l.close(1).unwrap().zoomed(), None);
        assert_eq!(l.toggle_zoom(1).unwrap().zoomed(), None);
        assert_eq!(l.without_zoom().zoomed(), None);
        assert_eq!(l.split(1, Vertical).unwrap().0.zoomed(), None, "Teilen hebt Zoom auf");
    }
}
