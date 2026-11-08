//! chida-core: the document model every other Chida crate builds on.
//!
//! A [`Project`] owns pages, reusable components and a design theme.
//! Each [`Page`] owns a tree of [`Node`] values (the scene graph).
//! Projects are saved and loaded as JSON (the `.chida.json` format).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// An RGBA colour with helpers for the hex forms used in project files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa`.
    pub fn from_hex(s: &str) -> Result<Self, CoreError> {
        let h = s
            .strip_prefix('#')
            .ok_or_else(|| CoreError::BadColor(s.to_string()))?;
        let v = |i: usize| {
            u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| CoreError::BadColor(s.to_string()))
        };
        match h.len() {
            3 => {
                let f = |i: usize| {
                    let n = u8::from_str_radix(&h[i..i + 1], 16)
                        .map_err(|_| CoreError::BadColor(s.to_string()));
                    n.map(|n| n * 17)
                };
                Ok(Self::rgb(f(0)?, f(1)?, f(2)?))
            }
            6 => Ok(Self::rgb(v(0)?, v(2)?, v(4)?)),
            8 => Ok(Self::rgba(v(0)?, v(2)?, v(4)?, v(6)?)),
            _ => Err(CoreError::BadColor(s.to_string())),
        }
    }
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
    /// Linear blend of `self` over `bg` using the alpha channel.
    pub fn over(&self, bg: Color) -> Color {
        let a = self.a as f32 / 255.0;
        let mix = |f: u8, b: u8| (f as f32 * a + b as f32 * (1.0 - a)).round() as u8;
        Color::rgb(mix(self.r, bg.r), mix(self.g, bg.g), mix(self.b, bg.b))
    }
}

/// A two stop linear gradient fill. `angle` is in degrees, CSS style:
/// 0 points up, 90 points right, 180 points down.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub from: Color,
    pub to: Color,
    #[serde(default = "default_gradient_angle")]
    pub angle: f32,
}
fn default_gradient_angle() -> f32 {
    180.0
}
impl Gradient {
    pub fn new(from: Color, to: Color, angle: f32) -> Self {
        Self { from, to, angle }
    }
    /// The colour at position `t` (0 = from, 1 = to) along the gradient.
    pub fn at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color::rgba(
            mix(self.from.r, self.to.r),
            mix(self.from.g, self.to.g),
            mix(self.from.b, self.to.b),
            mix(self.from.a, self.to.a),
        )
    }
    /// Unit direction of the gradient axis for an angle in degrees.
    pub fn direction(&self) -> (f32, f32) {
        let rad = self.angle.to_radians();
        (rad.sin(), -rad.cos())
    }
}

/// A drop shadow painted behind a node. Presence of the value means
/// the shadow is on; there is no separate enabled flag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub color: Color,
    #[serde(default = "default_shadow_opacity")]
    pub opacity: f32,
}
fn default_shadow_opacity() -> f32 {
    0.25
}
impl Shadow {
    pub fn soft() -> Self {
        Self {
            x: 0.0,
            y: 8.0,
            blur: 24.0,
            color: Color::BLACK,
            opacity: 0.18,
        }
    }
}

/// Auto-layout direction for a container node. `None` keeps the
/// free-form absolute positioning Chida started with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDirection {
    #[default]
    None,
    Row,
    Column,
}

/// How children of an auto-layout container align on the cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlign {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// The kind of a scene node. New kinds stay serialisation friendly strings
/// in spirit: each variant maps to one builder block in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Frame,
    Rectangle,
    Ellipse,
    Text,
    Button,
    Card,
    Image,
    Input,
    Divider,
    Icon,
    List,
    Table,
    Chart,
    Custom,
}

