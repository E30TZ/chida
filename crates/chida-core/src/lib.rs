//! chida-core: the document model every other Chida crate builds on.
//!
//! A [`Project`] owns pages, reusable components and a design theme.
//! Each [`Page`] owns a tree of [`Node`] values (the scene graph).
//! Projects are saved and loaded as JSON (the `.chida.json` format).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

/// One colour stop in a linear gradient.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}

/// A linear gradient fill. `angle` is in degrees, CSS style:
/// 0 points up, 90 points right, 180 points down. Files from
/// Chida 1.x carry only `from` and `to`; when `stops` is absent or
/// empty those two colours are the effective stops at 0 and 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    pub from: Color,
    pub to: Color,
    #[serde(default = "default_gradient_angle")]
    pub angle: f32,
    #[serde(default)]
    pub stops: Vec<GradientStop>,
}
fn default_gradient_angle() -> f32 {
    180.0
}
impl Gradient {
    pub fn new(from: Color, to: Color, angle: f32) -> Self {
        Self {
            from,
            to,
            angle,
            stops: vec![
                GradientStop { offset: 0.0, color: from },
                GradientStop { offset: 1.0, color: to },
            ],
        }
    }
    pub fn with_stops(mut stops: Vec<GradientStop>, angle: f32) -> Self {
        stops.retain(|s| s.offset.is_finite());
        for stop in &mut stops {
            stop.offset = stop.offset.clamp(0.0, 1.0);
        }
        stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
        if stops.len() < 2 {
            return Self::new(Color::BLACK, Color::WHITE, angle);
        }
        let from = stops.first().unwrap().color;
        let to = stops.last().unwrap().color;
        Self { from, to, angle, stops }
    }
    /// Effective stops, synthesising the legacy two colour form.
    pub fn stops(&self) -> Vec<GradientStop> {
        if self.stops.len() >= 2 {
            let mut stops = self.stops.clone();
            stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
            stops
        } else {
            vec![
                GradientStop { offset: 0.0, color: self.from },
                GradientStop { offset: 1.0, color: self.to },
            ]
        }
    }
    pub fn set_stops(&mut self, stops: Vec<GradientStop>) {
        *self = Self::with_stops(stops, self.angle);
    }
    pub fn css(&self) -> String {
        let stops = self
            .stops()
            .iter()
            .map(|s| format!("{} {}%", s.color.to_hex(), (s.offset.clamp(0.0, 1.0) * 100.0).round()))
            .collect::<Vec<_>>()
            .join(", ");
        format!("linear-gradient({}deg, {})", self.angle, stops)
    }
    /// The colour at position `t` (0 = start, 1 = end).
    pub fn at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let stops = self.stops();
        if t <= stops[0].offset {
            return stops[0].color;
        }
        for pair in stops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.offset {
                let span = (b.offset - a.offset).max(f32::EPSILON);
                let local = (t - a.offset) / span;
                let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * local).round() as u8;
                return Color::rgba(mix(a.color.r, b.color.r), mix(a.color.g, b.color.g), mix(a.color.b, b.color.b), mix(a.color.a, b.color.a));
            }
        }
        stops[stops.len() - 1].color
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
    #[serde(default = "default_shadow_enabled")]
    pub enabled: bool,
}
fn default_shadow_opacity() -> f32 {
    0.25
}
fn default_shadow_enabled() -> bool {
    true
}
impl Shadow {
    pub fn soft() -> Self {
        Self {
            x: 0.0,
            y: 8.0,
            blur: 24.0,
            color: Color::BLACK,
            opacity: 0.18,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
enum ShadowInput {
    One(Shadow),
    Many(Vec<Shadow>),
}

fn deserialize_shadows<'de, D>(deserializer: D) -> Result<Vec<Shadow>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<ShadowInput>::deserialize(deserializer)?;
    Ok(match value {
        Some(ShadowInput::One(shadow)) => vec![shadow],
        Some(ShadowInput::Many(shadows)) => shadows,
        None => Vec::new(),
    })
}

/// How a node sizes itself on one axis inside its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AxisSizing {
    #[default]
    Fixed,
    Hug,
    Fill,
}

/// Distribution of children along an auto-layout main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutJustify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

/// One anchor of a vector path. Coordinates are local to the node.
/// `in_point` and `out_point` are absolute local coordinates (not
/// offsets), which keeps editor handle dragging simple.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub in_x: Option<f32>,
    #[serde(default)]
    pub in_y: Option<f32>,
    #[serde(default)]
    pub out_x: Option<f32>,
    #[serde(default)]
    pub out_y: Option<f32>,
}
impl PathPoint {
    pub fn anchor(x: f32, y: f32) -> Self {
        Self { x, y, in_x: None, in_y: None, out_x: None, out_y: None }
    }
    pub fn smooth(x: f32, y: f32, in_point: (f32, f32), out_point: (f32, f32)) -> Self {
        Self { x, y, in_x: Some(in_point.0), in_y: Some(in_point.1), out_x: Some(out_point.0), out_y: Some(out_point.1) }
    }
    pub fn in_point(&self) -> Option<(f32, f32)> { self.in_x.zip(self.in_y) }
    pub fn out_point(&self) -> Option<(f32, f32)> { self.out_x.zip(self.out_y) }
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
    Path,
    Arrow,
    Component,
    Group,
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
    /// Drop shadows painted behind the node, first entry on top.
    /// Chida 1.x stored one object under the `shadow` key; the
    /// alias and custom reader accept that object or a new array.
    #[serde(default, alias = "shadow", deserialize_with = "deserialize_shadows")]
    pub shadows: Vec<Shadow>,
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
    /// Name or id of a component this node instantiates.
    #[serde(default)]
    pub component: Option<String>,
    /// Vector anchors for `Path` and `Arrow` nodes, in local units.
    #[serde(default)]
    pub path_points: Vec<PathPoint>,
    #[serde(default)]
    pub path_closed: bool,
    #[serde(default)]
    pub width_mode: AxisSizing,
    #[serde(default)]
    pub height_mode: AxisSizing,
    /// Layer blur radius in pixels. The editor canvas and generated
    /// CSS honour it; the small CPU PNG rasteriser documents it as
    /// an unblurred paint (see chida-render).
    #[serde(default)]
    pub blur_radius: f32,
    /// Backdrop blur radius for a translucent fill.
    #[serde(default)]
    pub background_blur: f32,
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
    #[serde(default)]
    pub layout_justify: LayoutJustify,
    #[serde(default)]
    pub layout_wrap: bool,
    /// Optional per-side padding. `None` falls back to the uniform
    /// `layout_padding` value used by Chida 1.x files.
    #[serde(default)]
    pub layout_padding_top: Option<f32>,
    #[serde(default)]
    pub layout_padding_right: Option<f32>,
    #[serde(default)]
    pub layout_padding_bottom: Option<f32>,
    #[serde(default)]
    pub layout_padding_left: Option<f32>,
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
            shadows: Vec::new(),
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
            path_points: Vec::new(),
            path_closed: false,
            width_mode: AxisSizing::Fixed,
            height_mode: AxisSizing::Fixed,
            blur_radius: 0.0,
            background_blur: 0.0,
            layout_direction: LayoutDirection::None,
            layout_gap: 0.0,
            layout_padding: 0.0,
            layout_align: LayoutAlign::Start,
            layout_justify: LayoutJustify::Start,
            layout_wrap: false,
            layout_padding_top: None,
            layout_padding_right: None,
            layout_padding_bottom: None,
            layout_padding_left: None,
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
        self.shadows = vec![shadow];
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
    /// Padding on each side: top, right, bottom, left. A per-side
    /// value wins over the uniform legacy `layout_padding`.
    pub fn padding_values(&self) -> (f32, f32, f32, f32) {
        let base = self.layout_padding.max(0.0);
        (
            self.layout_padding_top.unwrap_or(base).max(0.0),
            self.layout_padding_right.unwrap_or(base).max(0.0),
            self.layout_padding_bottom.unwrap_or(base).max(0.0),
            self.layout_padding_left.unwrap_or(base).max(0.0),
        )
    }

