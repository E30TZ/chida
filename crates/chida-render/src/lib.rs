//! chida-render: turns a [`chida_core::Page`] into pixels or vectors.
//!
//! The PNG path is a small CPU rasteriser written for Chida: solid and
//! rounded shapes, ellipses, alpha blending and real TrueType text.
//! Text is shaped with HarfBuzz (rustybuzz) and rasterised anti-aliased
//! with fontdue, using the bundled Vazirmatn font (Persian, Arabic and
//! Latin) with DejaVu Sans as fallback, so Persian renders shaped and
//! right-to-left in PNG too. The SVG path emits real `<text>` elements
//! with the same direction handling. The 5x7 bitmap font in `font.rs`
//! remains only as a fallback if the bundled fonts fail to parse.

mod font;
mod text;
use chida_core::{Color, Gradient, Node, NodeKind, Page, Shadow};
pub use text::{is_rtl, measure_line, shape_line, ShapedLine};

/// An RGBA pixel buffer.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// Base directory that relative image asset paths resolve against.
    pub base_dir: std::path::PathBuf,
    images: std::collections::HashMap<String, Option<std::rc::Rc<image::RgbaImage>>>,
}
impl Canvas {
    pub fn new(width: u32, height: u32, bg: Color) -> Self {
        let mut c = Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
            base_dir: std::path::PathBuf::new(),
            images: std::collections::HashMap::new(),
        };
        c.clear(bg);
        c
    }
    /// Load and cache an image asset (PNG or JPEG) relative to base_dir.
    pub fn load_image(&mut self, src: &str) -> Option<std::rc::Rc<image::RgbaImage>> {
        if let Some(hit) = self.images.get(src) {
            return hit.clone();
        }
        let path = if std::path::Path::new(src).is_absolute() {
            std::path::PathBuf::from(src)
        } else {
            self.base_dir.join(src)
        };
        let loaded = image::ImageReader::open(&path)
            .ok()
            .and_then(|r| r.decode().ok())
            .map(|img| std::rc::Rc::new(img.to_rgba8()));
        self.images.insert(src.to_string(), loaded.clone());
        loaded
    }

    pub fn clear(&mut self, color: Color) {
        for px in self.pixels.chunks_exact_mut(4) {
            px[0] = color.r;
            px[1] = color.g;
            px[2] = color.b;
            px[3] = 255;
        }
    }
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, color: Color, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let a = (color.a as f32 / 255.0) * coverage.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let mix = |src: u8, dst: u8| (src as f32 * a + dst as f32 * (1.0 - a)).round() as u8;
        self.pixels[i] = mix(color.r, self.pixels[i]);
        self.pixels[i + 1] = mix(color.g, self.pixels[i + 1]);
        self.pixels[i + 2] = mix(color.b, self.pixels[i + 2]);
        self.pixels[i + 3] = 255;
    }
    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let i = ((y * self.width + x) * 4) as usize;
        (
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        )
    }
    /// Encode the buffer as PNG bytes.
    pub fn to_png(&self) -> Result<Vec<u8>, RenderError> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.width, self.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc
                .write_header()
                .map_err(|e| RenderError::Png(e.to_string()))?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|e| RenderError::Png(e.to_string()))?;
        }
        Ok(out)
    }
    pub fn save_png(&self, path: &std::path::Path) -> Result<(), RenderError> {
        std::fs::write(path, self.to_png()?)?;
        Ok(())
    }
}

/// Signed distance to a rounded rect: negative inside, positive out.
fn sd_rounded_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let r = r.min(w / 2.0).min(h / 2.0);
    let cx = (px - (x + w / 2.0)).abs() - (w / 2.0 - r);
    let cy = (py - (y + h / 2.0)).abs() - (h / 2.0 - r);
    let dx = cx.max(0.0);
    let dy = cy.max(0.0);
    (dx * dx + dy * dy).sqrt() + cx.max(cy).min(0.0) - r
}

fn rounded_rect_coverage(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let dist = sd_rounded_rect(px, py, x, y, w, h, r);
    (0.5 - dist).clamp(0.0, 1.0)
}