/// One element on a page. Children are painted in order, later on top.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub fill: Option<Color>,
    /// Gradient fill. When set, it wins over the solid `fill` in every
    /// renderer (PNG, SVG and the editor canvas).
    #[serde(default)]
    pub fill_gradient: Option<Gradient>,
    /// Drop shadow painted behind the node. `None` means no shadow.
    #[serde(default)]
    pub shadow: Option<Shadow>,
    #[serde(default)]
    pub stroke: Option<Color>,
    #[serde(default)]
    pub stroke_width: f32,
    #[serde(default)]
    pub corner_radius: f32,
    #[serde(default)]
    pub opacity: f32,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub font_size: f32,
    #[serde(default)]
    pub text_color: Option<Color>,
    #[serde(default)]
    pub bold: bool,
    /// Numeric font weight: 400 normal, 500 medium, 700 bold. Zero in
    /// old files means unset; the effective weight then follows `bold`.
    #[serde(default)]
    pub font_weight: u16,
    /// Line height multiplier. Zero means automatic (the renderer
    /// default of 1.45).
    #[serde(default)]
    pub line_height: f32,
    /// Extra spacing between characters, in pixels.
    #[serde(default)]
    pub letter_spacing: f32,
    /// Name of a component this node instantiates, when it is an instance.
    #[serde(default)]
    pub component: Option<String>,
    /// Auto-layout for container nodes (frame, card, group kinds).
    /// When the direction is not `none`, [`Node::apply_layout`]
    /// recomputes the children's positions from these settings and
    /// manual positions of the children are ignored.
    #[serde(default)]
    pub layout_direction: LayoutDirection,
    #[serde(default)]
    pub layout_gap: f32,
    #[serde(default)]
    pub layout_padding: f32,
    #[serde(default)]
    pub layout_align: LayoutAlign,
    /// Free form properties set by blocks (placeholder, src, route, ...).
    #[serde(default)]
    pub props: BTreeMap<String, String>,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default = "default_visible")]
    pub visible: bool,
}
fn default_visible() -> bool {
    true
}
fn default_opacity() -> f32 {
    1.0
}