    fn hug_content_size(&self) -> (f32, f32) {
        if self.children.is_empty() {
            return (self.width, self.height);
        }
        let (pt, pr, pb, pl) = self.padding_values();
        let gap = self.layout_gap.max(0.0);
        match self.layout_direction {
            LayoutDirection::Row => {
                let width = pl
                    + pr
                    + self.children.iter().map(|c| c.width).sum::<f32>()
                    + gap * (self.children.len().saturating_sub(1) as f32);
                let height = pt
                    + pb
                    + self
                        .children
                        .iter()
                        .map(|c| c.height)
                        .fold(0.0f32, f32::max);
                (width, height)
            }
            LayoutDirection::Column => {
                let width = pl
                    + pr
                    + self
                        .children
                        .iter()
                        .map(|c| c.width)
                        .fold(0.0f32, f32::max);
                let height = pt
                    + pb
                    + self.children.iter().map(|c| c.height).sum::<f32>()
                    + gap * (self.children.len().saturating_sub(1) as f32);
                (width, height)
            }
            LayoutDirection::None => (
                self.children
                    .iter()
                    .map(|c| (c.x + c.width).max(0.0))
                    .fold(0.0f32, f32::max),
                self.children
                    .iter()
                    .map(|c| (c.y + c.height).max(0.0))
                    .fold(0.0f32, f32::max),
            ),
        }
    }