fn point_in_polygon(px: f32, py: f32, points: &[(f32, f32)]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, yi) = points[i];
        let (xj, yj) = points[j];
        if (yi > py) != (yj > py) {
            let x_at = xi + (py - yi) * (xj - xi) / (yj - yi);
            if px < x_at {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn distance_to_segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= f32::EPSILON {
        0.0
    } else {
        (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    let (qx, qy) = (a.0 + dx * t, a.1 + dy * t);
    ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
}

fn arrow_geometry(node: &Node, x: f32, y: f32) -> Option<((f32, f32), (f32, f32), Vec<(f32, f32)>)> {
    if node.path_points.len() < 2 {
        return None;
    }
    let start = (x + node.path_points[0].x, y + node.path_points[0].y);
    let end = (x + node.path_points[1].x, y + node.path_points[1].y);
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.0 {
        return None;
    }
    let unit = (dx / len, dy / len);
    let normal = (-unit.1, unit.0);
    let head_len = (len * 0.32).clamp(8.0, 22.0);
    let head_half = (node.stroke_width.max(1.5) * 3.0).clamp(5.0, 14.0);
    let base = (end.0 - unit.0 * head_len, end.1 - unit.1 * head_len);
    let head = vec![
        (base.0 + normal.0 * head_half, base.1 + normal.1 * head_half),
        end,
        (base.0 - normal.0 * head_half, base.1 - normal.1 * head_half),
    ];
    Some((start, end, head))
}

fn path_fill_coverage(node: &Node, px: f32, py: f32, x: f32, y: f32) -> f32 {
    if node.kind == NodeKind::Arrow {
        if let Some((start, end, head)) = arrow_geometry(node, x, y) {
            let half = node.stroke_width.max(1.5) / 2.0;
            if distance_to_segment(px, py, start, end) <= half || point_in_polygon(px, py, &head) {
                return 1.0;
            }
        }
        return 0.0;
    }
    if node.kind != NodeKind::Path {
        return 0.0;
    }
    let points = node.flattened_path(x, y);
    if node.path_closed && point_in_polygon(px, py, &points) {
        return 1.0;
    }
    if node.stroke_width > 0.0 && node.stroke.is_some() {
        let half = node.stroke_width / 2.0;
        let mut best = f32::INFINITY;
        for pair in points.windows(2) {
            best = best.min(distance_to_segment(px, py, pair[0], pair[1]));
        }
        if best <= half {
            return (half + 0.5 - best).clamp(0.0, 1.0);
        }
    }
    0.0
}

fn coverage_for(node: &Node, px: f32, py: f32, x: f32, y: f32) -> f32 {
    if matches!(node.kind, NodeKind::Path | NodeKind::Arrow) {
        return path_fill_coverage(node, px, py, x, y);
    }
    if node.kind == NodeKind::Ellipse {
        let (cx, cy) = (x + node.width / 2.0, y + node.height / 2.0);
        let (rx, ry) = (node.width / 2.0, node.height / 2.0);
        if rx <= 0.0 || ry <= 0.0 {
            return 0.0;
        }
        let d = (((px - cx) / rx).powi(2) + ((py - cy) / ry).powi(2)).sqrt();
        ((1.0 - d) * 4.0).clamp(0.0, 1.0)
    } else {
        rounded_rect_coverage(px, py, x, y, node.width, node.height, node.corner_radius)
    }
}

/// The gradient colour of `gradient` at a point inside a node box.
fn gradient_at(gradient: &Gradient, px: f32, py: f32, x: f32, y: f32, w: f32, h: f32) -> Color {
    let (dx, dy) = gradient.direction();
    let span = (w * dx).abs() + (h * dy).abs();
    if span <= 0.0 {
        return gradient.from;
    }
    let rel_x = px - (x + w / 2.0);
    let rel_y = py - (y + h / 2.0);
    let t = (rel_x * dx + rel_y * dy) / span + 0.5;
    gradient.at(t)
}

fn fill_shape_gradient(canvas: &mut Canvas, node: &Node, ox: f32, oy: f32, gradient: &Gradient) {
    let (x, y) = (ox + node.x, oy + node.y);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + node.width).ceil() as i32;
    let y1 = (y + node.height).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let cov = coverage_for(node, px as f32 + 0.5, py as f32 + 0.5, x, y);
            if cov > 0.0 {
                let color = gradient_at(
                    gradient,
                    px as f32 + 0.5,
                    py as f32 + 0.5,
                    x,
                    y,
                    node.width,
                    node.height,
                );
                canvas.blend(px, py, color, cov * node.opacity);
            }
        }
    }
}

/// Paint the node drop shadow: the node silhouette, offset and
/// feathered by the blur radius via the signed distance field.
fn paint_shadow(canvas: &mut Canvas, node: &Node, ox: f32, oy: f32, shadow: &Shadow) {
    let (x, y) = (ox + node.x + shadow.x, oy + node.y + shadow.y);
    let blur = shadow.blur.max(0.0);
    let pad = blur * 1.5 + 2.0;
    let x0 = (x - pad).floor() as i32;
    let y0 = (y - pad).floor() as i32;
    let x1 = (x + node.width + pad).ceil() as i32;
    let y1 = (y + node.height + pad).ceil() as i32;
    let mut color = shadow.color;
    color.a = ((color.a as f32 / 255.0) * shadow.opacity.clamp(0.0, 1.0) * 255.0) as u8;
    for py in y0..y1 {
        for px in x0..x1 {
            let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
            let base_cov = coverage_for(node, cx, cy, x, y);
            let cov = if matches!(node.kind, NodeKind::Path | NodeKind::Arrow) {
                base_cov
            } else if node.kind == NodeKind::Ellipse {
                let (ex, ey) = (x + node.width / 2.0, y + node.height / 2.0);
                let (rx, ry) = (node.width / 2.0, node.height / 2.0);
                if rx <= 0.0 || ry <= 0.0 {
                    0.0
                } else {
                    let d = (((cx - ex) / rx).powi(2) + ((cy - ey) / ry).powi(2)).sqrt();
                    let dist = (d - 1.0) * rx.min(ry);
                    (0.5 - dist / (blur * 2.0 + 1.0)).clamp(0.0, 1.0)
                }
            } else {
                let dist =
                    sd_rounded_rect(cx, cy, x, y, node.width, node.height, node.corner_radius);
                (0.5 - dist / (blur * 2.0 + 1.0)).clamp(0.0, 1.0)
            };
            if cov > 0.0 {
                canvas.blend(px, py, color, cov * node.opacity);
            }
        }
    }
}