impl Node {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: NodeKind,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            x,
            y,
            width,
            height,
            fill: None,
            fill_gradient: None,
            shadow: None,
            stroke: None,
            stroke_width: 0.0,
            corner_radius: 0.0,
            opacity: default_opacity(),
            text: None,
            font_size: 16.0,
            text_color: None,
            bold: false,
            font_weight: 0,
            line_height: 0.0,
            letter_spacing: 0.0,
            component: None,
            layout_direction: LayoutDirection::None,
            layout_gap: 0.0,
            layout_padding: 0.0,
            layout_align: LayoutAlign::Start,
            props: BTreeMap::new(),
            children: Vec::new(),
            visible: true,
        }
    }
    pub fn rect(
        id: impl Into<String>,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        fill: Color,
    ) -> Self {
        let mut n = Self::new(id, name, NodeKind::Rectangle, x, y, w, h);
        n.fill = Some(fill);
        n
    }
    pub fn text_node(
        id: impl Into<String>,
        name: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: impl Into<String>,
        size: f32,
        color: Color,
    ) -> Self {
        let mut n = Self::new(id, name, NodeKind::Text, x, y, w, h);
        n.text = Some(text.into());
        n.font_size = size;
        n.text_color = Some(color);
        n
    }
    pub fn with_radius(mut self, r: f32) -> Self {
        self.corner_radius = r;
        self
    }
    pub fn with_gradient(mut self, gradient: Gradient) -> Self {
        self.fill_gradient = Some(gradient);
        self
    }
    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
    /// The weight renderers should use: an explicit `font_weight`
    /// wins, otherwise the legacy `bold` flag decides.
    pub fn effective_weight(&self) -> u16 {
        if self.font_weight > 0 {
            self.font_weight
        } else if self.bold {
            700
        } else {
            400
        }
    }
    pub fn effective_bold(&self) -> bool {
        self.effective_weight() >= 600
    }
    pub fn with_child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }
    /// True when this container arranges its children automatically.
    pub fn has_layout(&self) -> bool {
        self.layout_direction != LayoutDirection::None
    }
    /// Recompute the positions of this node's children from its
    /// auto-layout settings, then recurse into every child so nested
    /// containers lay out with their final sizes (a parent `Stretch`
    /// can resize a child before the child's own layout runs).
    ///
    /// Main axis: children flow from `layout_padding`, separated by
    /// `layout_gap`, each keeping its stored main-axis size. Cross
    /// axis follows `layout_align`; `Stretch` also resizes the child
    /// to the container's cross size minus both paddings. With no
    /// layout the stored positions are left untouched.
    pub fn apply_layout(&mut self) {
        if self.has_layout() && !self.children.is_empty() {
            let pad = self.layout_padding.max(0.0);
            let gap = self.layout_gap.max(0.0);
            let align = self.layout_align;
            match self.layout_direction {
                LayoutDirection::Row => {
                    let mut cursor = pad;
                    for child in &mut self.children {
                        child.x = cursor;
                        child.y = match align {
                            LayoutAlign::Start => pad,
                            LayoutAlign::Center => (self.height - child.height) / 2.0,
                            LayoutAlign::End => self.height - pad - child.height,
                            LayoutAlign::Stretch => {
                                child.height = (self.height - 2.0 * pad).max(0.0);
                                pad
                            }
                        };
                        cursor += child.width + gap;
                    }
                }
                LayoutDirection::Column => {
                    let mut cursor = pad;
                    for child in &mut self.children {
                        child.y = cursor;
                        child.x = match align {
                            LayoutAlign::Start => pad,
                            LayoutAlign::Center => (self.width - child.width) / 2.0,
                            LayoutAlign::End => self.width - pad - child.width,
                            LayoutAlign::Stretch => {
                                child.width = (self.width - 2.0 * pad).max(0.0);
                                pad
                            }
                        };
                        cursor += child.height + gap;
                    }
                }
                LayoutDirection::None => {}
            }
        }
        for child in &mut self.children {
            child.apply_layout();
        }
    }
    pub fn with_prop(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.props.insert(k.into(), v.into());
        self
    }
    /// Depth first search, self included.
    pub fn find<'a>(&'a self, id: &str) -> Option<&'a Node> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }
    /// An image node backed by an asset file (path relative to the
    /// project file, stored in the `src` prop).
    pub fn image(id: impl Into<String>, src: impl Into<String>, x: f32, y: f32, w: f32, h: f32) -> Self {
        let mut n = Self::new(id, "Image", NodeKind::Image, x, y, w, h);
        n.props.insert("src".into(), src.into());
        n
    }

    /// The asset path of an image node, when it has one.
    pub fn image_src(&self) -> Option<&str> {
        self.props.get("src").map(|s| s.as_str())
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut Node> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(id))
    }
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(|c| c.count()).sum::<usize>()
    }
    pub fn walk(&self, f: &mut impl FnMut(&Node)) {
        f(self);
        for c in &self.children {
            c.walk(f);
        }
    }
    /// Absolute position after accumulating ancestor offsets.
    pub fn absolute_bounds(&self, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
        (ox + self.x, oy + self.y, self.width, self.height)
    }
}

/// One routable page of an app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub route: String,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub background: Option<Color>,
    pub root: Node,
}
impl Page {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        route: impl Into<String>,
        width: f32,
        height: f32,
    ) -> Self {
        let root = Node::new("root", "Root", NodeKind::Frame, 0.0, 0.0, width, height);
        Self {
            id: id.into(),
            name: name.into(),
            route: route.into(),
            width,
            height,
            background: Some(Color::WHITE),
            root,
        }
    }
    pub fn node_count(&self) -> usize {
        self.root.count()
    }
}

/// A reusable component definition (a named node tree).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDef {
    pub id: String,
    pub name: String,
    pub root: Node,
}

/// Design tokens shared by the whole project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub primary: Color,
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub muted_text: Color,
    pub radius: f32,
    pub font_family: String,
    pub dark: bool,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "Chida Light".into(),
            primary: Color::rgb(16, 122, 87),
            background: Color::WHITE,
            surface: Color::rgb(243, 245, 244),
            text: Color::rgb(20, 24, 22),
            muted_text: Color::rgb(96, 106, 101),
            radius: 12.0,
            font_family: "Vazirmatn, system-ui, sans-serif".into(),
            dark: false,
        }
    }
}