    /// SVG path data for a `Path` or `Arrow` node, translated by
    /// the absolute origin `ox, oy`.
    pub fn path_d(&self, ox: f32, oy: f32) -> String {
        if self.path_points.is_empty() {
            return String::new();
        }
        let point = |p: &PathPoint| (ox + p.x, oy + p.y);
        let first = self.path_points[0];
        let (fx, fy) = point(&first);
        let mut d = format!("M {fx} {fy}");
        let segments = if self.path_closed && self.path_points.len() > 1 {
            self.path_points.len()
        } else {
            self.path_points.len().saturating_sub(1)
        };
        for i in 0..segments {
            let cur = self.path_points[i];
            let next = self.path_points[(i + 1) % self.path_points.len()];
            let (nx, ny) = point(&next);
            match (cur.out_point(), next.in_point()) {
                (None, None) => d.push_str(&format!(" L {nx} {ny}")),
                (out_point, in_point) => {
                    let c1 = out_point
                        .map(|(x, y)| (ox + x, oy + y))
                        .unwrap_or_else(|| point(&cur));
                    let c2 = in_point
                        .map(|(x, y)| (ox + x, oy + y))
                        .unwrap_or_else(|| point(&next));
                    d.push_str(&format!(
                        " C {} {}, {} {}, {nx} {ny}",
                        c1.0, c1.1, c2.0, c2.1
                    ));
                }
            }
        }
        if self.path_closed {
            d.push_str(" Z");
        }
        d
    }

    /// The path outline flattened to line segments (curves are
    /// sampled), translated by the absolute origin `ox, oy`.
    pub fn flattened_path(&self, ox: f32, oy: f32) -> Vec<(f32, f32)> {
        if self.path_points.is_empty() {
            return Vec::new();
        }
        let mut out = vec![(ox + self.path_points[0].x, oy + self.path_points[0].y)];
        let segments = if self.path_closed && self.path_points.len() > 1 {
            self.path_points.len()
        } else {
            self.path_points.len().saturating_sub(1)
        };
        for i in 0..segments {
            let cur = self.path_points[i];
            let next = self.path_points[(i + 1) % self.path_points.len()];
            let p0 = (ox + cur.x, oy + cur.y);
            let p3 = (ox + next.x, oy + next.y);
            match (cur.out_point(), next.in_point()) {
                (None, None) => out.push(p3),
                (out_point, in_point) => {
                    let p1 = out_point.map(|(x, y)| (ox + x, oy + y)).unwrap_or(p0);
                    let p2 = in_point.map(|(x, y)| (ox + x, oy + y)).unwrap_or(p3);
                    for step in 1..=12 {
                        let t = step as f32 / 12.0;
                        let mt = 1.0 - t;
                        let x = mt.powi(3) * p0.0
                            + 3.0 * mt.powi(2) * t * p1.0
                            + 3.0 * mt * t.powi(2) * p2.0
                            + t.powi(3) * p3.0;
                        let y = mt.powi(3) * p0.1
                            + 3.0 * mt.powi(2) * t * p1.1
                            + 3.0 * mt * t.powi(2) * p2.1
                            + t.powi(3) * p3.1;
                        out.push((x, y));
                    }
                }
            }
        }
        out
    }

    /// The arrowhead polygon (local coordinates) for an `Arrow`
    /// node whose first two path points are the shaft ends.
    pub fn arrow_head_points(&self) -> Option<Vec<(f32, f32)>> {
        if self.path_points.len() < 2 {
            return None;
        }
        let start = (self.path_points[0].x, self.path_points[0].y);
        let end = (self.path_points[1].x, self.path_points[1].y);
        let (dx, dy) = (end.0 - start.0, end.1 - start.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return None;
        }
        let unit = (dx / len, dy / len);
        let normal = (-unit.1, unit.0);
        let head_len = (len * 0.32).clamp(8.0, 22.0);
        let head_half = (self.stroke_width.max(1.5) * 3.0).clamp(5.0, 14.0);
        let base = (end.0 - unit.0 * head_len, end.1 - unit.1 * head_len);
        Some(vec![
            (base.0 + normal.0 * head_half, base.1 + normal.1 * head_half),
            end,
            (base.0 - normal.0 * head_half, base.1 - normal.1 * head_half),
        ])
    }

