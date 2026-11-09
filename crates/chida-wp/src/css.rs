//! A small CSS reader for theme `style.css` files: the theme
//! header comment plus simple rules. Only the selector and property
//! subset the importer understands is kept; everything else is
//! parsed over and ignored.

use chida_core::Color;

/// The fields of the WordPress theme header comment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeHeader {
    pub name: String,
    pub uri: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub text_domain: String,
}

/// One part of a selector: `div`, `.card`, `div.card` or `#main`.
#[derive(Debug, Clone, PartialEq)]
pub enum SelPart {
    Tag(String),
    Class(String),
    TagClass(String, String),
    Id(String),
}

impl SelPart {
    fn parse(raw: &str) -> Option<Self> {
        let s = raw.trim();
        if s.is_empty() || s == "*" {
            return None;
        }
        // Pseudo classes and attribute selectors are out of scope.
        let s = s.split(&[':', '['][..]).next().unwrap_or(s).trim();
        if let Some(id) = s.strip_prefix('#') {
            return Some(Self::Id(id.to_string()));
        }
        if let Some(cls) = s.strip_prefix('.') {
            return Some(Self::Class(cls.to_string()));
        }
        if let Some((tag, cls)) = s.split_once('.') {
            if !tag.is_empty() && !cls.is_empty() {
                return Some(Self::TagClass(tag.to_lowercase(), cls.to_string()));
            }
        }
        Some(Self::Tag(s.to_lowercase()))
    }
    /// Specificity weight next to the other supported part kinds.
    fn score(&self) -> u32 {
        match self {
            Self::Id(_) => 100,
            Self::Class(_) => 10,
            Self::TagClass(..) => 11,
            Self::Tag(_) => 1,
        }
    }
    fn matches(&self, tag: &str, classes: &[String], id: Option<&str>) -> bool {
        match self {
            Self::Tag(t) => t == tag,
            Self::Class(c) => classes.iter().any(|x| x == c),
            Self::TagClass(t, c) => t == tag && classes.iter().any(|x| x == c),
            Self::Id(i) => id == Some(i.as_str()),
        }
    }
}

/// A selector: one part, or an ancestor part plus the element part
/// (a two part descendant selector, the most the importer supports).
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub ancestor: Option<SelPart>,
    pub own: SelPart,
}

impl Selector {
    fn parse(raw: &str) -> Option<Self> {
        let parts: Vec<&str> = raw.split_whitespace().collect();
        match parts.len() {
            1 => Some(Self {
                ancestor: None,
                own: SelPart::parse(parts[0])?,
            }),
            2 => Some(Self {
                ancestor: Some(SelPart::parse(parts[0])?),
                own: SelPart::parse(parts[1])?,
            }),
            _ => None,
        }
    }
    pub fn specificity(&self) -> u32 {
        self.own.score() + self.ancestor.as_ref().map(|a| a.score()).unwrap_or(0)
    }
    /// `ancestors` runs from the outermost element to the parent.
    pub fn matches(
        &self,
        tag: &str,
        classes: &[String],
        id: Option<&str>,
        ancestors: &[(String, Vec<String>, Option<String>)],
    ) -> bool {
        if !self.own.matches(tag, classes, id) {
            return false;
        }
        match &self.ancestor {
            None => true,
            Some(part) => ancestors
                .iter()
                .any(|(t, c, i)| part.matches(t, c, i.as_deref())),
        }
    }
}

/// The declarations the importer knows how to apply.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub align: Option<String>,
    pub radius: Option<f32>,
    pub padding: Option<f32>,
    pub margin_v: Option<f32>,
    pub flex: bool,
    /// Row direction for flex containers (the CSS default);
    /// `flex-direction: column` flips it to false.
    pub flex_row: bool,
    pub gap: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: None,
            background: None,
            font_size: None,
            font_weight: None,
            align: None,
            radius: None,
            padding: None,
            margin_v: None,
            flex: false,
            flex_row: true,
            gap: None,
            width: None,
            height: None,
        }
    }
}