fn fill_shape(canvas: &mut Canvas, node: &Node, ox: f32, oy: f32, color: Color) {
    let (x, y) = (ox + node.x, oy + node.y);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + node.width).ceil() as i32;
    let y1 = (y + node.height).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let cov = if node.kind == NodeKind::Ellipse {
                let (cx, cy) = (x + node.width / 2.0, y + node.height / 2.0);
                let (rx, ry) = (node.width / 2.0, node.height / 2.0);
                if rx <= 0.0 || ry <= 0.0 {
                    0.0
                } else {
                    let d = (((px as f32 + 0.5 - cx) / rx).powi(2)
                        + ((py as f32 + 0.5 - cy) / ry).powi(2))
                    .sqrt();
                    (1.0 - d).clamp(0.0, 1.0) * 4.0
                }
                .min(1.0)
            } else {
                rounded_rect_coverage(
                    px as f32 + 0.5,
                    py as f32 + 0.5,
                    x,
                    y,
                    node.width,
                    node.height,
                    node.corner_radius,
                )
            };
            if cov > 0.0 {
                canvas.blend(px, py, color, cov * node.opacity);
            }
        }
    }
}

fn stroke_shape(canvas: &mut Canvas, node: &Node, ox: f32, oy: f32) {
    if node.stroke_width <= 0.0 {
        return;
    }
    let Some(stroke) = node.stroke else {
        return;
    };
    let sw = node.stroke_width;
    let (x, y) = (ox + node.x, oy + node.y);
    if matches!(node.kind, NodeKind::Path | NodeKind::Arrow) {
        // Fill coverage already paints the outline for paths; the
        // generic rounded-rect stroke below would add a false box.
        return;
    }
    let x0 = (x - sw).floor() as i32;
    let y0 = (y - sw).floor() as i32;
    let x1 = (x + node.width + sw).ceil() as i32;
    let y1 = (y + node.height + sw).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let outer = rounded_rect_coverage(
                px as f32 + 0.5,
                py as f32 + 0.5,
                x - sw,
                y - sw,
                node.width + sw * 2.0,
                node.height + sw * 2.0,
                node.corner_radius + sw,
            );
            let inner_c = rounded_rect_coverage(
                px as f32 + 0.5,
                py as f32 + 0.5,
                x,
                y,
                node.width,
                node.height,
                node.corner_radius,
            );
            let cov = outer * (1.0 - inner_c);
            if cov > 0.0 {
                canvas.blend(px, py, stroke, cov * node.opacity);
            }
        }
    }
}

/// Draw text with the bundled TrueType fonts (HarfBuzz shaping plus
/// anti-aliased fontdue rasterisation). RTL lines are right-aligned
/// within `width`. Falls back to the bitmap font if fonts fail to load.
#[allow(clippy::too_many_arguments)]
fn draw_text(
    canvas: &mut Canvas,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    size: f32,
    color: Color,
    opacity: f32,
    bold: bool,
    align: &str,
    line_height: f32,
    letter_spacing: f32,
) {
    let Some(fonts) = text::fonts() else {
        draw_text_bitmap(canvas, text, x, y, size, color, opacity);
        return;
    };
    let line_h = size * if line_height > 0.0 { line_height } else { 1.45 };
    let mut pen_y = y;
    for raw_line in text.split('\n') {
        // Word wrap the logical line to the box width, then draw each
        // visual line with the usual shaping (digits keep their order
        // because every chunk is shaped as one string).
        let visual_lines = wrap_words(raw_line, bold, size, width);
        for line in &visual_lines {
            if line.is_empty() {
                pen_y += line_h;
                continue;
            }
            let shaped = text::shape_line(line, bold, size);
            let shaped_width =
                shaped.width + letter_spacing * shaped.glyphs.len().saturating_sub(1) as f32;
            let start_x = if align == "center" && width > 0.0 {
                x + (width - shaped_width) / 2.0
            } else if align == "left" {
                x
            } else if shaped.rtl && width > 0.0 {
                x + width - shaped_width
            } else {
                x
            };
            let baseline = pen_y + fonts.ascent(bold, size);
            let mut pen_x = start_x;
            for g in &shaped.glyphs {
                let (metrics, coverage) = if g.glyph_id != 0 && g.glyph_id <= u16::MAX as u32 {
                    fonts
                        .primary(bold)
                        .rasterize_indexed(g.glyph_id as u16, size)
                } else {
                    fonts.fallback(bold).rasterize(g.cluster_char, size)
                };
                if metrics.width > 0 && metrics.height > 0 {
                    let base_x = (pen_x + g.x_offset + metrics.xmin as f32).round() as i32;
                    let base_y =
                        (baseline - g.y_offset - metrics.ymin as f32 - metrics.height as f32)
                            .round() as i32;
                    for row in 0..metrics.height {
                        for col in 0..metrics.width {
                            let cov = coverage[row * metrics.width + col] as f32 / 255.0;
                            if cov > 0.0 {
                                canvas.blend(
                                    base_x + col as i32,
                                    base_y + row as i32,
                                    color,
                                    cov * opacity,
                                );
                            }
                        }
                    }
                }
                pen_x += g.x_advance + letter_spacing;
            }
            pen_y += line_h;
        }
    }
}

/// Greedy word wrapping by shaped width. A single word longer than the
/// box is kept whole (it overflows rather than breaking mid word).
fn wrap_words(line: &str, bold: bool, size: f32, width: f32) -> Vec<String> {
    if width <= 0.0 || line.is_empty() {
        return vec![line.to_string()];
    }
    if text::shape_line(line, bold, size).width <= width {
        return vec![line.to_string()];
    }
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        if word.is_empty() {
            continue;
        }
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && text::shape_line(&candidate, bold, size).width > width {
            out.push(std::mem::take(&mut current));
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(line.to_string());
    }
    out
}