    /// A closed regular polygon as a path node.
    pub fn polygon(
        id: impl Into<String>,
        name: impl Into<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        sides: usize,
        fill: Color,
    ) -> Self {
        let sides = sides.clamp(3, 10);
        let (cx, cy) = (width / 2.0, height / 2.0);
        let points = (0..sides)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2
                    + i as f32 * std::f32::consts::TAU / sides as f32;
                PathPoint::anchor(cx + cx * a.cos(), cy + cy * a.sin())
            })
            .collect();
        let mut node = Self::new(id, name, NodeKind::Path, x, y, width, height);
        node.fill = Some(fill);
        node.path_points = points;
        node.path_closed = true;
        node.props.insert("shape".into(), "polygon".into());
        node.props.insert("sides".into(), sides.to_string());
        node
    }

    /// A closed star as a path node.
    pub fn star(
        id: impl Into<String>,
        name: impl Into<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        points_count: usize,
        inner_ratio: f32,
        fill: Color,
    ) -> Self {
        let points_count = points_count.clamp(3, 12);
        let ratio = inner_ratio.clamp(0.1, 0.95);
        let (cx, cy) = (width / 2.0, height / 2.0);
        let mut points = Vec::new();
        for i in 0..points_count * 2 {
            let a = -std::f32::consts::FRAC_PI_2
                + i as f32 * std::f32::consts::PI / points_count as f32;
            let r = if i % 2 == 0 { 1.0 } else { ratio };
            points.push(PathPoint::anchor(
                cx + cx * r * a.cos(),
                cy + cy * r * a.sin(),
            ));
        }
        let mut node = Self::new(id, name, NodeKind::Path, x, y, width, height);
        node.fill = Some(fill);
        node.path_points = points;
        node.path_closed = true;
        node.props.insert("shape".into(), "star".into());
        node.props.insert("points".into(), points_count.to_string());
        node.props
            .insert("inner_ratio".into(), format!("{ratio:.3}"));
        node
    }

    /// Wrap every set of two or more selected direct children, at
    /// every level of this tree, in a new `Group` node whose origin
    /// is the selection bounding box. Returns the new group ids.
    pub fn group_children(
        &mut self,
        ids: &BTreeSet<String>,
        id_prefix: &str,
        counter: &mut u32,
    ) -> Vec<String> {
        let mut created = Vec::new();
        for child in &mut self.children {
            created.extend(child.group_children(ids, id_prefix, counter));
        }
        let positions: Vec<usize> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| ids.contains(&child.id))
            .map(|(i, _)| i)
            .collect();
        if positions.len() < 2 {
            return created;
        }
        let first = positions[0];
        let min_x = positions
            .iter()
            .map(|&i| self.children[i].x)
            .fold(f32::INFINITY, f32::min);
        let min_y = positions
            .iter()
            .map(|&i| self.children[i].y)
            .fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|&i| self.children[i].x + self.children[i].width)
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = positions
            .iter()
            .map(|&i| self.children[i].y + self.children[i].height)
            .fold(f32::NEG_INFINITY, f32::max);
        let mut taken: Vec<Node> = Vec::new();
        for &pos in positions.iter().rev() {
            taken.push(self.children.remove(pos));
        }
        taken.reverse();
        for child in &mut taken {
            child.x -= min_x;
            child.y -= min_y;
        }
        *counter += 1;
        let mut group = Node::new(
            format!("{id_prefix}-{counter}"),
            "Group",
            NodeKind::Group,
            min_x,
            min_y,
            (max_x - min_x).max(1.0),
            (max_y - min_y).max(1.0),
        );
        group.props.insert("real_group".into(), "1".into());
        group.children = taken;
        let group_id = group.id.clone();
        self.children.insert(first, group);
        created.push(group_id);
        created
    }

    /// Reverse [`Node::group_children`] for the given group ids:
    /// their children move back into the parent at the group slot,
    /// shifted by the group origin. Returns the released node ids.
    pub fn ungroup_children(&mut self, ids: &BTreeSet<String>) -> Vec<String> {
        let mut released = Vec::new();
        let mut i = 0;
        while i < self.children.len() {
            let is_group = self.children[i].kind == NodeKind::Group
                || self.children[i].props.contains_key("real_group")
                || self.children[i].props.contains_key("comp_group");
            if ids.contains(&self.children[i].id) && is_group {
                let group = self.children.remove(i);
                let count = group.children.len();
                let mut children = group.children;
                for child in &mut children {
                    child.x += group.x;
                    child.y += group.y;
                    released.push(child.id.clone());
                }
                for (j, child) in children.into_iter().enumerate() {
                    self.children.insert(i + j, child);
                }
                i += count;
            } else {
                released.extend(self.children[i].ungroup_children(ids));
                i += 1;
            }
        }
        released
    }

    /// Recompute this subtree's layout. Children are laid out
    /// first (so hug sizing sees final content), then this node's
    /// own hug size, then child positions for an auto-layout
    /// container: main-axis flow with gap, padding per side,
    /// optional wrapping for rows, cross-axis alignment, per-axis
    /// `Fill` sizing and the main-axis justify setting. A parent
    /// resize of a child re-runs that child's layout afterwards.
    pub fn apply_layout(&mut self) {
        for child in &mut self.children {
            child.apply_layout();
        }

        let (pt, pr, pb, pl) = self.padding_values();

        if !self.has_layout() {
            // Fill in a free-form parent means: stretch from the
            // child's position to the parent's inner far edge.
            let mut changed: Vec<usize> = Vec::new();
            for (i, child) in self.children.iter_mut().enumerate() {
                let old = (child.width, child.height);
                if child.width_mode == AxisSizing::Fill {
                    child.width = (self.width - child.x - pr).max(0.0);
                }
                if child.height_mode == AxisSizing::Fill {
                    child.height = (self.height - child.y - pb).max(0.0);
                }
                if (child.width, child.height) != old {
                    changed.push(i);
                }
            }
            for i in changed {
                if self.children[i].has_layout() {
                    self.children[i].apply_layout();
                }
            }
        }

        if self.width_mode == AxisSizing::Hug || self.height_mode == AxisSizing::Hug {
            let (content_w, content_h) = self.hug_content_size();
            if self.width_mode == AxisSizing::Hug {
                self.width = content_w.max(0.0);
            }
            if self.height_mode == AxisSizing::Hug {
                self.height = content_h.max(0.0);
            }
        }

        if !self.has_layout() || self.children.is_empty() {
            return;
        }

        let gap = self.layout_gap.max(0.0);
        let inner_w = (self.width - pl - pr).max(0.0);
        let inner_h = (self.height - pt - pb).max(0.0);
        let align = self.layout_align;
        let justify = self.layout_justify;
        let mut resized: Vec<usize> = Vec::new();
        let mark_resized =
            |resized: &mut Vec<usize>, i: usize, child: &Node, old: (f32, f32)| {
                if (child.width, child.height) != old && !resized.contains(&i) {
                    resized.push(i);
                }
            };

        match self.layout_direction {
            LayoutDirection::Row => {
                if !self.layout_wrap {
                    let fill_count = self
                        .children
                        .iter()
                        .filter(|c| c.width_mode == AxisSizing::Fill)
                        .count();
                    if fill_count > 0 {
                        let fixed: f32 = self
                            .children
                            .iter()
                            .filter(|c| c.width_mode != AxisSizing::Fill)
                            .map(|c| c.width)
                            .sum();
                        let each = ((inner_w
                            - fixed
                            - gap * (self.children.len().saturating_sub(1) as f32))
                            / fill_count as f32)
                            .max(0.0);
                        for (i, child) in self.children.iter_mut().enumerate() {
                            let old = (child.width, child.height);
                            if child.width_mode == AxisSizing::Fill {
                                child.width = each;
                            }
                            if child.height_mode == AxisSizing::Fill {
                                child.height = inner_h;
                            }
                            mark_resized(&mut resized, i, child, old);
                        }
                    } else {
                        for (i, child) in self.children.iter_mut().enumerate() {
                            let old = (child.width, child.height);
                            if child.height_mode == AxisSizing::Fill {
                                child.height = inner_h;
                            }
                            mark_resized(&mut resized, i, child, old);
                        }
                    }
                }
                // Group children into rows (one row without wrap).
                let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
                let mut row_width = 0.0f32;
                for (i, child) in self.children.iter().enumerate() {
                    let needed = if rows.last().unwrap().is_empty() {
                        child.width
                    } else {
                        row_width + gap + child.width
                    };
                    if self.layout_wrap
                        && !rows.last().unwrap().is_empty()
                        && needed > inner_w
                    {
                        rows.push(Vec::new());
                        row_width = child.width;
                    } else {
                        row_width = needed;
                    }
                    rows.last_mut().unwrap().push(i);
                }
                let mut y = pt;
                for row in &rows {
                    let row_h = if self.layout_wrap {
                        row.iter()
                            .map(|&i| self.children[i].height)
                            .fold(0.0f32, f32::max)
                    } else {
                        inner_h
                    };
                    let sum_w: f32 = row.iter().map(|&i| self.children[i].width).sum();
                    let (start_x, step_gap) = match justify {
                        LayoutJustify::Start => (pl, gap),
                        LayoutJustify::Center => {
                            (pl + (inner_w - sum_w - gap * (row.len().saturating_sub(1) as f32)).max(0.0) / 2.0, gap)
                        }
                        LayoutJustify::End => {
                            (pl + (inner_w - sum_w - gap * (row.len().saturating_sub(1) as f32)).max(0.0), gap)
                        }
                        LayoutJustify::SpaceBetween => {
                            if row.len() > 1 {
                                (pl, ((inner_w - sum_w) / (row.len() - 1) as f32).max(0.0))
                            } else {
                                (pl, gap)
                            }
                        }
                    };
                    let mut x = start_x;
                    for &i in row {
                        let old = (self.children[i].width, self.children[i].height);
                        self.children[i].x = x;
                        self.children[i].y = match align {
                            LayoutAlign::Start => y,
                            LayoutAlign::Center => y + (row_h - self.children[i].height) / 2.0,
                            LayoutAlign::End => y + row_h - self.children[i].height,
                            LayoutAlign::Stretch => {
                                self.children[i].height = row_h;
                                y
                            }
                        };
                        let child = &self.children[i];
                        mark_resized(&mut resized, i, child, old);
                        x += self.children[i].width + step_gap;
                    }
                    y += row_h + gap;
                }
            }
            LayoutDirection::Column => {
                let fill_count = self
                    .children
                    .iter()
                    .filter(|c| c.height_mode == AxisSizing::Fill)
                    .count();
                if fill_count > 0 {
                    let fixed: f32 = self
                        .children
                        .iter()
                        .filter(|c| c.height_mode != AxisSizing::Fill)
                        .map(|c| c.height)
                        .sum();
                    let each = ((inner_h
                        - fixed
                        - gap * (self.children.len().saturating_sub(1) as f32))
                        / fill_count as f32)
                        .max(0.0);
                    for (i, child) in self.children.iter_mut().enumerate() {
                        let old = (child.width, child.height);
                        if child.height_mode == AxisSizing::Fill {
                            child.height = each;
                        }
                        if child.width_mode == AxisSizing::Fill {
                            child.width = inner_w;
                        }
                        mark_resized(&mut resized, i, child, old);
                    }
                } else {
                    for (i, child) in self.children.iter_mut().enumerate() {
                        let old = (child.width, child.height);
                        if child.width_mode == AxisSizing::Fill {
                            child.width = inner_w;
                        }
                        mark_resized(&mut resized, i, child, old);
                    }
                }
                let sum_h: f32 = self.children.iter().map(|c| c.height).sum();
                let (start_y, step_gap) = match justify {
                    LayoutJustify::Start => (pt, gap),
                    LayoutJustify::Center => {
                        (pt + (inner_h - sum_h - gap * (self.children.len().saturating_sub(1) as f32)).max(0.0) / 2.0, gap)
                    }
                    LayoutJustify::End => {
                        (pt + (inner_h - sum_h - gap * (self.children.len().saturating_sub(1) as f32)).max(0.0), gap)
                    }
                    LayoutJustify::SpaceBetween => {
                        if self.children.len() > 1 {
                            (pt, ((inner_h - sum_h) / (self.children.len() - 1) as f32).max(0.0))
                        } else {
                            (pt, gap)
                        }
                    }
                };
                let mut y = start_y;
                for i in 0..self.children.len() {
                    let old = (self.children[i].width, self.children[i].height);
                    self.children[i].y = y;
                    self.children[i].x = match align {
                        LayoutAlign::Start => pl,
                        LayoutAlign::Center => pl + (inner_w - self.children[i].width) / 2.0,
                        LayoutAlign::End => pl + inner_w - self.children[i].width,
                        LayoutAlign::Stretch => {
                            self.children[i].width = inner_w;
                            pl
                        }
                    };
                    let child = &self.children[i];
                    mark_resized(&mut resized, i, child, old);
                    y += self.children[i].height + step_gap;
                }
            }
            LayoutDirection::None => {}
        }
        for i in resized {
            if self.children[i].has_layout() {
                self.children[i].apply_layout();
            }
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
    pub fn image(
        id: impl Into<String>,
        src: impl Into<String>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Self {
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
    /// X positions of vertical ruler guides stored on the page.
    #[serde(default)]
    pub guides_x: Vec<f32>,
    /// Y positions of horizontal ruler guides stored on the page.
    #[serde(default)]
    pub guides_y: Vec<f32>,
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
            guides_x: Vec::new(),
            guides_y: Vec::new(),
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
    /// Free form project level metadata set by importers and
    /// exporters (for example the WordPress theme flag, slug and
    /// text domain written by chida-wp). Empty for plain projects.
    #[serde(default)]
    pub meta: BTreeMap<String, String>,
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
            meta: BTreeMap::new(),
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
    /// Find a master component by id or name.
    pub fn component_def(&self, key: &str) -> Option<&ComponentDef> {
        self.components
            .iter()
            .find(|c| c.id == key || c.name == key)
    }
    /// Flatten one node for output: a component instance is
    /// replaced by a clone of its master tree with the instance's
    /// own geometry, visibility, opacity and paint/text overrides
    /// applied. Descendant ids are prefixed with the instance id
    /// so several instances never collide. Non-instance nodes are
    /// cloned with their children resolved recursively.
    pub fn resolve_node(&self, node: &Node) -> Node {
        if let Some(key) = &node.component {
            if let Some(def) = self.component_def(key) {
                let mut out = def.root.clone();
                apply_instance_overrides(&mut out, node);
                out.id = node.id.clone();
                out.name = node.name.clone();
                out.x = node.x;
                out.y = node.y;
                out.width = node.width;
                out.height = node.height;
                out.visible = node.visible;
                out.opacity = node.opacity;
                out.component = None;
                out.children = out
                    .children
                    .iter()
                    .map(|child| self.resolve_node(child))
                    .collect();
                prefix_descendant_ids(&mut out, &node.id);
                out.apply_layout();
                return out;
            }
        }
        let mut out = node.clone();
        out.children = node
            .children
            .iter()
            .map(|child| self.resolve_node(child))
            .collect();
        out
    }
    /// A copy of a page with every component instance flattened.
    pub fn resolve_page(&self, page: &Page) -> Page {
        let mut resolved = page.clone();
        resolved.root = self.resolve_node(&page.root);
        resolved.root.apply_layout();
        resolved
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

fn first_text_mut<'a>(node: &'a mut Node) -> Option<&'a mut Node> {
    if node.text.is_some() {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = first_text_mut(child) {
            return Some(found);
        }
    }
    None
}

fn apply_instance_overrides(master: &mut Node, instance: &Node) {
    for (key, value) in &instance.props {
        if let Some(id) = key.strip_prefix("override_text:") {
            if let Some(target) = master.find_mut(id) {
                target.text = Some(value.clone());
            }
        } else if let Some(id) = key.strip_prefix("override_fill:") {
            if let Ok(color) = Color::from_hex(value) {
                if let Some(target) = master.find_mut(id) {
                    target.fill = Some(color);
                    target.fill_gradient = None;
                }
            }
        }
    }
    if let Some(text) = &instance.text {
        if let Some(target) = first_text_mut(master) {
            target.text = Some(text.clone());
        }
    }
    if instance.fill.is_some() {
        master.fill = instance.fill;
        if instance.fill_gradient.is_none() {
            master.fill_gradient = None;
        }
    }
    if instance.fill_gradient.is_some() {
        master.fill_gradient = instance.fill_gradient.clone();
    }
}

fn prefix_descendant_ids(node: &mut Node, prefix: &str) {
    for child in &mut node.children {
        let old = child.id.clone();
        child.id = format!("{prefix}::{old}");
        prefix_descendant_ids(child, prefix);
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
        node.shadows = vec![Shadow::soft()];
        node.font_weight = 500;
        node.line_height = 1.6;
        node.letter_spacing = 2.0;
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(p, back);
        let g = back.pages[0].root.children[0].fill_gradient.clone().unwrap();
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
        assert!(n.shadows.is_empty());
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
        c.children = vec![
            kid("a", 50.0, 30.0),
            kid("b", 70.0, 40.0),
            kid("d", 60.0, 20.0),
        ];
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
    #[test]
    fn path_geometry_polygon_star_arrow() {
        let poly = Node::polygon("p", "Poly", 0.0, 0.0, 100.0, 80.0, 5, Color::rgb(1, 2, 3));
        assert_eq!(poly.kind, NodeKind::Path);
        assert!(poly.path_closed);
        assert_eq!(poly.path_points.len(), 5);
        let d = poly.path_d(0.0, 0.0);
        assert!(d.starts_with('M'));
        assert!(d.trim_end().ends_with('Z'));
        assert!(poly.flattened_path(0.0, 0.0).len() >= 5);
        let star = Node::star("s", "Star", 0.0, 0.0, 100.0, 100.0, 5, 0.5, Color::rgb(1, 2, 3));
        assert_eq!(star.path_points.len(), 10);
        let mut arrow = Node::new("a", "Arrow", NodeKind::Arrow, 0.0, 0.0, 100.0, 20.0);
        arrow.path_points = vec![PathPoint::anchor(0.0, 10.0), PathPoint::anchor(100.0, 10.0)];
        arrow.stroke_width = 3.0;
        let head = arrow.arrow_head_points().expect("arrow head");
        assert_eq!(head[1], (100.0, 10.0));
        // A smooth point carries symmetric handles through path_d.
        let mut curve = Node::new("c", "Curve", NodeKind::Path, 0.0, 0.0, 100.0, 100.0);
        curve.path_points = vec![
            PathPoint::smooth(0.0, 50.0, (-20.0, 50.0), (30.0, 50.0)),
            PathPoint::anchor(100.0, 50.0),
        ];
        assert!(curve.path_d(0.0, 0.0).contains('C'));
    }

    #[test]
    fn gradient_stops_serde_compat() {
        // Old two color JSON (no stops field) still loads and the
        // effective stops come from from/to.
        let g = Gradient::new(Color::rgb(255, 0, 0), Color::rgb(0, 0, 255), 90.0);
        let mut v = serde_json::to_value(&g).unwrap();
        v.as_object_mut().unwrap().remove("stops");
        let old: Gradient = serde_json::from_value(v).unwrap();
        let stops = old.stops();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].color, Color::rgb(255, 0, 0));
        assert_eq!(stops[1].color, Color::rgb(0, 0, 255));
        // A three stop gradient round trips and interpolates.
        let g3 = Gradient::with_stops(
            vec![
                GradientStop { offset: 0.0, color: Color::rgb(255, 0, 0) },
                GradientStop { offset: 0.5, color: Color::rgb(0, 255, 0) },
                GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) },
            ],
            45.0,
        );
        let back: Gradient =
            serde_json::from_str(&serde_json::to_string(&g3).unwrap()).unwrap();
        assert_eq!(back.stops().len(), 3);
        assert_eq!(back.at(0.5), Color::rgb(0, 255, 0));
        assert!(back.css().contains("50%"));
    }

    #[test]
    fn shadow_list_serde_compat() {
        // Old single shadow object JSON loads as a one item list.
        let mut node = Node::rect("r", "R", 0.0, 0.0, 10.0, 10.0, Color::WHITE);
        node.shadows = vec![Shadow {
            x: 3.0,
            y: 4.0,
            blur: 5.0,
            color: Color::rgb(1, 1, 1),
            opacity: 0.5,
            enabled: true,
        }];
        let mut v = serde_json::to_value(&node).unwrap();
        let obj = v.as_object_mut().unwrap();
        let mut shadow = obj.remove("shadows").unwrap();
        let first = shadow.as_array_mut().unwrap().remove(0);
        let mut first = first;
        first.as_object_mut().unwrap().remove("enabled");
        obj.insert("shadow".into(), first);
        let back: Node = serde_json::from_value(v).unwrap();
        assert_eq!(back.shadows.len(), 1);
        assert_eq!(back.shadows[0].x, 3.0);
        assert!(back.shadows[0].enabled);
        // New list form round trips with several entries.
        node.shadows.push(Shadow::soft());
        let back2: Node =
            serde_json::from_str(&serde_json::to_string(&node).unwrap()).unwrap();
        assert_eq!(back2.shadows.len(), 2);
    }

    #[test]
    fn hug_and_fill_layout() {
        let mut frame = Node::new("f", "F", NodeKind::Frame, 0.0, 0.0, 500.0, 100.0);
        frame.layout_direction = LayoutDirection::Row;
        frame.layout_gap = 5.0;
        frame.layout_padding = 10.0;
        frame.width_mode = AxisSizing::Hug;
        frame.children = vec![kid("a", 50.0, 20.0), kid("b", 70.0, 20.0)];
        frame.apply_layout();
        assert_eq!(frame.width, 145.0);
        // Fill: the second child takes the remaining inner width.
        let mut parent = Node::new("p", "P", NodeKind::Frame, 0.0, 0.0, 400.0, 100.0);
        parent.layout_direction = LayoutDirection::Row;
        parent.layout_padding = 10.0;
        let mut filler = kid("fill", 10.0, 20.0);
        filler.width_mode = AxisSizing::Fill;
        parent.children = vec![kid("fixed", 100.0, 20.0), filler];
        parent.apply_layout();
        assert_eq!(parent.children[1].width, 280.0);
        assert_eq!(parent.children[1].x, 110.0);
    }

    #[test]
    fn instance_flatten_with_overrides() {
        let mut master = Node::new("m-root", "Card", NodeKind::Card, 0.0, 0.0, 200.0, 100.0);
        master.fill = Some(Color::rgb(9, 9, 9));
        let mut label = Node::text_node(
            "m-text",
            "Label",
            8.0,
            8.0,
            100.0,
            20.0,
            "سلام",
            14.0,
            Color::WHITE,
        );
        label.id = "m-text".into();
        master.children.push(label);
        let mut project = Project::new("p", "P");
        project.components.push(ComponentDef {
            id: "comp-1".into(),
            name: "Card".into(),
            root: master,
        });
        let mut inst = Node::new("inst-1", "Card", NodeKind::Component, 30.0, 40.0, 200.0, 100.0);
        inst.component = Some("comp-1".into());
        inst.fill = Some(Color::rgb(200, 0, 0));
        inst.props
            .insert("override_text:m-text".into(), "خداحافظ".into());
        let resolved = project.resolve_node(&inst);
        assert_eq!(resolved.id, "inst-1");
        assert_eq!((resolved.x, resolved.y), (30.0, 40.0));
        assert_eq!(resolved.fill, Some(Color::rgb(200, 0, 0)));
        assert_eq!(resolved.children.len(), 1);
        assert_eq!(resolved.children[0].text.as_deref(), Some("خداحافظ"));
        assert!(resolved.children[0].id.starts_with("inst-1::"));
        assert!(resolved.component.is_none());
    }

    #[test]
    fn group_and_ungroup_tree_ops() {
        let mut root = Node::new("root", "Root", NodeKind::Frame, 0.0, 0.0, 800.0, 600.0);
        root.children = vec![
            Node::rect("a", "A", 10.0, 10.0, 100.0, 50.0, Color::WHITE),
            Node::rect("b", "B", 200.0, 20.0, 100.0, 50.0, Color::WHITE),
            Node::rect("c", "C", 500.0, 20.0, 100.0, 50.0, Color::WHITE),
        ];
        let ids: BTreeSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        let mut counter = 0u32;
        let created = root.group_children(&ids, "group", &mut counter);
        assert_eq!(created.len(), 1);
        assert_eq!(root.children.len(), 2);
        let group = &root.children[0];
        assert_eq!(group.kind, NodeKind::Group);
        assert_eq!(group.children.len(), 2);
        assert_eq!((group.x, group.y), (10.0, 10.0));
        assert_eq!(group.children[0].x, 0.0);
        // Ungroup restores the children at the group slot with
        // their original page positions back.
        let gids: BTreeSet<String> = created.iter().cloned().collect();
        let released = root.ungroup_children(&gids);
        assert_eq!(released.len(), 2);
        assert_eq!(root.children.len(), 3);
        assert_eq!((root.children[0].x, root.children[0].y), (10.0, 10.0));
        assert_eq!((root.children[1].x, root.children[1].y), (200.0, 20.0));
    }
}