impl Style {
    /// Apply one `property: value` declaration, keeping the subset
    /// the importer supports.
    pub fn apply_decl(&mut self, prop: &str, value: &str) {
        let value = value.trim();
        match prop.trim().to_lowercase().as_str() {
            "color" => self.color = parse_color(value).or(self.color),
            "background-color" => self.background = parse_color(value).or(self.background),
            "background" => {
                if let Some(c) = value.split_whitespace().find_map(parse_color) {
                    self.background = Some(c);
                }
            }
            "font-size" => self.font_size = parse_px(value).or(self.font_size),
            "font-weight" => {
                self.font_weight = parse_weight(value).or(self.font_weight);
            }
            "text-align" => {
                let v = value.to_lowercase();
                if ["left", "right", "center", "justify"].contains(&v.as_str()) {
                    self.align = Some(v);
                }
            }
            "border-radius" => {
                self.radius = value.split_whitespace().next().and_then(parse_px).or(self.radius)
            }
            "padding" => self.padding = parse_box_vertical(value).or(self.padding),
            "margin" => self.margin_v = parse_box_vertical(value).or(self.margin_v),
            "display" => {
                let v = value.to_lowercase();
                if v == "flex" || v == "inline-flex" {
                    self.flex = true;
                }
            }
            "flex-direction" => {
                self.flex_row = !value.to_lowercase().starts_with("column");
            }
            "gap" | "grid-gap" => {
                self.gap = value.split_whitespace().next().and_then(parse_px).or(self.gap)
            }
            "width" => self.width = parse_px(value).or(self.width),
            "height" => self.height = parse_px(value).or(self.height),
            _ => {}
        }
    }

}

/// One parsed CSS rule.
#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: Selector,
    pub decls: Vec<(String, String)>,
    pub order: usize,
}

/// A parsed stylesheet: header fields plus rules in source order.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub header: ThemeHeader,
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let header = parse_header(css);
        let body = strip_comments(css);
        let mut rules = Vec::new();
        let mut order = 0usize;
        for chunk in body.split('}') {
            let Some((sel_raw, decl_raw)) = chunk.split_once('{') else {
                continue;
            };
            let decls = parse_decls(decl_raw);
            if decls.is_empty() {
                continue;
            }
            for one in sel_raw.split(',') {
                if let Some(selector) = Selector::parse(one) {
                    rules.push(Rule {
                        selector,
                        decls: decls.clone(),
                        order,
                    });
                    order += 1;
                }
            }
        }
        Self { header, rules }
    }

    /// The resolved style of one element: matching rules ordered by
    /// specificity then source order, with the inline style last.
    pub fn resolve(
        &self,
        tag: &str,
        classes: &[String],
        id: Option<&str>,
        ancestors: &[(String, Vec<String>, Option<String>)],
        inline: Option<&str>,
    ) -> Style {
        let mut matched: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| r.selector.matches(tag, classes, id, ancestors))
            .collect();
        matched.sort_by_key(|r| (r.selector.specificity(), r.order));
        let mut style = Style::default();
        for rule in matched {
            for (prop, value) in &rule.decls {
                style.apply_decl(prop, value);
            }
        }
        if let Some(inline) = inline {
            for (prop, value) in parse_decls(inline) {
                style.apply_decl(&prop, &value);
            }
        }
        style
    }

    /// The merged style for a bare tag selector (used for `body`).
    pub fn tag_style(&self, tag: &str) -> Style {
        self.resolve(tag, &[], None, &[], None)
    }
}