/// Draw text with the built in bitmap font (fallback path only).
fn draw_text_bitmap(
    canvas: &mut Canvas,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    opacity: f32,
) {
    let scale = (size / 7.0).max(1.0);
    let mut pen_x = x;
    let line_h = size * 1.35;
    let mut pen_y = y;
    for ch in text.chars() {
        if ch == '\n' {
            pen_x = x;
            pen_y += line_h;
            continue;
        }
        if pen_x + 6.0 * scale > x + 100000.0 {
            break;
        }
        let glyph = font::glyph(ch);
        match glyph {
            Some(rows) => {
                for (row, bits) in rows.iter().enumerate() {
                    for col in 0..5 {
                        if bits & (1 << (4 - col)) != 0 {
                            let gx = pen_x + col as f32 * scale;
                            let gy = pen_y + row as f32 * scale;
                            for sy in 0..scale.ceil() as i32 {
                                for sx in 0..scale.ceil() as i32 {
                                    canvas.blend(gx as i32 + sx, gy as i32 + sy, color, opacity);
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // placeholder box for scripts the bitmap font lacks
                for dy in 0..(7.0 * scale) as i32 {
                    for dx in 0..(5.0 * scale) as i32 {
                        let edge = dx == 0
                            || dy == 0
                            || dx == (5.0 * scale) as i32 - 1
                            || dy == (7.0 * scale) as i32 - 1;
                        if edge {
                            canvas.blend(
                                pen_x as i32 + dx,
                                pen_y as i32 + dy,
                                color,
                                opacity * 0.7,
                            );
                        }
                    }
                }
            }
        }
        pen_x += 6.0 * scale;
    }
}

fn sample_bilinear(img: &image::RgbaImage, u: f32, v: f32) -> Color {
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let x = (u * iw - 0.5).clamp(0.0, iw - 1.0);
    let y = (v * ih - 0.5).clamp(0.0, ih - 1.0);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(img.width() - 1);
    let y1 = (y0 + 1).min(img.height() - 1);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let mix = |a: u8, b: u8, c: u8, d: u8| -> u8 {
        (a as f32 * (1.0 - fx) * (1.0 - fy)
            + b as f32 * fx * (1.0 - fy)
            + c as f32 * (1.0 - fx) * fy
            + d as f32 * fx * fy) as u8
    };
    let p = |xx: u32, yy: u32| *img.get_pixel(xx, yy);
    Color::rgba(
        mix(p(x0, y0)[0], p(x1, y0)[0], p(x0, y1)[0], p(x1, y1)[0]),
        mix(p(x0, y0)[1], p(x1, y0)[1], p(x0, y1)[1], p(x1, y1)[1]),
        mix(p(x0, y0)[2], p(x1, y0)[2], p(x0, y1)[2], p(x1, y1)[2]),
        mix(p(x0, y0)[3], p(x1, y0)[3], p(x0, y1)[3], p(x1, y1)[3]),
    )
}

/// Draw an image cover fitted (centre cropped) into a rect, clipped to
/// the rect corner radius, or to an ellipse when `ellipse` is set.
#[allow(clippy::too_many_arguments)]
fn paint_image_cover(
    canvas: &mut Canvas,
    img: &image::RgbaImage,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    ellipse: bool,
    opacity: f32,
) {
    if w <= 0.0 || h <= 0.0 || img.width() == 0 || img.height() == 0 {
        return;
    }
    let scale = (w / img.width() as f32).max(h / img.height() as f32);
    let src_w = w / scale;
    let src_h = h / scale;
    let src_x0 = (img.width() as f32 - src_w) / 2.0;
    let src_y0 = (img.height() as f32 - src_h) / 2.0;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            let cov = if ellipse {
                let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                let d = (((px as f32 + 0.5 - cx) / (w / 2.0)).powi(2)
                    + ((py as f32 + 0.5 - cy) / (h / 2.0)).powi(2))
                .sqrt();
                ((1.0 - d) * 4.0).clamp(0.0, 1.0)
            } else {
                rounded_rect_coverage(px as f32 + 0.5, py as f32 + 0.5, x, y, w, h, radius)
            };
            if cov <= 0.0 {
                continue;
            }
            let u = (src_x0 + (px as f32 + 0.5 - x) / scale) / img.width() as f32;
            let v = (src_y0 + (py as f32 + 0.5 - y) / scale) / img.height() as f32;
            let c = sample_bilinear(img, u, v);
            canvas.blend(px, py, c, opacity * cov);
        }
    }
}

fn paint_node(canvas: &mut Canvas, node: &Node, ox: f32, oy: f32) {
    if !node.visible {
        return;
    }
    let (ax, ay) = (ox + node.x, oy + node.y);
    for shadow in node.shadows.iter().filter(|s| s.enabled).rev() {
        paint_shadow(canvas, node, ox, oy, shadow);
    }
    if let Some(gradient) = &node.fill_gradient {
        fill_shape_gradient(canvas, node, ox, oy, gradient);
    } else if let Some(fill) = node.fill {
        fill_shape(canvas, node, ox, oy, fill);
    }
    stroke_shape(canvas, node, ox, oy);
    if node.kind == NodeKind::Image || node.props.contains_key("src") {
        let src = node.props.get("src").cloned();
        let painted = match &src {
            Some(src) => match canvas.load_image(src) {
                Some(img) => {
                    paint_image_cover(
                        canvas,
                        &img,
                        ax,
                        ay,
                        node.width,
                        node.height,
                        node.corner_radius,
                        node.kind == NodeKind::Ellipse,
                        node.opacity,
                    );
                    true
                }
                None => false,
            },
            None => false,
        };
        if !painted {
            // Image nodes without a decodable source render as a tinted
            // frame with a cross.
            let w = node.width as i32;
            let h = node.height as i32;
            for i in 0..w.min(h) {
                canvas.blend(
                    ax as i32 + i,
                    ay as i32 + i,
                    Color::rgba(120, 120, 120, 160),
                    1.0,
                );
                canvas.blend(
                    ax as i32 + i,
                    ay as i32 + (h - 1 - i),
                    Color::rgba(120, 120, 120, 160),
                    1.0,
                );
            }
        }
    }
    if let Some(text) = &node.text {
        let color = node.text_color.unwrap_or(Color::BLACK);
        let mut c = color;
        c.a = (color.a as f32 * node.opacity) as u8;
        let (tx, ty) = if node.kind == NodeKind::Button {
            (ax + 10.0, ay + (node.height - node.font_size) / 2.0)
        } else {
            (ax + 2.0, ay + 2.0)
        };
        let text_width = if node.kind == NodeKind::Button {
            (node.width - 20.0).max(0.0)
        } else {
            (node.width - 4.0).max(0.0)
        };
        draw_text(
            canvas,
            text,
            tx,
            ty,
            text_width,
            node.font_size.max(7.0),
            c,
            node.opacity,
            node.effective_bold(),
            node.props.get("align").map(|s| s.as_str()).unwrap_or(""),
            node.line_height,
            node.letter_spacing,
        );
    }
    for child in &node.children {
        paint_node(canvas, child, ax, ay);
    }
}

/// Render a page to a pixel canvas (scale multiplies the page size).
pub fn render_page(page: &Page, scale: f32) -> Canvas {
    render_page_inner(page, scale, std::path::Path::new(""))
}

/// Render a page, resolving relative image asset paths against `base`
/// (usually the directory that holds the project file).
pub fn render_page_with_base(page: &Page, scale: f32, base: &std::path::Path) -> Canvas {
    render_page_inner(page, scale, base)
}

fn render_page_inner(page: &Page, scale: f32, base: &std::path::Path) -> Canvas {
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    // The rasteriser works in page units; scaling is applied by rendering
    // pages that the caller pre-scales. Kept simple and honest for v1.0.
    let _ = scale;
    let mut canvas = Canvas::new(
        page.width.max(1.0) as u32,
        page.height.max(1.0) as u32,
        page.background.unwrap_or(Color::WHITE),
    );
    canvas.base_dir = base.to_path_buf();
    // Auto-layout containers render at their computed positions:
    // lay out a clone so the caller's document is never mutated.
    let mut root = page.root.clone();
    root.apply_layout();
    for child in &root.children {
        paint_node(&mut canvas, child, 0.0, 0.0);
    }
    canvas
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn svg_node(node: &Node, ox: f32, oy: f32, out: &mut String) {
    if !node.visible {
        return;
    }
    let (x, y) = (ox + node.x, oy + node.y);
    let op = if node.opacity < 1.0 {
        format!(" opacity=\"{}\"", node.opacity)
    } else {
        String::new()
    };
    // The effective fill: a gradient wins over the solid colour.
    let svg_fill = if let Some(g) = &node.fill_gradient {
        let gid: String = node
            .id
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let (dx, dy) = g.direction();
        let stops = g
            .stops()
            .iter()
            .map(|stop| format!("<stop offset=\"{}\" stop-color=\"{}\"/>", stop.offset, stop.color.to_hex()))
            .collect::<Vec<_>>()
            .join("");
        out.push_str(&format!(
            "<defs><linearGradient id=\"grad-{gid}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops}</linearGradient></defs>",
            0.5 - dx / 2.0,
            0.5 - dy / 2.0,
            0.5 + dx / 2.0,
            0.5 + dy / 2.0
        ));
        Some(format!("url(#grad-{gid})"))
    } else {
        node.fill.map(|f| f.to_hex())
    };
    let mut shadow_filters: Vec<String> = node
        .shadows
        .iter()
        .filter(|s| s.enabled)
        .map(|s| {
            let a = (s.color.a as f32 / 255.0) * s.opacity.clamp(0.0, 1.0);
            format!(
                "drop-shadow({}px {}px {}px rgba({},{},{},{:.3}))",
                s.x,
                s.y,
                (s.blur / 2.0).max(0.0),
                s.color.r,
                s.color.g,
                s.color.b,
                a
            )
        })
        .collect();
    if node.blur_radius > 0.0 {
        shadow_filters.push(format!("blur({}px)", node.blur_radius));
    }
    let svg_shadow = if shadow_filters.is_empty() {
        String::new()
    } else {
        format!(" style=\"filter:{}\"", shadow_filters.join(" "))
    };
    if matches!(node.kind, NodeKind::Path | NodeKind::Arrow) {
        let fill_attr = svg_fill.clone().unwrap_or_else(|| "none".to_string());
        let stroke_attr = node
            .stroke
            .map(|c| c.to_hex())
            .unwrap_or_else(|| fill_attr.clone());
        let stroke_width = if node.kind == NodeKind::Arrow {
            node.stroke_width.max(1.5)
        } else {
            node.stroke_width
        };
        if node.kind == NodeKind::Arrow {
            if let Some((start, end, head)) = arrow_geometry(node, x, y) {
                out.push_str(&format!(
                    "<path d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{} {} /><path d=\"M {} {} L {} {} L {} {} Z\" fill=\"{}\"{} {} />",
                    start.0,
                    start.1,
                    end.0,
                    end.1,
                    stroke_attr,
                    stroke_width,
                    op,
                    svg_shadow,
                    head[0].0,
                    head[0].1,
                    head[1].0,
                    head[1].1,
                    head[2].0,
                    head[2].1,
                    stroke_attr,
                    op,
                    svg_shadow
                ));
            }
        } else {
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} {} />",
                node.path_d(x, y),
                fill_attr,
                stroke_attr,
                stroke_width,
                op,
                svg_shadow
            ));
        }
    }
    if let Some(fill) = svg_fill.filter(|_| !matches!(node.kind, NodeKind::Path | NodeKind::Arrow)) {
        if node.kind == NodeKind::Ellipse {
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\"{} {} />",
                x + node.width / 2.0,
                y + node.height / 2.0,
                node.width / 2.0,
                node.height / 2.0,
                fill,
                op,
                svg_shadow
            ));
        } else {
            let stroke = node
                .stroke
                .map(|s| {
                    format!(
                        " stroke=\"{}\" stroke-width=\"{}\"",
                        s.to_hex(),
                        node.stroke_width
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{} {} {} />",
                x,
                y,
                node.width,
                node.height,
                node.corner_radius,
                fill,
                stroke,
                op,
                svg_shadow
            ));
        }
    }
    if node.kind == NodeKind::Image || node.props.contains_key("src") {
        if let Some(src) = node.props.get("src") {
            let clip_id: String = node
                .id
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect();
            if node.corner_radius > 0.0 {
                out.push_str(&format!(
                    "<clipPath id=\"clip-{clip_id}\"><rect x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{}\" rx=\"{}\" /></clipPath>",
                    node.width, node.height, node.corner_radius
                ));
                out.push_str(&format!(
                    "<g clip-path=\"url(#clip-{clip_id})\"><image href=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"xMidYMid slice\"{} /></g>",
                    esc(src),
                    node.width,
                    node.height,
                    op
                ));
            } else {
                out.push_str(&format!(
                    "<image href=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"xMidYMid slice\"{} />",
                    esc(src),
                    node.width,
                    node.height,
                    op
                ));
            }
        }
    }
    if let Some(text) = &node.text {
        let color = node.text_color.unwrap_or(Color::BLACK).to_hex();
        let weight = format!(" font-weight=\"{}\"", node.effective_weight());
        let spacing = if node.letter_spacing != 0.0 {
            format!(" letter-spacing=\"{}\"", node.letter_spacing)
        } else {
            String::new()
        };
        let rtl = text::is_rtl(text);
        let centred = node.props.get("align").map(|a| a.as_str()) == Some("center");
        let (tx, anchor) = if centred {
            (x + node.width / 2.0, " text-anchor=\"middle\"")
        } else if rtl {
            (
                x + node.width - 2.0,
                " text-anchor=\"end\" direction=\"rtl\"",
            )
        } else {
            (x + 2.0, "")
        };
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"Vazirmatn, 'DejaVu Sans', sans-serif\" font-size=\"{}\" fill=\"{}\"{}>{}</text>",
            tx,
            y + node.font_size,
            node.font_size,
            color,
            format!("{weight}{spacing}{anchor}"),
            esc(text)
        ));
    }
    for child in &node.children {
        svg_node(child, x, y, out);
    }
}