/// A whole Chida design project (design side; data and logic live in the
/// companion crates and are bundled by the CLI file format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub theme: Theme,
    pub pages: Vec<Page>,
    #[serde(default)]
    pub components: Vec<ComponentDef>,
}
impl Project {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            format_version: 1,
            id: id.into(),
            name: name.into(),
            description: String::new(),
            theme: Theme::default(),
            pages: Vec::new(),
            components: Vec::new(),
        }
    }
    pub fn page(&self, route_or_id: &str) -> Option<&Page> {
        self.pages
            .iter()
            .find(|p| p.route == route_or_id || p.id == route_or_id || p.name == route_or_id)
    }
    pub fn total_nodes(&self) -> usize {
        self.pages.iter().map(|p| p.node_count()).sum()
    }
    pub fn to_json(&self) -> Result<String, CoreError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn from_json(s: &str) -> Result<Self, CoreError> {
        Ok(serde_json::from_str(s)?)
    }
    pub fn save(&self, path: &std::path::Path) -> Result<(), CoreError> {
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }
    pub fn load(path: &std::path::Path) -> Result<Self, CoreError> {
        Self::from_json(&std::fs::read_to_string(path)?)
    }
    /// Structural validation used by the CLI and the editor.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.pages.is_empty() {
            issues.push("project has no pages".into());
        }
        let mut routes = std::collections::BTreeSet::new();
        for p in &self.pages {
            if !routes.insert(&p.route) {
                issues.push(format!("duplicate route {}", p.route));
            }
            if p.width <= 0.0 || p.height <= 0.0 {
                issues.push(format!("page {} has invalid size", p.name));
            }
            let mut ids = std::collections::BTreeSet::new();
            p.root.walk(&mut |n| {
                if !ids.insert(n.id.clone()) {
                    issues.push(format!("duplicate node id {} on page {}", n.id, p.name));
                }
            });
        }
        issues
    }
}

/// Generic undo and redo history with a bounded past stack.
#[derive(Debug, Clone)]
pub struct History<T: Clone> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
    limit: usize,
}
impl<T: Clone> History<T> {
    pub fn new(initial: T) -> Self {
        Self::with_limit(initial, 100)
    }
    pub fn with_limit(initial: T, limit: usize) -> Self {
        Self {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
            limit,
        }
    }
    pub fn present(&self) -> &T {
        &self.present
    }
    pub fn commit(&mut self, next: T) {
        self.past.push(self.present.clone());
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        self.present = next;
        self.future.clear();
    }
    pub fn undo(&mut self) -> Option<&T> {
        let prev = self.past.pop()?;
        self.future.push(self.present.clone());
        self.present = prev;
        Some(&self.present)
    }
    pub fn redo(&mut self) -> Option<&T> {
        let next = self.future.pop()?;
        self.past.push(self.present.clone());
        self.present = next;
        Some(&self.present)
    }
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    pub fn undo_depth(&self) -> usize {
        self.past.len()
    }
}