fn parse_header(css: &str) -> ThemeHeader {
    let mut header = ThemeHeader::default();
    let Some(start) = css.find("/*") else {
        return header;
    };
    let Some(end) = css[start..].find("*/") else {
        return header;
    };
    let block = &css[start + 2..start + end];
    for line in block.lines() {
        let line = line.trim().trim_start_matches('*').trim();
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim().to_lowercase().as_str() {
            "theme name" => header.name = value,
            "theme uri" => header.uri = value,
            "author" => header.author = value,
            "description" => header.description = value,
            "version" => header.version = value,
            "text domain" => header.text_domain = value,
            _ => {}
        }
    }
    header
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start..].find("*/") {
            Some(end) => rest = &rest[start + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Split a declaration block (or an inline style attribute) into
/// property and value pairs.
pub fn parse_decls(raw: &str) -> Vec<(String, String)> {
    raw.split(';')
        .filter_map(|part| {
            let (k, v) = part.split_once(':')?;
            let k = k.trim().to_lowercase();
            let v = v.trim().to_string();
            if k.is_empty() || v.is_empty() {
                None
            } else {
                Some((k, v))
            }
        })
        .collect()
}

/// Parse a pixel length like `16px`, `1.5rem` (approximated at a
/// 16px root) or a bare number. Percentages return None.
pub fn parse_px(value: &str) -> Option<f32> {
    let v = value.trim().to_lowercase();
    if v.ends_with('%') || v == "auto" {
        return None;
    }
    if let Some(n) = v.strip_suffix("px") {
        return n.trim().parse().ok();
    }
    if let Some(n) = v.strip_suffix("rem") {
        return n.trim().parse::<f32>().ok().map(|f| f * 16.0);
    }
    if let Some(n) = v.strip_suffix("em") {
        return n.trim().parse::<f32>().ok().map(|f| f * 16.0);
    }
    if let Some(n) = v.strip_suffix("pt") {
        return n.trim().parse::<f32>().ok().map(|f| f * 1.333);
    }
    v.parse().ok()
}

fn parse_weight(value: &str) -> Option<u16> {
    match value.trim().to_lowercase().as_str() {
        "bold" | "bolder" => Some(700),
        "normal" => Some(400),
        "medium" => Some(500),
        "semibold" => Some(600),
        other => other.parse().ok(),
    }
}

/// The average of the vertical components of a box shorthand:
/// `10px`, `10px 20px`, `10px 20px 30px` or `1px 2px 3px 4px`.
fn parse_box_vertical(value: &str) -> Option<f32> {
    let vals: Vec<f32> = value.split_whitespace().filter_map(parse_px).collect();
    match vals.len() {
        1 => Some(vals[0]),
        2 | 3 => Some(vals[0]),
        4 => Some((vals[0] + vals[2]) / 2.0),
        _ => None,
    }
}

/// Parse a CSS colour: hex, rgb()/rgba() or a common colour name.
pub fn parse_color(value: &str) -> Option<Color> {
    let v = value.trim().to_lowercase();
    if v.starts_with('#') {
        return Color::from_hex(&v).ok();
    }
    if let Some(inner) = v.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let nums: Vec<f32> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if nums.len() == 3 {
            return Some(Color::rgb(nums[0] as u8, nums[1] as u8, nums[2] as u8));
        }
    }
    if let Some(inner) = v.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let nums: Vec<f32> = inner
            .split(',')
            .filter_map(|p| p.trim().parse().ok())
            .collect();
        if nums.len() == 4 {
            return Some(Color::rgba(
                nums[0] as u8,
                nums[1] as u8,
                nums[2] as u8,
                (nums[3] * 255.0) as u8,
            ));
        }
    }
    Some(match v.as_str() {
        "white" => Color::WHITE,
        "black" => Color::BLACK,
        "transparent" => Color::TRANSPARENT,
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "blue" => Color::rgb(0, 0, 255),
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "silver" => Color::rgb(192, 192, 192),
        "orange" => Color::rgb(255, 165, 0),
        "purple" => Color::rgb(128, 0, 128),
        "teal" => Color::rgb(0, 128, 128),
        "navy" => Color::rgb(0, 0, 128),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_rules() {
        let css = "/*\nTheme Name: Demo\nText Domain: demo-x\nVersion: 2.0\n*/\n\
                   body { color: #222; background: #fff }\n\
                   .card { border-radius: 8px; padding: 12px 20px; }\n\
                   .nav a { color: rgb(1, 2, 3); font-weight: bold }\n";
        let sheet = Stylesheet::parse(css);
        assert_eq!(sheet.header.name, "Demo");
        assert_eq!(sheet.header.text_domain, "demo-x");
        let card = sheet.resolve("div", &["card".to_string()], None, &[], None);
        assert_eq!(card.radius, Some(8.0));
        assert_eq!(card.padding, Some(12.0));
        let link = sheet.resolve(
            "a",
            &[],
            None,
            &[("div".into(), vec!["nav".into()], None)],
            None,
        );
        assert_eq!(link.color, Some(Color::rgb(1, 2, 3)));
        assert_eq!(link.font_weight, Some(700));
        let body = sheet.tag_style("body");
        assert_eq!(body.color, Some(Color::rgb(0x22, 0x22, 0x22)));
    }

    #[test]
    fn inline_beats_class_beats_tag() {
        let css = "p { font-size: 14px } .big { font-size: 20px }";
        let sheet = Stylesheet::parse(css);
        let s = sheet.resolve("p", &["big".into()], None, &[], Some("font-size: 26px"));
        assert_eq!(s.font_size, Some(26.0));
        let s2 = sheet.resolve("p", &["big".into()], None, &[], None);
        assert_eq!(s2.font_size, Some(20.0));
    }
}