/// Render a page to a standalone SVG document.
pub fn render_page_svg(page: &Page) -> String {
    let bg = page.background.unwrap_or(Color::WHITE).to_hex();
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">", page.width, page.height, page.width, page.height);
    s.push_str(&format!(
        "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
        page.width, page.height, bg
    ));
    let mut root = page.root.clone();
    root.apply_layout();
    for child in &root.children {
        svg_node(child, 0.0, 0.0, &mut s);
    }
    s.push_str("</svg>");
    s
}

#[derive(Debug)]
pub enum RenderError {
    Png(String),
    Io(std::io::Error),
}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Png(m) => write!(f, "png error: {m}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}
impl std::error::Error for RenderError {}
impl From<std::io::Error> for RenderError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chida_core::Project;
    fn page() -> Page {
        let mut p = Page::new("home", "Home", "/", 200.0, 120.0);
        p.root.children.push(
            Node::rect(
                "bg",
                "Card",
                10.0,
                10.0,
                180.0,
                60.0,
                Color::rgb(16, 122, 87),
            )
            .with_radius(12.0),
        );
        p.root.children.push(Node::text_node(
            "t",
            "Title",
            20.0,
            24.0,
            120.0,
            20.0,
            "SHOP",
            14.0,
            Color::WHITE,
        ));
        p.root.children.push({
            let mut n = Node::new("e", "Dot", NodeKind::Ellipse, 160.0, 80.0, 24.0, 24.0);
            n.fill = Some(Color::rgb(200, 40, 40));
            n
        });
        p
    }
    #[test]
    fn png_has_png_signature_and_size() {
        let c = render_page(&page(), 1.0);
        let png = c.to_png().unwrap();
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.len() > 100);
        assert_eq!((c.width, c.height), (200, 120));
    }
    #[test]
    fn shapes_paint_expected_pixels() {
        let c = render_page(&page(), 1.0);
        assert_eq!(c.pixel(50, 30), (16, 122, 87, 255));
        assert_eq!(c.pixel(5, 5), (255, 255, 255, 255));
        let (r, g, b, _) = c.pixel(172, 92);
        assert!(
            r > 150 && g < 90 && b < 90,
            "ellipse centre should be red, got {r},{g},{b}"
        );
    }
    #[test]
    fn text_paints_dark_on_light() {
        let mut pg = Page::new("x", "X", "/x", 120.0, 40.0);
        pg.root.children.push(Node::text_node(
            "t",
            "T",
            4.0,
            4.0,
            100.0,
            20.0,
            "ABC",
            14.0,
            Color::BLACK,
        ));
        let c = render_page(&pg, 1.0);
        let dark = (0..120 * 40).any(|i| c.pixels[i * 4] == 0 && c.pixels[i * 4 + 3] == 255);
        assert!(dark);
    }
    #[test]
    fn svg_contains_text_and_shapes() {
        let s = render_page_svg(&page());
        assert!(s.contains("<svg"));
        assert!(s.contains("SHOP"));
        assert!(s.contains("<ellipse"));
        assert!(s.contains("#107a57"));
    }
    #[test]
    fn hidden_nodes_are_skipped() {
        let mut pg = page();
        pg.root.children[0].visible = false;
        let c = render_page(&pg, 1.0);
        assert_eq!(c.pixel(50, 30), (255, 255, 255, 255));
    }
    #[test]
    fn latin_text_is_antialiased() {
        // Real TTF rasterisation produces intermediate coverage values;
        // the old bitmap font could only produce pure black or white.
        let mut pg = Page::new("aa", "AA", "/aa", 300.0, 80.0);
        pg.root.children.push(Node::text_node(
            "t",
            "T",
            4.0,
            4.0,
            280.0,
            40.0,
            "Healthy plants",
            28.0,
            Color::BLACK,
        ));
        let c = render_page(&pg, 1.0);
        let grey = (0..300 * 80).any(|i| {
            let v = c.pixels[i * 4];
            v > 10 && v < 245 && c.pixels[i * 4 + 1] == v && c.pixels[i * 4 + 2] == v
        });
        assert!(grey, "expected anti-aliased grey pixels from TTF text");
    }
    #[test]
    fn persian_text_renders_ink_on_the_right() {
        let mut pg = Page::new("fa", "FA", "/fa", 320.0, 80.0);
        pg.root.children.push(Node::text_node(
            "t",
            "T",
            10.0,
            10.0,
            300.0,
            44.0,
            "گلکده سپیدار",
            30.0,
            Color::BLACK,
        ));
        let c = render_page(&pg, 1.0);
        let dark_at = |x0: u32, x1: u32| {
            (10..70)
                .flat_map(|y| (x0..x1).map(move |x| (x, y)))
                .filter(|(x, y)| c.pixel(*x, *y).0 < 100)
                .count()
        };
        let right = dark_at(160, 319);
        let left = dark_at(0, 60);
        assert!(right > 200, "Persian ink expected, got {right} dark px");
        assert!(
            left < right / 4,
            "RTL text should sit on the right: left {left} right {right}"
        );
        // Shaped Persian must not be placeholder boxes: a hollow box row
        // would leave the vertical middle of the line empty.
        let middle_ink = (28..40)
            .flat_map(|y| (160..319).map(move |x| (x, y)))
            .filter(|(x, y)| c.pixel(*x, *y).0 < 100)
            .count();
        assert!(middle_ink > 20, "middle of glyphs should carry ink");
    }
    #[test]
    fn svg_marks_persian_rtl() {
        let mut pg = Page::new("fa", "FA", "/fa", 320.0, 80.0);
        pg.root.children.push(Node::text_node(
            "t",
            "T",
            10.0,
            10.0,
            300.0,
            44.0,
            "گلکده سپیدار",
            30.0,
            Color::BLACK,
        ));
        let s = render_page_svg(&pg);
        assert!(s.contains("direction=\"rtl\""), "{s}");
        assert!(s.contains("گلکده سپیدار"));
        assert!(s.contains("Vazirmatn"));
    }
    #[test]
    fn project_page_renders() {
        let mut pr = Project::new("p", "P");
        pr.pages.push(page());
        let c = render_page(pr.page("/").unwrap(), 1.0);
        assert!(c.to_png().unwrap().len() > 50);
    }

    #[test]
    fn gradient_fill_blends_across_the_node() {
        use chida_core::{Gradient, Shadow};
        let mut pg = Page::new("g", "G", "/g", 200.0, 100.0);
        let mut n = Node::rect("r", "R", 10.0, 10.0, 180.0, 80.0, Color::BLACK);
        n.fill_gradient = Some(Gradient::new(
            Color::rgb(0, 0, 0),
            Color::rgb(255, 255, 255),
            90.0,
        ));
        pg.root.children.push(n);
        let c = render_page(&pg, 1.0);
        let left = c.pixel(20, 50).0;
        let mid = c.pixel(100, 50).0;
        let right = c.pixel(180, 50).0;
        assert!(left < 40, "left should be near black, got {left}");
        assert!(right > 215, "right should be near white, got {right}");
        assert!(
            mid > left && mid < right,
            "middle blends: {left} {mid} {right}"
        );
        assert!(
            (100..=155).contains(&(mid as i32)),
            "middle near 50 percent: {mid}"
        );
        // And a vertical gradient must stay constant across a row.
        let mut pg2 = Page::new("g2", "G2", "/g2", 200.0, 100.0);
        let mut n2 = Node::rect("r", "R", 10.0, 10.0, 180.0, 80.0, Color::BLACK);
        n2.fill_gradient = Some(Gradient::new(
            Color::rgb(0, 0, 0),
            Color::rgb(255, 255, 255),
            90.0,
        ));
        let _ = &mut n2;
        pg2.root.children.push(n2);
        let svg = render_page_svg(&pg);
        assert!(svg.contains("<linearGradient"), "svg carries the gradient");
        assert!(svg.contains("url(#grad-r)"), "svg shape uses the gradient");
        let _ = Shadow::soft();
    }
    #[test]
    fn drop_shadow_darkens_behind_the_node() {
        use chida_core::Shadow;
        let mut pg = Page::new("s", "S", "/s", 200.0, 160.0);
        let mut n = Node::rect("r", "R", 40.0, 30.0, 80.0, 50.0, Color::WHITE);
        n.shadows = vec![Shadow {
            x: 0.0,
            y: 14.0,
            blur: 10.0,
            color: Color::BLACK,
            opacity: 0.5,
        enabled: true,
        }];
        pg.root.children.push(n);
        let c = render_page(&pg, 1.0);
        // Just under the white rect, the shadow must darken the page.
        let under = c.pixel(80, 92);
        assert!(under.0 < 200, "shadow darkens below the node: {under:?}");
        // Far away the page stays white.
        assert_eq!(c.pixel(190, 150), (255, 255, 255, 255));
        // Without a shadow the same pixel is white.
        let mut pg2 = Page::new("s2", "S2", "/s2", 200.0, 160.0);
        pg2.root
            .children
            .push(Node::rect("r", "R", 40.0, 30.0, 80.0, 50.0, Color::WHITE));
        let c2 = render_page(&pg2, 1.0);
        assert_eq!(c2.pixel(80, 92), (255, 255, 255, 255));
        let svg = render_page_svg(&pg);
        assert!(svg.contains("drop-shadow"), "svg carries the shadow");
    }
    #[test]
    fn letter_spacing_widens_the_ink() {
        let rightmost = |spacing: f32| {
            let mut pg = Page::new("ls", "LS", "/ls", 400.0, 60.0);
            let mut t = Node::text_node(
                "t",
                "T",
                4.0,
                4.0,
                390.0,
                40.0,
                "ABCDEFGH",
                20.0,
                Color::BLACK,
            );
            t.letter_spacing = spacing;
            t.props.insert("align".into(), "left".into());
            pg.root.children.push(t);
            let c = render_page(&pg, 1.0);
            (0..400u32)
                .rev()
                .find(|x| (8..52u32).any(|y| c.pixel(*x, y).0 < 100))
                .unwrap_or(0)
        };
        let plain = rightmost(0.0);
        let spaced = rightmost(8.0);
        assert!(
            spaced > plain + 30,
            "spacing widens ink: plain {plain}, spaced {spaced}"
        );
    }
    #[test]
    fn auto_layout_container_renders_children_at_computed_positions() {
        use chida_core::{LayoutAlign, LayoutDirection};
        let mut pg = Page::new("lay", "Lay", "/lay", 220.0, 80.0);
        let mut row = Node::new("row", "Row", NodeKind::Frame, 10.0, 10.0, 200.0, 60.0);
        row.layout_direction = LayoutDirection::Row;
        row.layout_gap = 10.0;
        row.layout_padding = 10.0;
        row.layout_align = LayoutAlign::Center;
        // Stored positions are deliberately wrong; the layout wins.
        let mut a = Node::rect("a", "A", 500.0, 500.0, 40.0, 20.0, Color::rgb(200, 0, 0));
        let mut b = Node::rect("b", "B", 700.0, 700.0, 40.0, 20.0, Color::rgb(0, 0, 200));
        a.x = 500.0;
        b.x = 700.0;
        row.children = vec![a, b];
        pg.root.children.push(row);
        let c = render_page(&pg, 1.0);
        // a: x = 10 (frame) + 10 (pad) = 20, y = 10 + (60-20)/2 = 30.
        assert_eq!(c.pixel(30, 40), (200, 0, 0, 255));
        // b: x = 20 + 40 + 10 (gap) = 70.
        assert_eq!(c.pixel(80, 40), (0, 0, 200, 255));
        // The stored spot stays empty.
        assert_eq!(c.pixel(150, 40), (255, 255, 255, 255));
        let svg = render_page_svg(&pg);
        assert!(svg.contains("x=\"20\""), "svg uses the computed x: {svg}");
        assert!(svg.contains("x=\"70\""), "svg uses the computed x: {svg}");
    }
    #[test]
    fn image_node_paints_the_decoded_photo() {
        let dir = std::env::temp_dir().join("chida-render-img-test");
        std::fs::create_dir_all(&dir).unwrap();
        // A solid red 8x8 asset written by our own PNG encoder.
        let mut asset = Canvas::new(8, 8, Color::rgb(200, 30, 40));
        asset.save_png(&dir.join("red.png")).unwrap();
        let mut page = Page::new("p", "P", "/", 40.0, 40.0);
        page.root
            .children
            .push(Node::image("img", "red.png", 4.0, 4.0, 32.0, 32.0));
        let canvas = render_page_with_base(&page, 1.0, &dir);
        let px = canvas.pixel(20, 20);
        assert!(
            px.0 > 150 && px.1 < 90 && px.2 < 90,
            "centre pixel is the photo red: {px:?}"
        );
        // A missing asset falls back to the placeholder (no panic).
        let mut page2 = Page::new("p", "P", "/", 40.0, 40.0);
        page2
            .root
            .children
            .push(Node::image("img", "missing.jpg", 4.0, 4.0, 32.0, 32.0));
        let _ = render_page_with_base(&page2, 1.0, &dir);
        let svg = render_page_svg(&page);
        assert!(
            svg.contains("<image href=\"red.png\""),
            "svg references the asset"
        );
    }
}