#[derive(Debug)]
pub enum CoreError {
    BadColor(String),
    Json(serde_json::Error),
    Io(std::io::Error),
}
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadColor(s) => write!(f, "invalid colour: {s}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for CoreError {}
impl From<serde_json::Error> for CoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Project {
        let mut p = Project::new("p1", "Shop");
        let mut page = Page::new("home", "Home", "/", 800.0, 600.0);
        page.root.children.push(Node::rect(
            "r1",
            "Banner",
            0.0,
            0.0,
            800.0,
            80.0,
            Color::rgb(10, 20, 30),
        ));
        page.root.children.push(Node::text_node(
            "t1",
            "Title",
            16.0,
            16.0,
            300.0,
            40.0,
            "Hello",
            24.0,
            Color::WHITE,
        ));
        p.pages.push(page);
        p
    }
    #[test]
    fn hex_roundtrip() {
        let c = Color::from_hex("#107a57").unwrap();
        assert_eq!(c, Color::rgb(16, 122, 87));
        assert_eq!(c.to_hex(), "#107a57");
        assert!(Color::from_hex("red").is_err());
        assert_eq!(Color::from_hex("#fff").unwrap(), Color::WHITE);
    }
    #[test]
    fn alpha_blend() {
        assert_eq!(
            Color::rgba(0, 0, 0, 128).over(Color::WHITE),
            Color::rgb(127, 127, 127)
        );
    }
    #[test]
    fn json_roundtrip() {
        let p = sample();
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(p, back);
    }
    #[test]
    fn find_and_count() {
        let p = sample();
        let page = &p.pages[0];
        assert_eq!(page.node_count(), 3);
        assert_eq!(page.root.find("t1").unwrap().text.as_deref(), Some("Hello"));
        assert!(page.root.find("nope").is_none());
    }
    #[test]
    fn find_mut_edits() {
        let mut p = sample();
        p.pages[0].root.find_mut("t1").unwrap().text = Some("Hi".into());
        assert_eq!(
            p.pages[0].root.find("t1").unwrap().text.as_deref(),
            Some("Hi")
        );
    }
    #[test]
    fn validation_catches_dupes() {
        let mut p = sample();
        p.pages.push(p.pages[0].clone());
        assert!(!p.validate().is_empty());
        assert!(sample().validate().is_empty());
    }
    #[test]
    fn gradient_and_shadow_roundtrip() {
        let mut p = sample();
        let node = &mut p.pages[0].root.children[0];
        node.fill_gradient = Some(Gradient::new(Color::rgb(10, 20, 30), Color::WHITE, 90.0));
        node.shadow = Some(Shadow::soft());
        node.font_weight = 500;
        node.line_height = 1.6;
        node.letter_spacing = 2.0;
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(p, back);
        let g = back.pages[0].root.children[0].fill_gradient.unwrap();
        assert_eq!(g.at(0.0), Color::rgb(10, 20, 30));
        assert_eq!(g.at(1.0), Color::WHITE);
        assert_eq!(g.at(0.5).g, 138);
    }
    #[test]
    fn old_json_without_style_props_loads() {
        // Project files written before gradients, shadows and the
        // typography props existed must still parse, with the new
        // fields empty and the legacy bold flag driving the weight.
        let raw = r##"{"format_version":1,"id":"old","name":"Old",
          "theme":{"name":"T","primary":{"r":16,"g":122,"b":87,"a":255},"background":{"r":255,"g":255,"b":255,"a":255},"surface":{"r":243,"g":245,"b":244,"a":255},"text":{"r":20,"g":24,"b":24,"a":255},"muted_text":{"r":96,"g":106,"b":101,"a":255},"radius":12.0,"font_family":"Vazirmatn","dark":false},
          "pages":[{"id":"home","name":"Home","route":"/","width":100.0,"height":100.0,
            "root":{"id":"root","name":"Root","kind":"frame","x":0.0,"y":0.0,"width":100.0,"height":100.0,
              "children":[{"id":"t","name":"T","kind":"text","x":0.0,"y":0.0,"width":50.0,"height":20.0,"bold":true,"text":"Hi"}]}}]}"##;
        let p = Project::from_json(raw).unwrap();
        let n = &p.pages[0].root.children[0];
        assert!(n.fill_gradient.is_none());
        assert!(n.shadow.is_none());
        assert!(!n.has_layout(), "old files default to free positioning");
        assert_eq!(n.font_weight, 0);
        assert_eq!(n.effective_weight(), 700);
        assert!(n.effective_bold());
    }
    fn kid(id: &str, w: f32, h: f32) -> Node {
        Node::new(id, id, NodeKind::Rectangle, 999.0, 999.0, w, h)
    }
    #[test]
    fn row_layout_flows_children_with_gap_and_padding() {
        let mut c = Node::new("c", "Row", NodeKind::Frame, 0.0, 0.0, 400.0, 100.0);
        c.layout_direction = LayoutDirection::Row;
        c.layout_gap = 10.0;
        c.layout_padding = 20.0;
        c.children = vec![kid("a", 50.0, 30.0), kid("b", 70.0, 40.0), kid("d", 60.0, 20.0)];
        c.apply_layout();
        let pos: Vec<(f32, f32)> = c.children.iter().map(|k| (k.x, k.y)).collect();
        assert_eq!(pos, vec![(20.0, 20.0), (80.0, 20.0), (160.0, 20.0)]);
    }
    #[test]
    fn column_layout_align_end_and_center() {
        let mut c = Node::new("c", "Col", NodeKind::Frame, 0.0, 0.0, 200.0, 300.0);
        c.layout_direction = LayoutDirection::Column;
        c.layout_gap = 5.0;
        c.layout_padding = 10.0;
        c.layout_align = LayoutAlign::End;
        c.children = vec![kid("a", 50.0, 30.0), kid("b", 80.0, 40.0)];
        c.apply_layout();
        assert_eq!((c.children[0].x, c.children[0].y), (140.0, 10.0));
        assert_eq!((c.children[1].x, c.children[1].y), (110.0, 45.0));
        c.layout_align = LayoutAlign::Center;
        c.apply_layout();
        assert_eq!(c.children[0].x, 75.0);
        assert_eq!(c.children[1].x, 60.0);
        // Main axis positions do not depend on the cross alignment.
        assert_eq!(c.children[1].y, 45.0);
    }
    #[test]
    fn stretch_resizes_the_cross_axis() {
        let mut c = Node::new("c", "Row", NodeKind::Frame, 0.0, 0.0, 300.0, 120.0);
        c.layout_direction = LayoutDirection::Row;
        c.layout_padding = 15.0;
        c.layout_align = LayoutAlign::Stretch;
        c.children = vec![kid("a", 50.0, 30.0)];
        c.apply_layout();
        assert_eq!(c.children[0].height, 90.0);
        assert_eq!(c.children[0].y, 15.0);
        assert_eq!(c.children[0].width, 50.0, "main axis size is kept");
    }
    #[test]
    fn nested_layout_uses_the_stretched_child_size() {
        // Outer row stretches the inner column container to height
        // 200; the inner End alignment must use that final height.
        let mut inner = Node::new("in", "Inner", NodeKind::Frame, 0.0, 0.0, 100.0, 50.0);
        inner.layout_direction = LayoutDirection::Row;
        inner.layout_align = LayoutAlign::End;
        inner.children = vec![kid("leaf", 30.0, 20.0)];
        let mut outer = Node::new("out", "Outer", NodeKind::Frame, 0.0, 0.0, 400.0, 200.0);
        outer.layout_direction = LayoutDirection::Row;
        outer.layout_align = LayoutAlign::Stretch;
        outer.children = vec![inner];
        outer.apply_layout();
        let inner = &outer.children[0];
        assert_eq!(inner.height, 200.0);
        // Row cross axis is y: End alignment must use the stretched
        // height (200), not the stored one (50).
        assert_eq!(inner.children[0].y, 180.0);
        assert_eq!(inner.children[0].x, 0.0);
    }
    #[test]
    fn no_layout_keeps_stored_positions() {
        let mut c = Node::new("c", "Free", NodeKind::Frame, 0.0, 0.0, 100.0, 100.0);
        c.children = vec![kid("a", 10.0, 10.0)];
        c.apply_layout();
        assert_eq!((c.children[0].x, c.children[0].y), (999.0, 999.0));
    }
    #[test]
    fn history_undo_redo() {
        let mut h = History::new(1);
        h.commit(2);
        h.commit(3);
        assert_eq!(h.undo(), Some(&2));
        assert_eq!(h.undo(), Some(&1));
        assert_eq!(h.redo(), Some(&2));
        h.commit(9);
        assert!(!h.can_redo());
    }
    #[test]
    fn history_limit() {
        let mut h = History::with_limit(0, 2);
        for i in 1..6 {
            h.commit(i);
        }
        assert_eq!(h.undo_depth(), 2);
    }
}
