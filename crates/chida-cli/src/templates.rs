//! Built-in templates beyond the starter shop: a Persian marketplace in
//! the style of large Iranian e-commerce sites (original sample brand
//! «بازارینو», original data, no third party logos or content) and an
//! Apple liquid glass family.
//!
//! Every template is a real Chida project built from chida-core nodes,
//! a chida-db schema with seed rows, and chida-logic flows, so it can be
//! rendered, edited in the editor and code generated like any project.

use crate::project_file::ProjectFile;
use chida_core::{Color, Node, NodeKind, Page, Project};
use chida_db::{Column, ColumnType, Database, Relation, RelationKind, Table};
use chida_logic::{Action, CompareOp, Event, EventKind, Expr, Flow, Step};
use serde_json::{json, Map};

// ---------------------------------------------------------------------
// Small builders shared by both templates.
// ---------------------------------------------------------------------

/// Format an integer with Persian digits and thousands separators,
/// e.g. 480000 becomes ۴۸۰٬۰۰۰.
pub fn fa_num(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('٬');
        }
        out.push(match c {
            '0' => '۰',
            '1' => '۱',
            '2' => '۲',
            '3' => '۳',
            '4' => '۴',
            '5' => '۵',
            '6' => '۶',
            '7' => '۷',
            '8' => '۸',
            _ => '۹',
        });
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

pub fn fa_price(n: i64) -> String {
    format!("{} تومان", fa_num(n))
}

/// Format a rating like 4.4 with Persian digits and decimal separator.
pub fn fa_rating(r: f64) -> String {
    let whole = r.trunc() as i64;
    let frac = ((r - whole as f64) * 10.0).round() as i64;
    format!("{}٫{}", fa_num(whole), fa_num(frac))
}

fn t(id: &str, x: f32, y: f32, w: f32, s: &str, size: f32, color: Color, bold: bool) -> Node {
    let mut n = Node::text_node(id, s, x, y, w, size + 12.0, s, size, color);
    n.bold = bold;
    n
}

fn rect(id: &str, name: &str, x: f32, y: f32, w: f32, h: f32, fill: Color, radius: f32) -> Node {
    let mut n = Node::rect(id, name, x, y, w, h, fill);
    n.corner_radius = radius;
    n
}

fn outlined(
    id: &str,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: Color,
    radius: f32,
) -> Node {
    let mut n = rect(id, name, x, y, w, h, fill, radius);
    n.stroke = Some(Color::rgb(224, 226, 231));
    n.stroke_width = 1.0;
    n
}

fn btn(
    id: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    size: f32,
    fill: Color,
    tc: Color,
    radius: f32,
) -> Node {
    let mut b = Node::new(id, label, NodeKind::Button, x, y, w, h);
    b.fill = Some(fill);
    b.corner_radius = radius;
    b.text = Some(label.into());
    b.font_size = size;
    b.text_color = Some(tc);
    b.bold = true;
    b
}

fn ellipse(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> Node {
    Node::new(id, "Glow", NodeKind::Ellipse, x, y, w, h).with_fill(fill)
}

trait WithFill {
    fn with_fill(self, fill: Color) -> Node;
}
impl WithFill for Node {
    fn with_fill(mut self, fill: Color) -> Node {
        self.fill = Some(fill);
        self
    }
}

// ---------------------------------------------------------------------
// Marketplace palette and product data (all sample data is original).
// ---------------------------------------------------------------------

const M_RED: Color = Color::rgb(239, 64, 86);
const M_AMAZING: Color = Color::rgb(216, 47, 78);
const M_GREEN: Color = Color::rgb(111, 186, 42);
const M_CREAM: Color = Color::rgb(255, 251, 237);
const M_INK: Color = Color::rgb(63, 64, 100);
const M_MUTED: Color = Color::rgb(129, 133, 149);
const M_LINE: Color = Color::rgb(224, 226, 231);
const M_PILL: Color = Color::rgb(240, 240, 241);
const M_ORANGE: Color = Color::rgb(245, 127, 23);
const M_ORANGE_BG: Color = Color::rgb(255, 243, 224);
const MX: f32 = 65.0;
const CW: f32 = 1310.0;

/// Centred text: the renderer honours the `align` prop.
fn tc(id: &str, x: f32, y: f32, w: f32, s: &str, size: f32, color: Color, bold: bool) -> Node {
    let mut n = t(id, x, y, w, s, size, color, bold);
    n.props.insert("align".into(), "center".into());
    n
}

/// Left aligned text (visual left), for price columns.
fn tl(id: &str, x: f32, y: f32, w: f32, s: &str, size: f32, color: Color, bold: bool) -> Node {
    let mut n = t(id, x, y, w, s, size, color, bold);
    n.props.insert("align".into(), "left".into());
    n
}

/// A real photo node backed by a bundled asset file.
fn img(id: &str, file: &str, x: f32, y: f32, w: f32, h: f32, radius: f32) -> Node {
    let mut n = Node::image(id, format!("assets/digikala/{file}"), x, y, w, h);
    n.name = "Product photo".into();
    n.corner_radius = radius;
    n.fill = Some(Color::rgb(240, 240, 241));
    n
}

#[derive(Clone)]
struct Product {
    name: &'static str,
    brand: &'static str,
    price: i64,
    off: i64,
    rating: f64,
    reviews: &'static str,
    img: &'static str,
}

const fn prod(
    name: &'static str,
    brand: &'static str,
    price: i64,
    off: i64,
    rating: f64,
    reviews: &'static str,
    img: &'static str,
) -> Product {
    Product { name, brand, price, off, rating, reviews, img }
}

fn products() -> Vec<Product> {
    vec![
        prod("ساعت هوشمند شیائومی مدل Watch S1 Active", "شیائومی", 4_850_000, 12, 4.5, "۲۳۴", "watch.jpg"),
        prod("هدفون بی‌سیم سونی مدل WH-1000XM5", "سونی", 11_500_000, 18, 4.8, "۵۱۲", "headphones.jpg"),
        prod("گوشی موبایل سامسونگ مدل Galaxy A55", "سامسونگ", 18_900_000, 8, 4.4, "۸۹۰", "phone.jpg"),
        prod("لپ‌تاپ ایسوس مدل Vivobook 15", "ایسوس", 32_500_000, 10, 4.3, "۱۴۵", "laptop.jpg"),
        prod("کفش رانینگ نایک مدل Air Zoom", "نایک", 5_400_000, 25, 4.6, "۳۲۱", "sneakers.jpg"),
        prod("تلویزیون ال‌جی مدل 55UQ8000 سایز ۵۵ اینچ", "ال‌جی", 28_900_000, 15, 4.5, "۲۰۸", "tv.jpg"),
        prod("اسپرسوساز دلونگی مدل Magnifica S", "دلونگی", 9_800_000, 20, 4.7, "۱۷۶", "coffee.jpg"),
        prod("کوله‌پشتی مسافرتی ۲۵ لیتری ضد آب", "‌نوین چرم", 1_250_000, 30, 4.2, "۹۸", "backpack.jpg"),
        prod("کرم مرطوب‌کننده نیوآ مدل Soft", "نیوآ", 385_000, 22, 4.4, "۱٬۲۴۰", "cream.jpg"),
        prod("مکمل ویتامین D3 هزار واحدی", "ویتالوگ", 290_000, 15, 4.3, "۶۷۰", "vitamins.jpg"),
        prod("قاب ژله‌ای شفاف گوشی سامسونگ A55", "‌اپیکوی", 180_000, 40, 4.1, "۲٬۳۰۰", "phonecase.jpg"),
        prod("رمان ملت عشق اثر الیف شافاک", "نشر ققنوس", 240_000, 10, 4.9, "۳٬۴۵۰", "book.jpg"),
    ]
}

fn extra_products() -> Vec<Product> {
    vec![
        prod("عطر ادکلن لالیک مدل Encre Noire", "لالیک", 2_850_000, 35, 4.6, "۴۳۲", "perfume.jpg"),
        prod("یخچال فریزر اسنوا ۲۷ فوت مدل SN8", "اسنوا", 42_000_000, 12, 4.3, "۱۵۶", "fridge.jpg"),
        prod("ماشین لباسشویی پاکشوما ۸ کیلویی", "پاکشوما", 21_500_000, 14, 4.2, "۲۳۱", "washer.jpg"),
        prod("دریل شارژی رونیکس مدل ۸۹۰۰", "رونیکس", 1_980_000, 28, 4.5, "۵۴۳", "drill.jpg"),
        prod("چتر اتوماتیک تاشو ضد باد", "‌شیدور", 420_000, 33, 4.0, "۸۷۶", "umbrella.jpg"),
        prod("لگو ساختنی کلاسیک ۵۰۰ قطعه", "لگو", 890_000, 20, 4.8, "۱٬۱۲۰", "toy.jpg"),
        prod("کارت هدیه بازارینو به مبلغ دلخواه", "بازارینو", 500_000, 0, 4.9, "۹۹۰", "gift.jpg"),
        prod("گردنبند طلا ۱۸ عیار زنانه", "گالری زرین", 12_500_000, 5, 4.7, "۶۵", "jewellery.jpg"),
        prod("سبد خرید سوپرمارکتی ماهانه خانواده", "بازارینو مارکت", 750_000, 10, 4.4, "۲٬۰۱۰", "groceries.jpg"),
        prod("سرویس قابلمه گرانیتی ۱۰ پارچه", "کاندید", 3_600_000, 22, 4.5, "۳۸۷", "kitchen.jpg"),
        prod("دمبل قابل تنظیم ۱۰ کیلوگرمی", "پرو اسپرت", 1_450_000, 18, 4.3, "۲۹۸", "sport.jpg"),
        prod("دستگاه فشارخون دیجیتال بازویی", "امرون", 1_750_000, 16, 4.6, "۷۵۴", "medical.jpg"),
        prod("لاستیک خودرو سایز ۲۰۵/۶۰R۱۵", "بارز", 2_900_000, 9, 4.4, "۱۶۷", "car.jpg"),
        prod("مانتو زنانه تابستانه مدل کمربندی", "استایلینو", 980_000, 45, 4.1, "۴۵۹", "fashion.jpg"),
    ]
}

fn final_price(p: &Product) -> i64 {
    p.price * (100 - p.off) / 100
}

/// Truncate a title the way storefront cards do, with an ellipsis.
fn short_title(name: &str, max: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= max {
        name.to_string()
    } else {
        let cut: String = chars.iter().take(max - 1).collect();
        format!("{cut}…")
    }
}

/// Compact card for the cream delivery strip: photo, one line title,
/// one line price.
fn photo_card_mini(idp: &str, x: f32, y: f32, w: f32, h: f32, p: &Product) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(rect(idp, "Product card", x, y, w, h, Color::WHITE, 8.0));
    n.push(img(&format!("{idp}-img"), p.img, x + 10.0, y + 8.0, w - 20.0, 76.0, 6.0));
    n.push(tc(&format!("{idp}-title"), x + 8.0, y + 90.0, w - 16.0, &short_title(p.name, 20), 12.0, M_INK, false));
    n.push(tc(&format!("{idp}-price"), x + 8.0, y + h - 30.0, w - 16.0, &format!("{} تومان", fa_price(final_price(p))), 13.0, M_INK, true));
    n
}

/// Centred price block (carousel card anatomy): old price struck with
/// the red discount pill on the first line, final price bold below.
fn price_centred(idp: &str, x: f32, y: f32, w: f32, p: &Product) -> Vec<Node> {
    let mut n = Vec::new();
    if p.off > 0 {
        n.push(tc(&format!("{idp}-old"), x + 20.0, y, w - 52.0, &fa_price(p.price), 12.0, M_MUTED, false));
        n.push(rect(&format!("{idp}-strike"), "", x + w / 2.0 - 26.0, y + 8.0, 52.0, 1.0, M_MUTED, 0.0));
        let pill = btn(&format!("{idp}-off"), x + w - 46.0, y, 38.0, 20.0, &format!("٪{}", fa_num(p.off)), 12.0, M_RED, Color::WHITE, 10.0);
        n.push(pill);
        n.push(tc(&format!("{idp}-price"), x - 16.0, y + 22.0, w + 32.0, &format!("{} تومان", fa_price(final_price(p))), 15.0, M_INK, true));
    } else {
        n.push(tc(&format!("{idp}-price"), x - 16.0, y + 22.0, w + 32.0, &format!("{} تومان", fa_price(p.price)), 15.0, M_INK, true));
    }
    n
}

/// Carousel card (anatomy A): white, photo, centred title and prices.
fn photo_card(idp: &str, x: f32, y: f32, w: f32, h: f32, p: &Product, bordered: bool) -> Vec<Node> {
    let mut n = Vec::new();
    if bordered {
        n.push(outlined(idp, "Product card", x, y, w, h, Color::WHITE, 10.0));
    } else {
        n.push(rect(idp, "Product card", x, y, w, h, Color::WHITE, 8.0));
    }
    let img_h = (h - 118.0).max(70.0);
    n.push(img(&format!("{idp}-img"), p.img, x + 10.0, y + 10.0, w - 20.0, img_h, 6.0));
    n.push(tc(&format!("{idp}-title"), x + 8.0, y + img_h + 16.0, w - 16.0, &short_title(p.name, 30), 13.0, M_INK, false));
    n.extend(price_centred(idp, x, y + h - 52.0, w, p));
    n
}

fn market_header(prefix: &str) -> Vec<Node> {
    let mut n = Vec::new();
    // 1. Promo strip.
    n.push(rect(&format!("{prefix}-promo"), "Promo strip", 0.0, 0.0, 1440.0, 40.0, Color::rgb(20, 20, 24), 0.0));
    n.push(tc(&format!("{prefix}-promo-t"), 420.0, 9.0, 600.0, "جشنواره فروش تابستانه بازارینو، تا ۴۰ درصد تخفیف", 13.0, Color::WHITE, true));
    // 2. Main header row.
    n.push(rect(&format!("{prefix}-hdr"), "Header", 0.0, 40.0, 1440.0, 76.0, Color::WHITE, 0.0));
    n.push(t(&format!("{prefix}-logo"), 1175.0, 58.0, 200.0, "بازارینو", 30.0, M_RED, true));
    let search = outlined(&format!("{prefix}-search"), "Search box", 430.0, 56.0, 690.0, 46.0, M_PILL, 12.0);
    n.push(search);
    n.push(t(&format!("{prefix}-search-t"), 452.0, 68.0, 640.0, "جستجو در بازارینو", 14.0, M_MUTED, false));
    let bell = outlined(&format!("{prefix}-bell"), "Notifications", 372.0, 56.0, 46.0, 46.0, Color::WHITE, 10.0);
    n.push(bell);
    n.push(tc(&format!("{prefix}-bell-t"), 372.0, 66.0, 46.0, "اعلان", 10.0, M_MUTED, false));
    let login = outlined(&format!("{prefix}-login"), "Login button", 216.0, 56.0, 138.0, 46.0, Color::WHITE, 10.0);
    n.push(login);
    n.push(tc(&format!("{prefix}-login-t"), 216.0, 68.0, 138.0, "ورود | ثبت‌نام", 13.0, M_INK, true));
    n.push(rect(&format!("{prefix}-div"), "", 198.0, 62.0, 1.0, 34.0, M_LINE, 0.0));
    let cart = outlined(&format!("{prefix}-cart"), "Cart button", 118.0, 56.0, 62.0, 46.0, Color::WHITE, 10.0);
    n.push(cart);
    n.push(tc(&format!("{prefix}-cart-t"), 118.0, 68.0, 62.0, "سبد خرید", 11.0, M_INK, true));
    // 3. Nav row.
    n.push(rect(&format!("{prefix}-nav"), "Nav", 0.0, 116.0, 1440.0, 46.0, Color::WHITE, 0.0));
    n.push(rect(&format!("{prefix}-navline"), "", 0.0, 161.0, 1440.0, 1.0, M_LINE, 0.0));
    let links: [(&str, bool, Color); 7] = [
        ("☰ دسته‌بندی کالاها", true, M_INK),
        ("پیشنهاد شگفت‌انگیز", true, M_RED),
        ("سوپرمارکت", false, M_INK),
        ("طلای دیجیتال", false, M_INK),
        ("پرفروش‌ترین‌ها", false, M_INK),
        ("بازارینو استایل", false, M_INK),
        ("در بازارینو بفروشید", false, M_MUTED),
    ];
    let widths = [150.0, 128.0, 84.0, 96.0, 108.0, 104.0, 128.0];
    let mut lx = 1375.0;
    for (i, (label, bold, color)) in links.iter().enumerate() {
        lx -= widths[i];
        n.push(t(&format!("{prefix}-nav{i}"), lx, 128.0, widths[i], label, 13.0, *color, *bold));
    }
    let loc = rect(&format!("{prefix}-loc"), "Location pill", MX, 124.0, 218.0, 30.0, M_ORANGE_BG, 15.0);
    n.push(loc);
    n.push(tc(&format!("{prefix}-loc-t"), MX, 130.0, 218.0, "ارسال به تهران، تهران", 12.0, M_ORANGE, true));
    n
}

/// One amazing offers band (red or green supermarket).
fn amazing_section(prefix: &str, y: f32, bg: Color, title: &str, items: &[Product]) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(rect(&format!("{prefix}-band"), "Amazing band", MX, y, CW, 330.0, bg, 16.0));
    // Right typography block.
    n.push(tc(&format!("{prefix}-pct"), 1185.0, y + 18.0, 180.0, "٪", 56.0, Color::WHITE, true));
    n.push(tc(&format!("{prefix}-title"), 1185.0, y + 92.0, 180.0, title, 21.0, Color::WHITE, true));
    let times = ["۳۶", "۲۵", "۰۴"];
    for (i, tm) in times.iter().enumerate() {
        let bx = 1309.0 - (i as f32) * 48.0;
        n.push(rect(&format!("{prefix}-tm{i}"), "", bx, y + 140.0, 42.0, 42.0, Color::WHITE, 8.0));
        n.push(tc(&format!("{prefix}-tm{i}t"), bx, y + 149.0, 42.0, tm, 16.0, M_INK, true));
    }
    n.push(btn(&format!("{prefix}-all"), 1200.0, y + 262.0, 120.0, 38.0, "مشاهده همه", 13.0, Color::WHITE, bg, 19.0));
    // Cards.
    let card_w = 137.0;
    let gap = 6.0;
    let start = 1033.0;
    for (i, p) in items.iter().take(7).enumerate() {
        let cx = start - (i as f32) * (card_w + gap);
        n.extend(photo_card(&format!("{prefix}-c{i}"), cx, y + 12.0, card_w, 306.0, p, false));
    }
    // End tile: see all.
    let tx = 75.0;
    let tile = outlined(&format!("{prefix}-end"), "See all tile", tx, y + 12.0, 96.0, 306.0, Color::WHITE, 8.0);
    n.push(tile);
    n.push(ellipse(&format!("{prefix}-end-c"), tx + 28.0, y + 110.0, 40.0, 40.0, M_PILL));
    n.push(tc(&format!("{prefix}-end-a"), tx + 28.0, y + 114.0, 40.0, "←", 20.0, M_INK, true));
    n.push(tc(&format!("{prefix}-end-t"), tx + 8.0, y + 168.0, 80.0, "مشاهده همه", 12.0, M_INK, true));
    n
}

fn banner_tile(idp: &str, x: f32, y: f32, w: f32, h: f32, file: &str, label: &str) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(img(&format!("{idp}-img"), file, x, y, w, h, 16.0));
    n.push(rect(&format!("{idp}-shade"), "", x, y + h - 56.0, w, 56.0, Color::rgba(15, 15, 25, 130), 0.0));
    n.push(t(&format!("{idp}-t"), x + 16.0, y + h - 42.0, w - 32.0, label, 17.0, Color::WHITE, true));
    n
}

fn banner_row4(prefix: &str, y: f32, tiles: [(&str, &str); 4]) -> Vec<Node> {
    let mut n = Vec::new();
    let w = 317.0;
    for (i, (file, label)) in tiles.iter().enumerate() {
        let x = MX + (i as f32) * (w + 13.0);
        n.extend(banner_tile(&format!("{prefix}-{i}"), x, y, w, 230.0, file, label));
    }
    n
}

fn banner_row2(prefix: &str, y: f32, h: f32, tiles: [(&str, &str); 2], buy_pills: bool) -> Vec<Node> {
    let mut n = Vec::new();
    let w = 649.0;
    for (i, (file, label)) in tiles.iter().enumerate() {
        let x = MX + (i as f32) * (w + 12.0);
        n.extend(banner_tile(&format!("{prefix}-{i}"), x, y, w, h, file, label));
        if buy_pills {
            n.push(btn(&format!("{prefix}-{i}-buy"), x + w - 116.0, y + h - 48.0, 96.0, 32.0, "خرید", 13.0, Color::WHITE, M_INK, 16.0));
        }
    }
    n
}

/// Bordered product row with header and 7 cards plus an edge arrow.
fn product_row(prefix: &str, y: f32, title: &str, subtitle: &str, items: &[Product]) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(outlined(&format!("{prefix}-box"), "Product row", MX, y, CW, 352.0, Color::WHITE, 16.0));
    n.push(t(&format!("{prefix}-title"), 651.0, y + 18.0, 700.0, title, 19.0, M_INK, true));
    n.push(t(&format!("{prefix}-sub"), 651.0, y + 50.0, 700.0, subtitle, 12.0, M_MUTED, false));
    n.push(btn(&format!("{prefix}-all"), MX + 20.0, y + 20.0, 110.0, 34.0, "مشاهده همه", 12.0, M_PILL, M_INK, 17.0));
    let card_w = 168.0;
    let gap = 10.0;
    let start = MX + CW - 24.0 - card_w;
    for (i, p) in items.iter().take(7).enumerate() {
        let cx = start - (i as f32) * (card_w + gap);
        n.extend(photo_card(&format!("{prefix}-c{i}"), cx, y + 92.0, card_w, 244.0, p, true));
    }
    n.push(ellipse(&format!("{prefix}-arrow"), MX + 6.0, y + 196.0, 40.0, 40.0, Color::WHITE));
    n.push(tc(&format!("{prefix}-arrow-t"), MX + 6.0, y + 201.0, 40.0, "‹", 22.0, M_INK, true));
    n
}

/// Ranked grid (best sellers / hottest): 3 columns x 5 rows, no prices.
fn ranked_section(prefix: &str, y: f32, title: &str, items: &[Product]) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(t(&format!("{prefix}-title"), 675.0, y, 700.0, title, 19.0, M_INK, true));
    let col_w = 423.0;
    for idx in 0..15 {
        let p = &items[idx % items.len()];
        let col = (idx % 3) as f32;
        let row = (idx / 3) as f32;
        let cx = 1375.0 - (col + 1.0) * col_w;
        let cy = y + 48.0 + row * 112.0;
        let idp = format!("{prefix}-{idx}");
        n.push(ellipse(&format!("{idp}-rank"), cx + col_w - 44.0, cy + 30.0, 28.0, 28.0, M_RED));
        n.push(tc(&format!("{idp}-rank-t"), cx + col_w - 44.0, cy + 35.0, 28.0, &fa_num((idx + 1) as i64), 13.0, Color::WHITE, true));
        n.push(img(&format!("{idp}-img"), p.img, cx + col_w - 142.0, cy + 6.0, 88.0, 88.0, 8.0));
        n.push(t(&format!("{idp}-t"), cx + 6.0, cy + 18.0, col_w - 160.0, p.name, 13.0, M_INK, false));
    }
    n
}

/// Suggestion grid card (anatomy B): right aligned title, rating line,
/// price row with the pill on the right and old price below the final.
fn suggestion_card(idp: &str, x: f32, y: f32, w: f32, h: f32, p: &Product, stock_note: bool) -> Vec<Node> {
    let mut n = Vec::new();
    n.push(img(&format!("{idp}-img"), p.img, x + (w - 190.0) / 2.0, y + 8.0, 190.0, 140.0, 6.0));
    n.push(t(&format!("{idp}-title"), x + 12.0, y + 156.0, w - 24.0, &short_title(p.name, 44), 13.0, M_INK, false));
    if stock_note {
        n.push(t(&format!("{idp}-stock"), x + 12.0, y + 200.0, w - 24.0, "تنها ۲ عدد در انبار باقی مانده", 11.0, M_RED, true));
    }
    n.push(tl(&format!("{idp}-rate"), x + 12.0, y + 224.0, 90.0, &format!("★ {}", fa_rating(p.rating)), 12.0, Color::rgb(240, 164, 0), true));
    if p.off > 0 {
        n.push(btn(&format!("{idp}-off"), x + w - 50.0, y + h - 58.0, 38.0, 20.0, &format!("٪{}", fa_num(p.off)), 12.0, M_RED, Color::WHITE, 10.0));
        n.push(tl(&format!("{idp}-old"), x + 12.0, y + h - 34.0, 130.0, &fa_price(p.price), 12.0, M_MUTED, false));
        n.push(rect(&format!("{idp}-strike"), "", x + 16.0, y + h - 27.0, 52.0, 1.0, M_MUTED, 0.0));
    }
    n.push(tl(&format!("{idp}-price"), x + 12.0, y + h - 62.0, 178.0, &format!("{} تومان", fa_price(final_price(p))), 15.0, M_INK, true));
    n
}

fn market_footer(prefix: &str, y: f32) -> Node {
    let mut g = Node::new(format!("{prefix}-footer"), "Footer", NodeKind::Frame, 0.0, 0.0, 1440.0, 700.0);
    let mut n: Vec<Node> = Vec::new();
    n.push(rect(&format!("{prefix}-fline"), "", 0.0, y, 1440.0, 1.0, M_LINE, 0.0));
    // a. Top line: logo, support phones, back to top.
    n.push(t(&format!("{prefix}-flogo"), 1175.0, y + 22.0, 200.0, "بازارینو", 26.0, M_RED, true));
    n.push(t(&format!("{prefix}-ftel"), 500.0, y + 30.0, 660.0, "تلفن پشتیبانی: ۰۲۱-۹۱۰۰۰۱۰۰ | ۰۲۱-۹۱۰۰۰۲۰۰ ، پاسخگویی ۷ روز هفته، ۲۴ ساعته", 13.0, M_MUTED, false));
    n.push(btn(&format!("{prefix}-ftop"), MX, y + 20.0, 130.0, 38.0, "بازگشت به بالا", 12.0, M_PILL, M_INK, 10.0));
    // b. Service highlights.
    let services = [
        ("تحویل اکسپرس", "ارسال سریع به سراسر کشور"),
        ("پرداخت در محل", "برای سفارش‌های تهران"),
        ("۷ روز هفته، ۲۴ ساعته", "پشتیبانی همیشه پاسخگو"),
        ("۷ روز ضمانت بازگشت", "بدون قید و شرط"),
        ("ضمانت اصل بودن کالا", "کالای اورجینال با گارانتی"),
    ];
    for (i, (a, b)) in services.iter().enumerate() {
        let cx = 1375.0 - (i as f32 + 1.0) * 262.0 + 131.0;
        n.push(ellipse(&format!("{prefix}-fs{i}"), cx - 22.0, y + 84.0, 44.0, 44.0, M_PILL));
        n.push(tc(&format!("{prefix}-fs{i}s"), cx - 22.0, y + 93.0, 44.0, "✓", 18.0, M_RED, true));
        n.push(tc(&format!("{prefix}-fs{i}a"), cx - 120.0, y + 138.0, 240.0, a, 13.0, M_INK, true));
        n.push(tc(&format!("{prefix}-fs{i}b"), cx - 120.0, y + 160.0, 240.0, b, 11.0, M_MUTED, false));
    }
    // c. Link columns plus newsletter.
    let cols: [(&str, [&str; 5]); 3] = [
        ("با بازارینو", ["اتاق خبر بازارینو", "فروش در بازارینو", "فرصت‌های شغلی", "گزارش تخلف", "تماس با ما"]),
        ("خدمات مشتریان", ["پاسخ به پرسش‌های متداول", "رویه بازگرداندن کالا", "شرایط استفاده", "حریم خصوصی", "گزارش خطا"]),
        ("راهنمای خرید", ["نحوه ثبت سفارش", "رویه ارسال سفارش", "شیوه‌های پرداخت", "پیگیری سفارش", "کارت هدیه"]),
    ];
    for (ci, (head, items)) in cols.iter().enumerate() {
        let cx = 1130.0 - (ci as f32) * 245.0;
        n.push(t(&format!("{prefix}-fc{ci}"), cx, y + 208.0, 220.0, head, 15.0, M_INK, true));
        for (ri, item) in items.iter().enumerate() {
            n.push(t(&format!("{prefix}-fc{ci}-{ri}"), cx, y + 240.0 + (ri as f32) * 26.0, 220.0, item, 13.0, M_MUTED, false));
        }
    }
    n.push(t(&format!("{prefix}-fnl"), MX, y + 208.0, 300.0, "همراه ما باشید", 15.0, M_INK, true));
    for i in 0..5 {
        n.push(ellipse(&format!("{prefix}-fsoc{i}"), 335.0 - (i as f32) * 44.0, y + 238.0, 36.0, 36.0, M_PILL));
        n.push(tc(&format!("{prefix}-fsoc{i}t"), 335.0 - (i as f32) * 44.0, y + 246.0, 36.0, "●", 12.0, M_MUTED, false));
    }
    n.push(t(&format!("{prefix}-fnl2"), MX, y + 292.0, 300.0, "از جدیدترین تخفیف‌ها باخبر شوید", 13.0, M_INK, false));
    let mail = outlined(&format!("{prefix}-fmail"), "Newsletter email", MX + 76.0, y + 322.0, 224.0, 44.0, M_PILL, 10.0);
    n.push(mail);
    n.push(t(&format!("{prefix}-fmail-t"), MX + 88.0, y + 334.0, 200.0, "ایمیل شما", 13.0, M_MUTED, false));
    n.push(btn(&format!("{prefix}-fsub"), MX, y + 322.0, 68.0, 44.0, "ثبت", 13.0, Color::rgb(66, 71, 80), Color::WHITE, 10.0));
    // d. App download.
    n.push(t(&format!("{prefix}-fapp"), 900.0, y + 396.0, 475.0, "دانلود اپلیکیشن بازارینو", 16.0, M_INK, true));
    let stores = ["کافه‌بازار", "مایکت", "سیب‌اپ"];
    for (i, sname) in stores.iter().enumerate() {
        let bx = 620.0 - (i as f32) * 150.0;
        n.push(btn(&format!("{prefix}-fstore{i}"), bx, y + 390.0, 140.0, 44.0, sname, 13.0, Color::rgb(66, 71, 80), Color::WHITE, 10.0));
    }
    n.push(t(&format!("{prefix}-fabout"), MX, y + 452.0, 1310.0, "بازارینو یک فروشگاه اینترنتی نمونه است که با چیدا ساخته شده؛ این متن معرفی کوتاه فروشگاه است و در نسخه واقعی با متن برند شما جایگزین می‌شود.", 12.0, M_MUTED, false));
    // e. Trust badges (labelled placeholders).
    for i in 0..4 {
        let bx = 470.0 + (i as f32) * 130.0;
        n.push(outlined(&format!("{prefix}-ftrust{i}"), "Trust badge placeholder", bx, y + 500.0, 120.0, 84.0, Color::WHITE, 10.0));
        n.push(tc(&format!("{prefix}-ftrust{i}t"), bx + 6.0, y + 532.0, 108.0, "نماد اعتماد (نمونه)", 11.0, M_MUTED, false));
    }
    // f. Copyright.
    n.push(tc(&format!("{prefix}-fcopy"), 320.0, y + 606.0, 800.0, "تمامی حقوق این وب‌سایت برای فروشگاه نمونه بازارینو محفوظ است.", 12.0, M_MUTED, false));
    // g. Group companies strip.
    n.push(rect(&format!("{prefix}-fgroup"), "", 0.0, y + 640.0, 1440.0, 48.0, M_PILL, 0.0));
    let group = ["بازارینو", "بازارینو مارکت", "بازارینو استایل", "بازارینو پی", "بازارینو تراول", "بازارینو بوک", "بازارینو جت", "بازارینو هلث"];
    for (i, gname) in group.iter().enumerate() {
        n.push(tc(&format!("{prefix}-fg{i}"), 60.0 + (i as f32) * 168.0, y + 654.0, 150.0, gname, 12.0, M_MUTED, true));
    }
    // h. Floating support button.
    n.push(ellipse(&format!("{prefix}-fsup"), MX, y + 560.0, 52.0, 52.0, Color::rgb(29, 155, 240)));
    n.push(tc(&format!("{prefix}-fsup-t"), MX, y + 573.0, 52.0, "؟", 20.0, Color::WHITE, true));
    g.children = n;
    g
}

fn market_home(prods: &[Product], extras: &[Product]) -> Page {
    let mut page = Page::new("home", "صفحه اصلی", "/", 1440.0, 8700.0);
    page.background = Some(Color::WHITE);
    let mut n: Vec<Node> = Vec::new();
    n.extend(market_header("hm"));
    // 4. Hero slider.
    n.push(img("hm-hero", "hero.jpg", 0.0, 178.0, 1440.0, 380.0, 0.0));
    n.push(rect("hm-hero-shade", "", 830.0, 300.0, 520.0, 170.0, Color::rgba(15, 15, 25, 120), 16.0));
    n.push(t("hm-hero-t", 856.0, 322.0, 470.0, "جشنواره تابستانه بازارینو", 32.0, Color::WHITE, true));
    n.push(t("hm-hero-s", 856.0, 372.0, 470.0, "تا ۴۰ درصد تخفیف روی کالای دیجیتال و خانه", 16.0, Color::WHITE, false));
    n.push(btn("hm-hero-b", 1180.0, 416.0, 150.0, 40.0, "مشاهده پیشنهادها", 13.0, Color::WHITE, M_RED, 20.0));
    n.push(rect("hm-dot0", "", 690.0, 530.0, 26.0, 8.0, Color::rgb(50, 50, 60), 4.0));
    for i in 0..4 {
        n.push(ellipse(&format!("hm-dot{}", i + 1), 724.0 + (i as f32) * 16.0, 530.0, 8.0, 8.0, Color::rgba(255, 255, 255, 200)));
    }
    for (i, glyph) in ["›", "‹"].iter().enumerate() {
        n.push(ellipse(&format!("hm-arrow{i}"), 90.0 + (i as f32) * 52.0, 492.0, 42.0, 42.0, Color::WHITE));
        n.push(tc(&format!("hm-arrow{i}t"), 90.0 + (i as f32) * 52.0, 497.0, 42.0, glyph, 22.0, M_INK, true));
    }
    // 5. Service highlights.
    let services: [(&str, &str); 10] = [
        ("٪", "بازارینو پلاس"),
        ("+", "وام بانکی"),
        ("★", "بازارینو کوین"),
        ("♦", "بازارینو استایل"),
        ("●", "طلای دیجیتال"),
        ("٪", "تخفیف طلا"),
        ("▲", "دانشنامه شاهنامه"),
        ("■", "سوپرمارکت"),
        ("✚", "اعتبار فوری"),
        ("…", "بیشتر"),
    ];
    for (i, (glyph, label)) in services.iter().enumerate() {
        let cx = 130.0 + (i as f32) * 131.0;
        n.push(ellipse(&format!("hm-sv{i}"), cx - 26.0, 578.0, 52.0, 52.0, M_PILL));
        n.push(tc(&format!("hm-sv{i}g"), cx - 26.0, 590.0, 52.0, glyph, 19.0, M_RED, true));
        n.push(tc(&format!("hm-sv{i}t"), cx - 60.0, 638.0, 120.0, label, 11.0, M_INK, false));
    }
    // 6. Amazing red.
    let amazing_items: Vec<Product> = [prods[1].clone(), prods[4].clone(), prods[6].clone(), prods[7].clone(), prods[10].clone(), prods[8].clone(), extras[4].clone()].to_vec();
    n.extend(amazing_section("az1", 706.0, M_AMAZING, "پیشنهاد شگفت‌انگیز", &amazing_items));
    // 7. Banner grid 1.
    n.extend(banner_row4("bg1", 1060.0, [("tv.jpg", "جشنواره تلویزیون"), ("fashion.jpg", "حراج پوشاک تابستانه"), ("kitchen.jpg", "آشپزخانه مدرن"), ("sport.jpg", "ورزش در خانه")]));
    // 8. Three hour delivery strip (cream).
    n.push(rect("hm-cream", "Three hour delivery", MX, 1314.0, CW, 276.0, M_CREAM, 16.0));
    n.push(t("hm-cream-t", 700.0, 1332.0, 640.0, "همین حالا سفارش بده، ۳ ساعته تحویل بگیر", 18.0, M_INK, true));
    n.push(t("hm-cream-s", 700.0, 1364.0, 640.0, "با انتخاب آدرس، کالاهای سوپرمارکتی منتخب در کمتر از سه ساعت می‌رسند", 12.0, M_MUTED, false));
    n.push(btn("hm-cream-b", MX + 20.0, 1336.0, 130.0, 38.0, "انتخاب آدرس", 13.0, Color::WHITE, M_INK, 10.0));
    let cream_items: Vec<Product> = [extras[8].clone(), prods[9].clone(), prods[8].clone(), extras[9].clone(), extras[11].clone(), prods[10].clone(), extras[13].clone()].to_vec();
    let cw7 = 168.0;
    let start7 = MX + CW - 24.0 - cw7;
    for (i, p) in cream_items.iter().enumerate() {
        let cx = start7 - (i as f32) * (cw7 + 10.0);
        n.extend(photo_card_mini(&format!("hm-cr{i}"), cx, 1396.0, cw7, 178.0, p));
    }
    // 9. Banner grid 2.
    n.extend(banner_row4("bg2", 1614.0, [("coffee.jpg", "صبح با طعم قهوه"), ("laptop.jpg", "بازگشت به مدرسه"), ("jewellery.jpg", "طلای بدون اجرت"), ("toy.jpg", "دنیای اسباب‌بازی")]));
    // 10. Amazing green (supermarket).
    let green_items: Vec<Product> = [extras[8].clone(), extras[9].clone(), extras[11].clone(), prods[9].clone(), extras[6].clone(), prods[8].clone(), extras[12].clone()].to_vec();
    n.extend(amazing_section("az2", 1868.0, M_GREEN, "شگفت‌انگیز سوپرمارکتی", &green_items));
    // 11. Categories, 18 photo circles in 2 rows of 9.
    let cats: [(&str, &str); 18] = [
        ("موبایل", "phone.jpg"), ("لپ‌تاپ", "laptop.jpg"), ("کالای دیجیتال", "headphones.jpg"),
        ("خانه و آشپزخانه", "kitchen.jpg"), ("لوازم خانگی", "washer.jpg"), ("زیبایی و سلامت", "perfume.jpg"),
        ("مد و پوشاک", "fashion.jpg"), ("طلا و نقره", "jewellery.jpg"), ("خودرو و موتور", "car.jpg"),
        ("ابزار و تجهیزات", "drill.jpg"), ("پزشکی و سلامت", "medical.jpg"), ("کتاب و شهر هنر", "book.jpg"),
        ("ورزش و سفر", "sport.jpg"), ("کارت هدیه", "gift.jpg"), ("کالای سوپرمارکتی", "groceries.jpg"),
        ("اسباب‌بازی و کودک", "toy.jpg"), ("پت‌شاپ", "pet.jpg"), ("صنایع دستی", "fashion.jpg"),
    ];
    for (i, (label, file)) in cats.iter().enumerate() {
        let row = (i / 9) as f32;
        let col = (i % 9) as f32;
        let cx = 1375.0 - col * 145.5 - 128.0;
        let cy = 2236.0 + row * 158.0;
        let mut c = img(&format!("hm-cat{i}"), file, cx, cy, 104.0, 104.0, 52.0);
        c.kind = NodeKind::Ellipse;
        n.push(c);
        n.push(tc(&format!("hm-cat{i}t"), cx - 16.0, cy + 112.0, 136.0, label, 13.0, M_INK, false));
    }
    // 12. Banner row 3.
    n.extend(banner_row2("br3", 2580.0, 230.0, [("fridge.jpg", "یخچال‌های نسل جدید"), ("sneakers.jpg", "دویدن را شروع کن")], false));
    // 13. Product rows with a banner row between them.
    let vitamins: Vec<Product> = [prods[9].clone(), extras[11].clone(), prods[8].clone(), extras[0].clone(), prods[11].clone(), extras[13].clone(), prods[10].clone()].to_vec();
    n.extend(product_row("rw1", 2834.0, "ویتامین‌ها و مکمل‌ها", "بر اساس سلیقه و جستجوهای شما", &vitamins));
    let cases: Vec<Product> = [prods[10].clone(), prods[2].clone(), prods[0].clone(), prods[1].clone(), extras[4].clone(), prods[4].clone(), extras[6].clone()].to_vec();
    n.extend(product_row("rw2", 3210.0, "قاب و کاور گوشی", "محافظ گوشی شما، با هر سلیقه‌ای", &cases));
    n.extend(banner_row2("br4", 3586.0, 200.0, [("watch.jpg", "ساعت‌های هوشمند"), ("sneakers.jpg", "کفش‌های رانینگ")], true));
    let phones_audio: Vec<Product> = [prods[1].clone(), prods[6].clone(), extras[3].clone(), prods[0].clone(), prods[3].clone(), extras[10].clone(), prods[7].clone()].to_vec();
    n.extend(product_row("rw3", 3810.0, "هدفون و هدست", "صدای شفاف برای موسیقی و تماس", &phones_audio));
    let mobiles: Vec<Product> = [prods[2].clone(), prods[0].clone(), prods[3].clone(), prods[10].clone(), extras[6].clone(), prods[1].clone(), extras[12].clone()].to_vec();
    n.extend(product_row("rw4", 4186.0, "گوشی‌های موبایل", "جدیدترین مدل‌ها با گارانتی معتبر", &mobiles));
    // 14. Popular brands.
    n.push(outlined("hm-brands", "Popular brands", MX, 4562.0, CW, 196.0, Color::WHITE, 16.0));
    n.push(t("hm-brands-t", 751.0, 4580.0, 600.0, "✧ محبوب‌ترین برندها", 19.0, M_INK, true));
    let brands = ["Samsung", "Apple", "Xiaomi", "LG", "Sony", "Nike", "Adidas", "Bosch", "HP", "Lenovo"];
    for (i, b) in brands.iter().enumerate() {
        let bx = 1375.0 - (i as f32 + 1.0) * 124.0 + 6.0;
        n.push(outlined(&format!("hm-br{i}"), "Brand card", bx, 4624.0, 112.0, 112.0, Color::WHITE, 10.0));
        n.push(ellipse(&format!("hm-br{i}c"), bx + 36.0, 4638.0, 40.0, 40.0, M_PILL));
        n.push(tc(&format!("hm-br{i}m"), bx + 36.0, 4648.0, 40.0, &b[..1], 16.0, M_MUTED, true));
        n.push(tc(&format!("hm-br{i}t"), bx + 6.0, 4688.0, 100.0, b, 13.0, M_INK, true));
    }
    // 15 and 16. Ranked grids.
    let mut all: Vec<Product> = prods.to_vec();
    all.extend(extras.iter().cloned());
    n.extend(ranked_section("rk1", 4782.0, "پرفروش‌ترین‌ها", &all));
    let mut hottest = all.clone();
    hottest.reverse();
    n.extend(ranked_section("rk2", 5418.0, "پیشنهاد لحظه‌ای، داغ‌ترین‌های چند ساعت گذشته", &hottest));
    // 17. Our suggestion, 5 x 6 divider grid.
    n.push(t("hm-sg-title", 675.0, 6054.0, 700.0, "پیشنهاد بازارینو برای شما", 19.0, M_INK, true));
    let mut sugg: Vec<Product> = all.clone();
    sugg.push(prod("مچ‌بند هوشمند فیت‌پرو", "فیت‌پرو", 890_000, 20, 4.2, "۳۴۵", "watch.jpg"));
    sugg.push(prod("گلس و قاب محافظ آیفون", "‌اپیکوی", 150_000, 35, 4.0, "۱٬۰۲۰", "phonecase.jpg"));
    sugg.push(prod("کفش پیاده‌روی روزانه", "‌اسپورتینو", 1_680_000, 30, 4.3, "۵۴۳", "sneakers.jpg"));
    sugg.push(prod("لوسیون بدن آبرسان", "نیوآ", 290_000, 18, 4.4, "۸۷۶", "cream.jpg"));
    let cell_w = 262.0;
    let cell_h = 296.0;
    let gx = MX;
    let gy = 6098.0;
    n.push(rect("hm-sg-v0", "", gx, gy, 1.0, cell_h * 6.0, M_LINE, 0.0));
    for c in 0..5 {
        n.push(rect(&format!("hm-sg-v{}", c + 1), "", gx + (c as f32 + 1.0) * cell_w, gy, 1.0, cell_h * 6.0, M_LINE, 0.0));
    }
    for r in 0..6 {
        n.push(rect(&format!("hm-sg-h{r}"), "", gx, gy + (r as f32) * cell_h, CW, 1.0, M_LINE, 0.0));
    }
    n.push(rect("hm-sg-h6", "", gx, gy + 6.0 * cell_h, CW, 1.0, M_LINE, 0.0));
    for idx in 0..30 {
        let p = &sugg[idx % sugg.len()];
        let col = (idx % 5) as f32;
        let row = (idx / 5) as f32;
        // RTL: first item at the right.
        let cx = gx + CW - (col + 1.0) * cell_w;
        let cy = gy + row * cell_h;
        n.extend(suggestion_card(&format!("hm-sg{idx}"), cx, cy, cell_w, cell_h, p, idx % 7 == 3));
    }
    n.push(btn("hm-sg-more", 640.0, gy + 6.0 * cell_h + 22.0, 160.0, 40.0, "مشاهده بیشتر", 13.0, M_PILL, M_INK, 20.0));
    // 18. Footer.
    let mut footer = market_footer("hm", 7952.0);
    n.append(&mut footer.children);
    page.root.children = n;
    page
}

fn market_category(prods: &[Product], extras: &[Product]) -> Page {
    let mut page = Page::new("category", "دسته‌بندی موبایل", "/category", 1440.0, 1560.0);
    page.background = Some(Color::WHITE);
    let mut n: Vec<Node> = Vec::new();
    n.extend(market_header("ct"));
    n.push(t("ct-crumb", 475.0, 178.0, 900.0, "بازارینو / کالای دیجیتال / گوشی موبایل", 12.0, M_MUTED, false));
    n.push(t("ct-title", 575.0, 204.0, 800.0, "گوشی‌های موبایل", 22.0, M_INK, true));
    // Filter sidebar (right in RTL).
    n.push(outlined("ct-side", "Filters", 1135.0, 250.0, 240.0, 560.0, Color::WHITE, 12.0));
    n.push(t("ct-side-t", 1159.0, 268.0, 190.0, "فیلترها", 15.0, M_INK, true));
    let filters = ["برند: سامسونگ، اپل، شیائومی", "محدوده قیمت", "حافظه: ۱۲۸ و ۲۵۶ گیگ", "فقط کالاهای موجود", "فقط پیشنهاد شگفت‌انگیز"];
    for (i, f) in filters.iter().enumerate() {
        n.push(t(&format!("ct-f{i}"), 1159.0, 306.0 + (i as f32) * 44.0, 195.0, &format!("▢ {f}"), 13.0, M_INK, false));
        n.push(rect(&format!("ct-fl{i}"), "", 1159.0, 336.0 + (i as f32) * 44.0, 192.0, 1.0, M_LINE, 0.0));
    }
    // Sort bar and photo grid.
    n.push(outlined("ct-sort", "Sort bar", MX, 250.0, 1050.0, 52.0, Color::WHITE, 12.0));
    n.push(t("ct-sort-t", 89.0, 264.0, 1000.0, "مرتب‌سازی: پربازدیدترین | ارزان‌ترین | گران‌ترین | پرفروش‌ترین", 13.0, M_INK, false));
    let mut grid: Vec<Product> = prods.to_vec();
    grid.extend(extras.iter().cloned());
    for idx in 0..12 {
        let p = &grid[idx];
        let col = (idx % 4) as f32;
        let row = (idx / 4) as f32;
        let cx = MX + 1050.0 - (col + 1.0) * 258.0 + 6.0;
        let cy = 322.0 + row * 342.0;
        n.extend(photo_card(&format!("ct-c{idx}"), cx, cy, 252.0, 326.0, p, true));
    }
    let mut footer = market_footer("ct", 1400.0);
    footer.children.retain(|c| !c.id.contains("fgroup") && !c.id.contains("fcopy"));
    n.append(&mut footer.children);
    page.root.children = n;
    page
}

fn market_product(prods: &[Product]) -> Page {
    let p = &prods[0];
    let mut page = Page::new("product", "صفحه محصول", "/product", 1440.0, 1420.0);
    page.background = Some(Color::WHITE);
    let mut n: Vec<Node> = Vec::new();
    n.extend(market_header("pd"));
    n.push(t("pd-crumb", 375.0, 178.0, 1000.0, "بازارینو / کالای دیجیتال / ساعت هوشمند / شیائومی", 12.0, M_MUTED, false));
    // Gallery: main photo plus thumbs (right in RTL).
    n.push(outlined("pd-main", "Main photo", 855.0, 214.0, 520.0, 520.0, Color::WHITE, 12.0));
    n.push(img("pd-main-img", p.img, 865.0, 224.0, 500.0, 500.0, 8.0));
    let thumbs = ["watch.jpg", "phone.jpg", "headphones.jpg", "laptop.jpg"];
    for (i, th) in thumbs.iter().enumerate() {
        let tx = 1291.0 - (i as f32) * 88.0;
        n.push(outlined(&format!("pd-th{i}"), "Thumb", tx, 750.0, 76.0, 76.0, Color::WHITE, 10.0));
        n.push(img(&format!("pd-th{i}i"), th, tx + 5.0, 755.0, 66.0, 66.0, 6.0));
    }
    // Title and specs (middle).
    n.push(t("pd-title", 395.0, 222.0, 440.0, p.name, 21.0, M_INK, true));
    n.push(t("pd-brand", 395.0, 262.0, 440.0, &format!("برند: {} | دسته: ساعت و مچ‌بند هوشمند", p.brand), 13.0, M_MUTED, false));
    n.push(t("pd-rate", 395.0, 292.0, 440.0, &format!("★ {} ({} دیدگاه) | ۱۲ پرسش پاسخ داده شده", fa_rating(p.rating), p.reviews), 13.0, Color::rgb(240, 164, 0), true));
    let specs = [
        ("رنگ", "مشکی، نقره‌ای، سبز"),
        ("مقاومت در برابر آب", "۵ اتمسفر"),
        ("سازگاری", "اندروید و iOS"),
        ("باتری", "تا ۱۴ روز شارژدهی"),
        ("گارانتی", "۱۸ ماه گارانتی معتبر"),
    ];
    for (i, (k, v)) in specs.iter().enumerate() {
        let sy = 340.0 + (i as f32) * 40.0;
        n.push(rect(&format!("pd-sp{i}"), "", 395.0, sy, 440.0, 34.0, M_PILL, 8.0));
        n.push(t(&format!("pd-sp{i}k"), 415.0, sy + 7.0, 130.0, k, 12.0, M_MUTED, false));
        n.push(t(&format!("pd-sp{i}v"), 555.0, sy + 7.0, 260.0, v, 12.0, M_INK, true));
    }
    // Buy box (left).
    n.push(outlined("pd-buy", "Buy box", MX, 214.0, 300.0, 340.0, Color::rgb(250, 250, 251), 12.0));
    n.push(t("pd-buy-1", 89.0, 234.0, 260.0, "✓ گارانتی اصالت و سلامت فیزیکی", 13.0, M_INK, false));
    n.push(t("pd-buy-2", 89.0, 268.0, 260.0, "✓ موجود در انبار بازارینو", 13.0, M_INK, false));
    n.push(t("pd-buy-3", 89.0, 302.0, 260.0, "ارسال اکسپرس: امروز تا ساعت ۱۸", 13.0, M_MUTED, false));
    n.push(t("pd-buy-old", 89.0, 356.0, 180.0, &fa_price(p.price), 14.0, M_MUTED, false));
    n.push(rect("pd-buy-strike", "", 262.0, 365.0, 64.0, 1.0, M_MUTED, 0.0));
    n.push(btn("pd-buy-off", 285.0, 350.0, 44.0, 24.0, &format!("٪{}", fa_num(p.off)), 13.0, M_RED, Color::WHITE, 12.0));
    n.push(t("pd-buy-price", 89.0, 384.0, 260.0, &format!("{} تومان", fa_price(final_price(p))), 20.0, M_INK, true));
    n.push(btn("pd-buy-add", 89.0, 436.0, 260.0, 50.0, "افزودن به سبد خرید", 15.0, M_RED, Color::WHITE, 12.0));
    n.push(btn("pd-buy-fast", 89.0, 496.0, 260.0, 44.0, "خرید فوری", 14.0, Color::WHITE, M_RED, 10.0));
    // Similar products with photos.
    n.push(t("pd-sim-t", 675.0, 880.0, 700.0, "کالاهای مشابه", 19.0, M_INK, true));
    for (i, sp) in prods.iter().skip(1).take(4).enumerate() {
        let cx = 1375.0 - (i as f32 + 1.0) * 322.0 + 8.0;
        n.extend(photo_card(&format!("pd-sim{i}"), cx, 926.0, 314.0, 330.0, sp, true));
    }
    let mut footer = market_footer("pd", 1300.0);
    footer.children.retain(|c| !c.id.contains("fgroup") && !c.id.contains("fcopy"));
    n.append(&mut footer.children);
    page.root.children = n;
    page
}

fn market_cart(prods: &[Product]) -> Page {
    let mut page = Page::new("cart", "سبد خرید", "/cart", 1440.0, 1180.0);
    page.background = Some(Color::WHITE);
    let mut n: Vec<Node> = Vec::new();
    n.extend(market_header("cr"));
    n.push(t("cr-title", 775.0, 182.0, 600.0, "سبد خرید شما", 22.0, M_INK, true));
    // Items with real photos.
    let items = [&prods[0], &prods[1], &prods[8]];
    let mut subtotal: i64 = 0;
    for (i, p) in items.iter().enumerate() {
        let iy = 232.0 + (i as f32) * 168.0;
        subtotal += final_price(p);
        n.push(outlined(&format!("cr-it{i}"), "Cart item", 435.0, iy, 940.0, 152.0, Color::WHITE, 12.0));
        n.push(img(&format!("cr-it{i}img"), p.img, 1245.0, iy + 16.0, 120.0, 120.0, 8.0));
        n.push(t(&format!("cr-it{i}t"), 760.0, iy + 22.0, 460.0, p.name, 15.0, M_INK, true));
        n.push(t(&format!("cr-it{i}m"), 760.0, iy + 54.0, 460.0, "گارانتی اصالت کالا | فروشنده: بازارینو", 12.0, M_MUTED, false));
        n.push(outlined(&format!("cr-it{i}q"), "Quantity", 1210.0, iy + 100.0, 130.0, 38.0, Color::WHITE, 8.0));
        n.push(tc(&format!("cr-it{i}qt"), 1210.0, iy + 108.0, 130.0, "−  ۱  +", 14.0, M_INK, true));
        n.push(t(&format!("cr-it{i}p"), 470.0, iy + 104.0, 240.0, &format!("{} تومان", fa_price(final_price(p))), 16.0, M_INK, true));
    }
    // Summary (left).
    n.push(outlined("cr-sum", "Summary", MX, 232.0, 340.0, 330.0, Color::rgb(250, 250, 251), 12.0));
    n.push(t("cr-sum-1", 89.0, 256.0, 300.0, &format!("جمع اقلام: {} تومان", fa_price(subtotal)), 14.0, M_INK, false));
    n.push(t("cr-sum-2", 89.0, 292.0, 300.0, "هزینه ارسال: رایگان", 14.0, M_INK, false));
    n.push(t("cr-sum-3", 89.0, 328.0, 300.0, &format!("سود شما از این خرید: {} تومان", fa_price(950_000)), 14.0, M_RED, true));
    n.push(rect("cr-sum-line", "", 89.0, 366.0, 292.0, 1.0, M_LINE, 0.0));
    n.push(t("cr-sum-4", 89.0, 380.0, 300.0, &format!("مبلغ قابل پرداخت: {} تومان", fa_price(subtotal)), 16.0, M_INK, true));
    n.push(btn("cr-sum-b", 89.0, 428.0, 292.0, 50.0, "ادامه و ثبت سفارش", 15.0, M_RED, Color::WHITE, 12.0));
    n.push(t("cr-note", 470.0, 756.0, 880.0, "کالاهای سبد خرید رزرو نمی‌شوند؛ برای نهایی کردن خرید، ثبت سفارش را کامل کنید.", 12.0, M_MUTED, false));
    let mut footer = market_footer("cr", 820.0);
    footer.children.retain(|c| !c.id.contains("fgroup") && !c.id.contains("fcopy"));
    n.append(&mut footer.children);
    page.root.children = n;
    page
}

/// The bundled photo asset files of the marketplace template, embedded
/// so `chida new --template digikala` can materialise them next to the
/// generated project. Credits live in templates/assets/digikala/SOURCES.md.
pub fn digikala_assets() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("watch.jpg", include_bytes!("../../../templates/assets/digikala/watch.jpg")),
        ("headphones.jpg", include_bytes!("../../../templates/assets/digikala/headphones.jpg")),
        ("phone.jpg", include_bytes!("../../../templates/assets/digikala/phone.jpg")),
        ("laptop.jpg", include_bytes!("../../../templates/assets/digikala/laptop.jpg")),
        ("sneakers.jpg", include_bytes!("../../../templates/assets/digikala/sneakers.jpg")),
        ("tv.jpg", include_bytes!("../../../templates/assets/digikala/tv.jpg")),
        ("coffee.jpg", include_bytes!("../../../templates/assets/digikala/coffee.jpg")),
        ("backpack.jpg", include_bytes!("../../../templates/assets/digikala/backpack.jpg")),
        ("cream.jpg", include_bytes!("../../../templates/assets/digikala/cream.jpg")),
        ("vitamins.jpg", include_bytes!("../../../templates/assets/digikala/vitamins.jpg")),
        ("phonecase.jpg", include_bytes!("../../../templates/assets/digikala/phonecase.jpg")),
        ("book.jpg", include_bytes!("../../../templates/assets/digikala/book.jpg")),
        ("perfume.jpg", include_bytes!("../../../templates/assets/digikala/perfume.jpg")),
        ("fridge.jpg", include_bytes!("../../../templates/assets/digikala/fridge.jpg")),
        ("washer.jpg", include_bytes!("../../../templates/assets/digikala/washer.jpg")),
        ("drill.jpg", include_bytes!("../../../templates/assets/digikala/drill.jpg")),
        ("umbrella.jpg", include_bytes!("../../../templates/assets/digikala/umbrella.jpg")),
        ("toy.jpg", include_bytes!("../../../templates/assets/digikala/toy.jpg")),
        ("gift.jpg", include_bytes!("../../../templates/assets/digikala/gift.jpg")),
        ("jewellery.jpg", include_bytes!("../../../templates/assets/digikala/jewellery.jpg")),
        ("groceries.jpg", include_bytes!("../../../templates/assets/digikala/groceries.jpg")),
        ("kitchen.jpg", include_bytes!("../../../templates/assets/digikala/kitchen.jpg")),
        ("sport.jpg", include_bytes!("../../../templates/assets/digikala/sport.jpg")),
        ("medical.jpg", include_bytes!("../../../templates/assets/digikala/medical.jpg")),
        ("car.jpg", include_bytes!("../../../templates/assets/digikala/car.jpg")),
        ("fashion.jpg", include_bytes!("../../../templates/assets/digikala/fashion.jpg")),
        ("pet.jpg", include_bytes!("../../../templates/assets/digikala/pet.jpg")),
        ("hero.jpg", include_bytes!("../../../templates/assets/digikala/hero.jpg")),
    ]
}

pub fn digikala_marketplace() -> ProjectFile {
    let prods = products();
    let extras = extra_products();
    let mut project = Project::new("marketplace-pro", "Marketplace Pro");
    project.description =
        "Persian RTL marketplace template mirroring the layout of large Iranian e-commerce homepages. Original sample brand (Bazarino) and sample data; product photos are openly licensed, see templates/assets/digikala/SOURCES.md.".into();
    project.theme.primary = M_RED;
    project.pages = vec![
        market_home(&prods, &extras),
        market_category(&prods, &extras),
        market_product(&prods),
        market_cart(&prods),
    ];

    // Database.
    let mut categories = Table::new("categories");
    categories.label = "Categories".into();
    categories = categories
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("products_count", ColumnType::Integer));
    for (name, count) in [
        ("کالای دیجیتال", 12000),
        ("لپ تاپ و کامپیوتر", 8500),
        ("خانه و آشپزخانه", 23000),
        ("مد و پوشاک", 31000),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("products_count".into(), json!(count));
        categories.seed_rows.push(m);
    }
    let mut products_t = Table::new("products");
    products_t.label = "Products".into();
    products_t = products_t
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("price", ColumnType::Money).required())
        .column(Column::new("discount_percent", ColumnType::Integer))
        .column(Column::new("stock", ColumnType::Integer).required())
        .column(Column::new("rating", ColumnType::Real))
        .column(
            Column::new("category_id", ColumnType::Relation)
                .required()
                .relates_to("categories"),
        );
    for (i, p) in prods.iter().enumerate() {
        let mut m = Map::new();
        m.insert("name".into(), json!(p.name));
        m.insert("price".into(), json!(p.price));
        m.insert("discount_percent".into(), json!(p.off));
        m.insert("stock".into(), json!(24 + (i as i64) * 3));
        m.insert("rating".into(), json!(p.rating));
        m.insert("category_id".into(), json!((i % 4) + 1));
        products_t.seed_rows.push(m);
    }
    let orders = Table::new("orders")
        .column(
            Column::new("product_id", ColumnType::Relation)
                .required()
                .relates_to("products"),
        )
        .column(Column::new("qty", ColumnType::Integer).required())
        .column(
            Column::new("user_id", ColumnType::Relation)
                .required()
                .relates_to("users"),
        )
        .column(Column::new("total", ColumnType::Money).required());
    let mut users = Table::new("users");
    users.label = "Users".into();
    users = users
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("email", ColumnType::Text).required())
        .column(Column::new("phone", ColumnType::Text));
    for (name, email, phone) in [
        ("سارا محمدی", "sara@example.ir", "09120000001"),
        ("علی رضایی", "ali@example.ir", "09120000002"),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("email".into(), json!(email));
        m.insert("phone".into(), json!(phone));
        users.seed_rows.push(m);
    }
    let database = Database {
        tables: vec![categories, products_t, orders, users],
        relations: vec![
            Relation {
                name: "product_category".into(),
                from_table: "products".into(),
                from_column: "category_id".into(),
                to_table: "categories".into(),
                to_column: "id".into(),
                kind: RelationKind::OneToMany,
            },
            Relation {
                name: "order_product".into(),
                from_table: "orders".into(),
                from_column: "product_id".into(),
                to_table: "products".into(),
                to_column: "id".into(),
                kind: RelationKind::OneToMany,
            },
            Relation {
                name: "order_user".into(),
                from_table: "orders".into(),
                from_column: "user_id".into(),
                to_table: "users".into(),
                to_column: "id".into(),
                kind: RelationKind::OneToMany,
            },
        ],
    };

    // Flows: add to cart, plus a search stub wired to the API action.
    let mut mapping = Map::new();
    mapping.insert("product_id".into(), json!("$product_id"));
    mapping.insert("qty".into(), json!(1));
    mapping.insert("user_id".into(), json!("$user_id"));
    mapping.insert("total".into(), json!("$price"));
    let add_to_cart = Flow {
        id: "add-to-cart".into(),
        name: "Add to cart".into(),
        trigger: Event {
            kind: EventKind::Click,
            source_node: Some("pd-buy-add".into()),
            page_route: Some("/product".into()),
        },
        steps: vec![Step::Condition {
            when: Expr::Compare {
                field: "stock".into(),
                op: CompareOp::Gt,
                value: json!(0),
            },
            then_steps: vec![
                Step::Action {
                    action: Action::SaveRecord {
                        table: "orders".into(),
                        mapping,
                    },
                },
                Step::Action {
                    action: Action::ShowMessage {
                        text: "به سبد خرید اضافه شد".into(),
                    },
                },
                Step::Action {
                    action: Action::Navigate {
                        route: "/cart".into(),
                    },
                },
            ],
            else_steps: vec![Step::Action {
                action: Action::ShowMessage {
                    text: "این کالا ناموجود است".into(),
                },
            }],
        }],
    };
    let search = Flow {
        id: "search".into(),
        name: "Search products".into(),
        trigger: Event {
            kind: EventKind::Submit,
            source_node: Some("hm-search".into()),
            page_route: Some("/".into()),
        },
        steps: vec![
            Step::Action {
                action: Action::CallApi {
                    url: "/api/products/search".into(),
                    method: "GET".into(),
                },
            },
            Step::Action {
                action: Action::Navigate {
                    route: "/category".into(),
                },
            },
        ],
    };
    ProjectFile {
        project,
        database,
        flows: vec![add_to_cart, search],
    }
}


const GLASS_HINT: &str = "backdrop-filter: blur(18px) saturate(160%)";

fn glass(id: &str, name: &str, x: f32, y: f32, w: f32, h: f32, radius: f32, alpha: u8) -> Node {
    let mut n = rect(id, name, x, y, w, h, Color::rgba(255, 255, 255, alpha), radius);
    n.stroke = Some(Color::rgba(255, 255, 255, 130));
    n.stroke_width = 1.0;
    n.props
        .insert("style_hint".into(), GLASS_HINT.into());
    n
}

const G_SUB: Color = Color::rgb(203, 209, 240);
const G_FAINT: Color = Color::rgb(168, 175, 214);

fn glass_nav(prefix: &str) -> Node {
    let mut nav = glass(
        &format!("{prefix}-nav"),
        "Glass capsule nav",
        370.0,
        28.0,
        700.0,
        56.0,
        28.0,
        36,
    );
    nav.children = vec![
        t(&format!("{prefix}-nav-logo"), 24.0, 13.0, 200.0, "Chida Glass", 17.0, Color::WHITE, true),
        t(&format!("{prefix}-nav-links"), 250.0, 16.0, 424.0, "Product      Pricing      Docs      Blog", 13.0, G_SUB, false),
    ];
    nav
}

fn glass_landing() -> Page {
    let mut page = Page::new("landing", "Glass landing", "/", 1440.0, 980.0);
    page.background = Some(Color::rgb(23, 18, 46));
    let mut root = vec![
        rect("g-bg", "Backdrop", 0.0, 0.0, 1440.0, 980.0, Color::rgb(23, 18, 46), 0.0),
        ellipse("g-glow-1", -100.0, -120.0, 560.0, 420.0, Color::rgba(124, 58, 237, 110)),
        ellipse("g-glow-2", 1000.0, -80.0, 520.0, 420.0, Color::rgba(56, 150, 255, 100)),
        ellipse("g-glow-3", 480.0, 580.0, 640.0, 420.0, Color::rgba(255, 90, 160, 66)),
        ellipse("g-glow-4", 1180.0, 540.0, 380.0, 300.0, Color::rgba(45, 220, 190, 66)),
        glass_nav("g"),
    ];
    let mut hero = glass("g-hero", "Glass hero card", 270.0, 140.0, 900.0, 480.0, 32.0, 30);
    hero.children = vec![
        glass("g-badge", "Badge", 330.0, 42.0, 240.0, 34.0, 17.0, 50),
        t("g-badge-t", 354.0, 49.0, 200.0, "New: Chida 1.0 is here", 12.0, Color::WHITE, false),
        t("g-hero-title", 70.0, 106.0, 760.0, "Design at the speed of thought", 46.0, Color::WHITE, true),
        t("g-hero-sub", 70.0, 186.0, 700.0, "Build full stack apps visually and ship real Rust code. No lock-in, no heavy runtime.", 18.0, G_SUB, false),
        btn("g-cta-1", 70.0, 268.0, 190.0, 52.0, "Start building", 15.0, Color::rgba(255, 255, 255, 235), Color::rgb(23, 18, 46), 26.0),
        {
            let mut b = btn("g-cta-2", 280.0, 268.0, 170.0, 52.0, "Watch demo", 15.0, Color::rgba(255, 255, 255, 40), Color::WHITE, 26.0);
            b.stroke = Some(Color::rgba(255, 255, 255, 130));
            b.stroke_width = 1.0;
            b
        },
        glass("g-stat-1", "Stat", 70.0, 356.0, 210.0, 76.0, 18.0, 28),
        t("g-stat-1-v", 94.0, 366.0, 170.0, "100% Rust", 18.0, Color::WHITE, true),
        t("g-stat-1-l", 94.0, 396.0, 170.0, "core and codegen", 11.0, G_FAINT, false),
        glass("g-stat-2", "Stat", 300.0, 356.0, 210.0, 76.0, 18.0, 28),
        t("g-stat-2-v", 324.0, 366.0, 170.0, "3.4 MB", 18.0, Color::WHITE, true),
        t("g-stat-2-l", 324.0, 396.0, 170.0, "single binary", 11.0, G_FAINT, false),
        glass("g-stat-3", "Stat", 530.0, 356.0, 210.0, 76.0, 18.0, 28),
        t("g-stat-3-v", 554.0, 366.0, 170.0, "1 file", 18.0, Color::WHITE, true),
        t("g-stat-3-l", 554.0, 396.0, 170.0, "project format", 11.0, G_FAINT, false),
    ];
    root.push(hero);
    let feats: [(&str, &str); 3] = [
        ("Visual canvas", "Drag blocks, not layers. Pages and\ncomponents without the learning curve."),
        ("Real database", "Tables, relations and validation,\ndesigned visually, generated as schema."),
        ("Rust code out", "A full Axum and Leptos project you\nown, plus PNG and SVG renders."),
    ];
    for (i, (title, body)) in feats.iter().enumerate() {
        let mut card = glass(
            &format!("g-feat-{i}"),
            "Feature card",
            40.0 + i as f32 * 467.0,
            664.0,
            426.0,
            190.0,
            24.0,
            24,
        );
        card.children = vec![
            t(&format!("g-feat-{i}-t"), 26.0, 24.0, 374.0, title, 19.0, Color::WHITE, true),
            t(&format!("g-feat-{i}-b"), 26.0, 64.0, 374.0, body, 13.0, G_SUB, false),
        ];
        root.push(card);
    }
    root.push(t("g-foot", 40.0, 896.0, 1360.0, "Chida Glass, a liquid glass template family made with Chida.", 12.0, G_FAINT, false));
    page.root.children = root;
    page
}

fn glass_dashboard() -> Page {
    let mut page = Page::new("dashboard", "Glass dashboard", "/dashboard", 1440.0, 900.0);
    page.background = Some(Color::rgb(13, 20, 36));
    let mut root = vec![
        rect("d-bg", "Backdrop", 0.0, 0.0, 1440.0, 900.0, Color::rgb(13, 20, 36), 0.0),
        ellipse("d-glow-1", -80.0, -100.0, 480.0, 360.0, Color::rgba(45, 150, 220, 84)),
        ellipse("d-glow-2", 1060.0, 480.0, 520.0, 380.0, Color::rgba(45, 220, 190, 60)),
        ellipse("d-glow-3", 620.0, -140.0, 420.0, 300.0, Color::rgba(124, 58, 237, 70)),
    ];
    // Glass sidebar.
    let mut side = glass("d-side", "Glass sidebar", 24.0, 24.0, 250.0, 852.0, 24.0, 26);
    let mut schildren = vec![t("d-brand", 24.0, 22.0, 200.0, "Chida", 20.0, Color::WHITE, true)];
    for (i, item) in ["Overview", "Orders", "Products", "Customers", "Analytics", "Settings"].iter().enumerate() {
        let y = 84.0 + i as f32 * 54.0;
        if i == 0 {
            schildren.push(glass(&format!("d-nav-{i}-pill"), "Active nav", 14.0, y - 6.0, 222.0, 40.0, 12.0, 60));
        }
        schildren.push(t(&format!("d-nav-{i}"), 24.0, y, 200.0, item, 14.0, Color::WHITE, i == 0));
    }
    schildren.push(t("d-side-note", 24.0, 760.0, 200.0, "Glass OS workspace", 11.0, G_FAINT, false));
    side.children = schildren;
    root.push(side);
    // Top bar.
    let mut top = glass("d-top", "Glass topbar", 298.0, 24.0, 1118.0, 64.0, 20.0, 26);
    top.children = vec![
        t("d-top-title", 24.0, 17.0, 300.0, "Overview", 18.0, Color::WHITE, true),
        glass("d-search", "Search", 640.0, 12.0, 300.0, 40.0, 20.0, 36),
        t("d-search-t", 664.0, 20.0, 260.0, "Search anything", 13.0, G_SUB, false),
        ellipse("d-avatar", 1046.0, 10.0, 44.0, 44.0, Color::rgba(45, 220, 190, 190)),
    ];
    root.push(top);
    // KPI cards.
    let kpis: [(&str, &str); 4] = [
        ("Revenue", "$48,200"),
        ("Orders", "1,248"),
        ("Customers", "3,914"),
        ("Conversion", "3.6%"),
    ];
    for (i, (label, value)) in kpis.iter().enumerate() {
        let mut k = glass(&format!("d-kpi-{i}"), "KPI card", 298.0 + i as f32 * 286.0, 112.0, 262.0, 130.0, 20.0, 30);
        k.children = vec![
            t(&format!("d-kpi-{i}-l"), 22.0, 20.0, 218.0, label, 12.0, G_FAINT, false),
            t(&format!("d-kpi-{i}-v"), 22.0, 48.0, 218.0, value, 26.0, Color::WHITE, true),
            t(&format!("d-kpi-{i}-s"), 22.0, 92.0, 218.0, "Sample data", 11.0, G_FAINT, false),
        ];
        root.push(k);
    }
    // Chart card with bars.
    let mut chart = glass("d-chart", "Chart card", 298.0, 266.0, 700.0, 380.0, 24.0, 24);
    let mut cchildren = vec![t("d-chart-t", 26.0, 20.0, 400.0, "Revenue, last 12 weeks", 16.0, Color::WHITE, true)];
    let heights = [70.0, 96.0, 84.0, 120.0, 108.0, 140.0, 126.0, 158.0, 146.0, 182.0, 168.0, 205.0];
    for (i, h) in heights.iter().enumerate() {
        cchildren.push(rect(
            &format!("d-bar-{i}"),
            "Chart bar",
            40.0 + i as f32 * 52.0,
            330.0 - h,
            36.0,
            *h,
            Color::rgba(94, 180, 255, 180),
            6.0,
        ));
    }
    chart.children = cchildren;
    root.push(chart);
    // Top products card.
    let mut top_prod = glass("d-topprod", "Top products", 1022.0, 266.0, 394.0, 380.0, 24.0, 24);
    let mut tchildren = vec![t("d-topprod-t", 24.0, 20.0, 340.0, "Top products", 16.0, Color::WHITE, true)];
    for (i, (name, sales)) in [
        ("Aurora Lamp", "842 sold"),
        ("Nebula Chair", "731 sold"),
        ("Halo Speaker", "655 sold"),
        ("Drift Desk", "512 sold"),
    ]
    .iter()
    .enumerate()
    {
        let y = 66.0 + i as f32 * 74.0;
        tchildren.push(ellipse(&format!("d-topprod-{i}-dot"), 24.0, y + 2.0, 34.0, 34.0, Color::rgba(255, 255, 255, 60)));
        tchildren.push(t(&format!("d-topprod-{i}-n"), 72.0, y, 290.0, name, 14.0, Color::WHITE, true));
        tchildren.push(t(&format!("d-topprod-{i}-s"), 72.0, y + 24.0, 290.0, sales, 12.0, G_FAINT, false));
    }
    top_prod.children = tchildren;
    root.push(top_prod);
    // Recent orders strip.
    let mut orders = glass("d-orders", "Recent orders", 298.0, 670.0, 1118.0, 206.0, 24.0, 24);
    orders.children = vec![
        t("d-orders-t", 26.0, 18.0, 500.0, "Recent orders", 16.0, Color::WHITE, true),
        t("d-orders-1", 26.0, 58.0, 1060.0, "#1042    Aurora Lamp        $129       Paid", 13.0, G_SUB, false),
        t("d-orders-2", 26.0, 96.0, 1060.0, "#1041    Nebula Chair       $349       Shipped", 13.0, G_SUB, false),
        t("d-orders-3", 26.0, 134.0, 1060.0, "#1040    Halo Speaker       $89        Pending", 13.0, G_SUB, false),
    ];
    root.push(orders);
    page.root.children = root;
    page
}

fn glass_shop() -> Page {
    let mut page = Page::new("shop", "Glass shop", "/shop", 1440.0, 900.0);
    page.background = Some(Color::rgb(28, 24, 58));
    let mut root = vec![
        rect("s-bg", "Backdrop", 0.0, 0.0, 1440.0, 900.0, Color::rgb(28, 24, 58), 0.0),
        ellipse("s-glow-1", -60.0, 200.0, 520.0, 420.0, Color::rgba(255, 90, 160, 84)),
        ellipse("s-glow-2", 980.0, -100.0, 560.0, 420.0, Color::rgba(124, 58, 237, 100)),
        ellipse("s-glow-3", 520.0, 620.0, 560.0, 340.0, Color::rgba(56, 150, 255, 76)),
        glass_nav("s"),
        t("s-title", 470.0, 116.0, 500.0, "Glass Store", 34.0, Color::WHITE, true),
        t("s-sub", 470.0, 168.0, 500.0, "Objects that float in light.", 14.0, G_SUB, false),
    ];
    let items: [(&str, &str, Color); 4] = [
        ("Aurora Lamp", "$129", Color::rgba(255, 190, 90, 150)),
        ("Nebula Chair", "$349", Color::rgba(140, 120, 255, 150)),
        ("Halo Speaker", "$89", Color::rgba(90, 220, 200, 150)),
        ("Drift Desk", "$499", Color::rgba(255, 130, 170, 150)),
    ];
    for (i, (name, price, tint)) in items.iter().enumerate() {
        let mut card = glass(
            &format!("s-card-{i}"),
            "Glass product card",
            40.0 + i as f32 * 343.0,
            220.0,
            310.0,
            400.0,
            26.0,
            30,
        );
        card.children = vec![
            glass(&format!("s-card-{i}-img"), "Product visual", 24.0, 24.0, 262.0, 170.0, 18.0, 46),
            ellipse(&format!("s-card-{i}-orb"), 99.0, 59.0, 112.0, 100.0, *tint),
            t(&format!("s-card-{i}-name"), 24.0, 218.0, 262.0, name, 16.0, Color::WHITE, true),
            t(&format!("s-card-{i}-price"), 24.0, 250.0, 262.0, price, 15.0, G_SUB, false),
            btn(&format!("s-card-{i}-buy"), 24.0, 322.0, 262.0, 44.0, "Add to cart", 14.0, Color::rgba(255, 255, 255, 235), Color::rgb(28, 24, 58), 22.0),
        ];
        root.push(card);
    }
    let mut banner = glass("s-banner", "Glass banner", 190.0, 664.0, 1060.0, 150.0, 28.0, 24);
    banner.children = vec![
        t("s-banner-t", 36.0, 30.0, 620.0, "Free shipping over $50", 20.0, Color::WHITE, true),
        t("s-banner-s", 36.0, 68.0, 620.0, "Members also get early access to new drops.", 13.0, G_SUB, false),
        btn("s-banner-b", 796.0, 51.0, 220.0, 48.0, "Join free", 14.0, Color::rgba(255, 255, 255, 235), Color::rgb(28, 24, 58), 24.0),
    ];
    root.push(banner);
    page.root.children = root;
    page
}

fn glass_mobile() -> Page {
    let mut page = Page::new("mobile", "Glass mobile", "/mobile", 390.0, 844.0);
    page.background = Some(Color::rgb(18, 16, 38));
    let mut root = vec![
        rect("m-bg", "Backdrop", 0.0, 0.0, 390.0, 844.0, Color::rgb(18, 16, 38), 0.0),
        ellipse("m-glow-1", -60.0, -40.0, 260.0, 220.0, Color::rgba(124, 58, 237, 130)),
        ellipse("m-glow-2", 210.0, 120.0, 240.0, 220.0, Color::rgba(56, 150, 255, 110)),
        ellipse("m-glow-3", -40.0, 560.0, 240.0, 220.0, Color::rgba(255, 90, 160, 80)),
        t("m-status", 28.0, 16.0, 120.0, "9:41", 13.0, Color::WHITE, true),
        t("m-hello", 24.0, 58.0, 342.0, "Good morning", 24.0, Color::WHITE, true),
        t("m-date", 24.0, 96.0, 342.0, "Your glass wallet at a glance", 13.0, G_SUB, false),
    ];
    let mut balance = glass("m-balance", "Balance card", 24.0, 140.0, 342.0, 150.0, 26.0, 40);
    balance.children = vec![
        t("m-bal-l", 24.0, 20.0, 290.0, "Balance", 12.0, G_FAINT, false),
        t("m-bal-v", 24.0, 46.0, 290.0, "$12,480.90", 30.0, Color::WHITE, true),
        t("m-bal-s", 24.0, 100.0, 290.0, "Main account  •• 4021", 12.0, G_SUB, false),
    ];
    root.push(balance);
    for (i, label) in ["Send", "Request", "Top up", "More"].iter().enumerate() {
        let x = 24.0 + i as f32 * 87.0;
        root.push(glass(&format!("m-act-{i}"), "Quick action", x, 318.0, 72.0, 72.0, 20.0, 36));
        root.push(t(&format!("m-act-{i}-t"), x, 398.0, 72.0, label, 11.0, G_SUB, false));
    }
    let mut tx = glass("m-tx", "Transactions", 24.0, 448.0, 342.0, 236.0, 26.0, 30);
    tx.children = vec![
        t("m-tx-t", 22.0, 16.0, 290.0, "Recent activity", 15.0, Color::WHITE, true),
        t("m-tx-1", 22.0, 56.0, 290.0, "Coffee House        -$6.50", 13.0, G_SUB, false),
        t("m-tx-2", 22.0, 96.0, 290.0, "Salary                +$3,200", 13.0, G_SUB, false),
        t("m-tx-3", 22.0, 136.0, 290.0, "Glass Store         -$129", 13.0, G_SUB, false),
        t("m-tx-4", 22.0, 176.0, 290.0, "Metro Card          -$22", 13.0, G_SUB, false),
    ];
    root.push(tx);
    let mut tab = glass("m-tab", "Glass tab bar", 24.0, 724.0, 342.0, 64.0, 32.0, 70);
    tab.children = vec![
        t("m-tab-1", 30.0, 22.0, 70.0, "Home", 11.0, Color::WHITE, true),
        t("m-tab-2", 108.0, 22.0, 70.0, "Shop", 11.0, G_SUB, false),
        t("m-tab-3", 186.0, 22.0, 70.0, "Cards", 11.0, G_SUB, false),
        t("m-tab-4", 264.0, 22.0, 60.0, "Me", 11.0, G_SUB, false),
    ];
    root.push(tab);
    page.root.children = root;
    page
}

/// The liquid glass family: landing, dashboard, shop and mobile pages.
pub fn liquid_glass() -> ProjectFile {
    let mut project = Project::new("liquid-glass", "Liquid Glass Family");
    project.description =
        "Apple liquid glass style template family: landing, dashboard, shop and mobile pages with translucent glass surfaces.".into();
    project.theme.primary = Color::rgb(124, 58, 237);
    project.pages = vec![
        glass_landing(),
        glass_dashboard(),
        glass_shop(),
        glass_mobile(),
    ];
    ProjectFile {
        project,
        database: Database::default(),
        flows: Vec::new(),
    }
}


// ---------------------------------------------------------------------
// Wave 5 templates: SaaS dashboard, clinic, restaurant, portfolio.
// Same rules as above: real projects, Persian RTL, original sample
// data, a database and at least one flow each.
// ---------------------------------------------------------------------

fn input_field(
    id: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    placeholder: &str,
    fill: Color,
    tc: Color,
) -> Node {
    let mut n = Node::new(id, "Input", NodeKind::Input, x, y, w, h);
    n.fill = Some(fill);
    n.corner_radius = 8.0;
    n.stroke = Some(Color::rgb(210, 214, 220));
    n.stroke_width = 1.0;
    n.text = Some(placeholder.into());
    n.font_size = 13.0;
    n.text_color = Some(tc);
    n
}

fn stat_chip(id: &str, x: f32, y: f32, w: f32, label: &str, value: &str, accent: Color) -> Node {
    let mut card = outlined(id, "Stat", x, y, w, 104.0, Color::WHITE, 14.0);
    card.children = vec![
        ellipse(&format!("{id}-dot"), w - 46.0, 20.0, 28.0, 28.0, accent),
        t(&format!("{id}-v"), 16.0, 18.0, w - 88.0, value, 22.0, M_INK, true),
        t(&format!("{id}-l"), 16.0, 58.0, w - 88.0, label, 13.0, M_MUTED, false),
    ];
    card
}

// ---------------------------- SaaS dashboard ------------------------

const S_BG: Color = Color::rgb(13, 17, 28);
const S_PANEL: Color = Color::rgb(22, 28, 43);
const S_PANEL2: Color = Color::rgb(28, 36, 54);
const S_LINE: Color = Color::rgb(42, 52, 74);
const S_TEXT: Color = Color::rgb(232, 236, 244);
const S_MUTED: Color = Color::rgb(139, 148, 169);
const S_ACCENT: Color = Color::rgb(99, 102, 241);
const S_TEAL: Color = Color::rgb(45, 212, 191);
const S_GREEN: Color = Color::rgb(52, 211, 153);
const S_AMBER: Color = Color::rgb(251, 191, 36);
const S_RED2: Color = Color::rgb(248, 113, 113);

fn saas_sidebar(prefix: &str, active: usize) -> Node {
    let mut bar = rect(prefix, "Sidebar", 0.0, 0.0, 250.0, 900.0, S_PANEL, 0.0);
    let items = [
        "داشبورد",
        "سفارش ها",
        "مشتریان",
        "محصولات",
        "گزارش ها",
        "امور مالی",
        "تنظیمات",
    ];
    let mut children = vec![
        t(&format!("{prefix}-brand"), 24.0, 22.0, 202.0, "بینا دیتا", 20.0, S_TEXT, true),
        t(&format!("{prefix}-brandsub"), 24.0, 56.0, 202.0, "پنل مدیریت فروش", 12.0, S_MUTED, false),
    ];
    for (i, label) in items.iter().enumerate() {
        let y = 110.0 + i as f32 * 52.0;
        if i == active {
            let mut pill = rect(&format!("{prefix}-pill"), "Active item", 16.0, y, 218.0, 40.0, S_ACCENT, 10.0);
            pill.children = vec![t(
                &format!("{prefix}-item-{i}"),
                16.0,
                9.0,
                186.0,
                label,
                14.0,
                Color::WHITE,
                true,
            )];
            children.push(pill);
        } else {
            children.push(t(&format!("{prefix}-item-{i}"), 40.0, y + 9.0, 186.0, label, 14.0, S_MUTED, false));
        }
    }
    children.push(t(
        &format!("{prefix}-plan"),
        24.0,
        800.0,
        202.0,
        "پلن حرفه ای\nتا ۳۰ روز دیگر فعال است",
        12.0,
        S_MUTED,
        false,
    ));
    bar.children = children;
    bar
}

fn saas_topbar(prefix: &str) -> Node {
    let mut top = rect(prefix, "Topbar", 250.0, 0.0, 1190.0, 64.0, S_PANEL, 0.0);
    let mut search = input_field(&format!("{prefix}-search"), 880.0, 12.0, 270.0, 40.0, "جستجو در پنل", S_PANEL2, S_MUTED);
    search.stroke = Some(S_LINE);
    top.children = vec![
        search,
        btn(&format!("{prefix}-export"), 24.0, 12.0, 150.0, 40.0, "خروجی گزارش", 13.0, S_ACCENT, Color::WHITE, 8.0),
        t(&format!("{prefix}-admin"), 200.0, 22.0, 180.0, "سارا محمدی", 13.0, S_TEXT, true),
        ellipse(&format!("{prefix}-avatar"), 396.0, 12.0, 40.0, 40.0, S_TEAL),
    ];
    top
}

fn kpi_card(id: &str, x: f32, label: &str, value: &str, delta: &str, delta_color: Color) -> Node {
    let mut card = rect(id, "KPI card", x, 148.0, 264.0, 118.0, S_PANEL, 14.0);
    card.stroke = Some(S_LINE);
    card.stroke_width = 1.0;
    card.children = vec![
        t(&format!("{id}-l"), 18.0, 16.0, 228.0, label, 13.0, S_MUTED, false),
        t(&format!("{id}-v"), 18.0, 44.0, 228.0, value, 21.0, S_TEXT, true),
        t(&format!("{id}-d"), 18.0, 84.0, 228.0, delta, 12.0, delta_color, true),
    ];
    card
}

fn saas_dashboard_page() -> Page {
    let mut page = Page::new("saas-home", "SaaS dashboard", "/", 1440.0, 900.0);
    page.background = Some(S_BG);
    let mut root = vec![saas_sidebar("saas-side", 0), saas_topbar("saas-top")];
    root.push(t("saas-title", 286.0, 84.0, 1118.0, "داشبورد فروش", 25.0, S_TEXT, true));
    root.push(t("saas-sub", 286.0, 122.0, 1118.0, "نمای کلی عملکرد فروشگاه در امروز", 13.0, S_MUTED, false));
    root.push(kpi_card("saas-kpi-0", 286.0, "درآمد امروز", &fa_price(84500000), "۱۲٪ بیشتر از دیروز", S_GREEN));
    root.push(kpi_card("saas-kpi-1", 574.0, "سفارش های جدید", &fa_num(324), "۸٪ بیشتر از دیروز", S_GREEN));
    root.push(kpi_card("saas-kpi-2", 862.0, "مشتریان جدید", &fa_num(87), "۳٪ کمتر از دیروز", S_RED2));
    root.push(kpi_card("saas-kpi-3", 1150.0, "نرخ تبدیل", "۴٫۸ درصد", "۰٫۶٪ بیشتر از دیروز", S_TEAL));

    // Weekly sales chart.
    let mut chart = rect("saas-chart", "Chart card", 286.0, 292.0, 700.0, 296.0, S_PANEL, 14.0);
    chart.stroke = Some(S_LINE);
    chart.stroke_width = 1.0;
    let heights = [90.0f32, 140.0, 115.0, 175.0, 150.0, 205.0, 180.0];
    let days = ["شنبه", "یکشنبه", "دوشنبه", "سه شنبه", "چهارشنبه", "پنجشنبه", "جمعه"];
    let mut cc = vec![
        t("saas-chart-t", 24.0, 18.0, 640.0, "روند فروش این هفته", 16.0, S_TEXT, true),
        rect("saas-chart-base", "Baseline", 24.0, 238.0, 652.0, 1.0, S_LINE, 0.0),
    ];
    for (i, (h, d)) in heights.iter().zip(days.iter()).enumerate() {
        let bx = 560.0 - i as f32 * 82.0;
        cc.push(rect(&format!("saas-bar-{i}"), "Bar", bx, 238.0 - h, 44.0, *h, S_ACCENT, 5.0));
        cc.push(t(&format!("saas-day-{i}"), bx - 14.0, 250.0, 72.0, d, 10.0, S_MUTED, false));
    }
    chart.children = cc;
    root.push(chart);

    // Top products list.
    let mut top = rect("saas-top-list", "Top products", 1010.0, 292.0, 394.0, 296.0, S_PANEL, 14.0);
    top.stroke = Some(S_LINE);
    top.stroke_width = 1.0;
    let tops = [
        ("هدفون بی سیم", fa_price(12400000)),
        ("ساعت هوشمند", fa_price(9800000)),
        ("کیبورد مکانیکی", fa_price(7600000)),
        ("ماوس بی سیم", fa_price(5400000)),
    ];
    let mut tc = vec![t("saas-top-t", 20.0, 18.0, 354.0, "پرفروش ترین محصولات", 16.0, S_TEXT, true)];
    for (i, (name, amount)) in tops.iter().enumerate() {
        let y = 62.0 + i as f32 * 56.0;
        tc.push(t(&format!("saas-top-{i}-n"), 20.0, y, 220.0, name, 13.0, S_TEXT, false));
        tc.push(t(&format!("saas-top-{i}-a"), 240.0, y, 134.0, amount, 12.0, S_TEAL, true));
        if i < 3 {
            tc.push(rect(&format!("saas-top-{i}-line"), "Divider", 20.0, y + 34.0, 354.0, 1.0, S_LINE, 0.0));
        }
    }
    top.children = tc;
    root.push(top);

    // Latest orders table.
    let mut table = rect("saas-table", "Orders table", 286.0, 614.0, 1118.0, 252.0, S_PANEL, 14.0);
    table.stroke = Some(S_LINE);
    table.stroke_width = 1.0;
    let rows = [
        ("۱۰۰۲۴", "سارا محمدی", fa_price(4850000), "پرداخت شده", S_GREEN),
        ("۱۰۰۲۳", "علی رضایی", fa_price(1890000), "در انتظار پرداخت", S_AMBER),
        ("۱۰۰۲۲", "مریم احمدی", fa_price(9270000), "پرداخت شده", S_GREEN),
        ("۱۰۰۲۱", "رضا کریمی", fa_price(640000), "لغو شده", S_RED2),
    ];
    let mut rc = vec![
        t("saas-table-t", 24.0, 16.0, 700.0, "آخرین سفارش ها", 16.0, S_TEXT, true),
        t("saas-table-h", 24.0, 54.0, 1070.0, "شماره سفارش                    مشتری                              مبلغ                         وضعیت", 12.0, S_MUTED, true),
        rect("saas-table-hl", "Divider", 24.0, 84.0, 1070.0, 1.0, S_LINE, 0.0),
    ];
    for (i, (num, customer, amount, status, sc)) in rows.iter().enumerate() {
        let y = 100.0 + i as f32 * 38.0;
        rc.push(t(&format!("saas-row-{i}-num"), 1010.0, y, 84.0, num, 13.0, S_MUTED, false));
        rc.push(t(&format!("saas-row-{i}-c"), 700.0, y, 280.0, customer, 13.0, S_TEXT, false));
        rc.push(t(&format!("saas-row-{i}-a"), 330.0, y, 300.0, amount, 13.0, S_TEXT, false));
        let mut chip = rect(&format!("saas-row-{i}-chip"), "Status chip", 24.0, y - 3.0, 130.0, 26.0, S_PANEL2, 13.0);
        chip.children = vec![t(&format!("saas-row-{i}-s"), 8.0, 4.0, 114.0, status, 11.0, *sc, true)];
        rc.push(chip);
    }
    table.children = rc;
    root.push(table);

    page.root.children = root;
    page
}

fn saas_customers_page() -> Page {
    let mut page = Page::new("saas-customers", "SaaS customers", "/customers", 1440.0, 900.0);
    page.background = Some(S_BG);
    let mut side = saas_sidebar("saasc-side", 2);
    side.height = 900.0;
    let mut root = vec![side, saas_topbar("saasc-top")];
    root.push(t("saasc-title", 286.0, 84.0, 1118.0, "مشتریان", 25.0, S_TEXT, true));
    root.push(t("saasc-sub", 286.0, 122.0, 1118.0, "فهرست مشتریان فعال فروشگاه", 13.0, S_MUTED, false));
    let mut table = rect("saasc-table", "Customers table", 286.0, 160.0, 1118.0, 330.0, S_PANEL, 14.0);
    table.stroke = Some(S_LINE);
    table.stroke_width = 1.0;
    let people = [
        ("سارا محمدی", "sara@example.ir", "۰۹۱۲۰۰۰۰۰۰۱", fa_price(12400000)),
        ("علی رضایی", "ali@example.ir", "۰۹۱۲۰۰۰۰۰۰۲", fa_price(8900000)),
        ("مریم احمدی", "maryam@example.ir", "۰۹۱۲۰۰۰۰۰۰۳", fa_price(15700000)),
        ("رضا کریمی", "reza@example.ir", "۰۹۱۲۰۰۰۰۰۰۴", fa_price(3200000)),
        ("زهرا حسینی", "zahra@example.ir", "۰۹۱۲۰۰۰۰۰۰۵", fa_price(6800000)),
    ];
    let mut rc = vec![t("saasc-table-h", 24.0, 20.0, 1070.0, "نام مشتری                         ایمیل                                     شماره تماس                    مجموع خرید", 12.0, S_MUTED, true)];
    for (i, (name, email, phone, total)) in people.iter().enumerate() {
        let y = 62.0 + i as f32 * 52.0;
        rc.push(rect(&format!("saasc-row-{i}-line"), "Divider", 24.0, y - 10.0, 1070.0, 1.0, S_LINE, 0.0));
        rc.push(t(&format!("saasc-row-{i}-n"), 810.0, y, 284.0, name, 14.0, S_TEXT, true));
        rc.push(t(&format!("saasc-row-{i}-e"), 470.0, y, 320.0, email, 13.0, S_MUTED, false));
        rc.push(t(&format!("saasc-row-{i}-p"), 250.0, y, 200.0, phone, 13.0, S_MUTED, false));
        rc.push(t(&format!("saasc-row-{i}-t"), 24.0, y, 200.0, total, 13.0, S_TEAL, true));
    }
    table.children = rc;
    root.push(table);
    root.push(stat_chip("saasc-stat", 286.0, 522.0, 340.0, "مجموع مشتریان فعال", &fa_num(1284), S_TEAL));
    page.root.children = root;
    page
}

pub fn saas_dashboard() -> ProjectFile {
    let mut project = Project::new("saas-dashboard", "SaaS Dashboard");
    project.description =
        "Dark SaaS admin dashboard template: sidebar, KPI cards, weekly sales chart, orders and customers tables.".into();
    project.theme.primary = S_ACCENT;
    project.pages = vec![saas_dashboard_page(), saas_customers_page()];

    let mut orders = Table::new("orders");
    orders.label = "Orders".into();
    orders = orders
        .column(Column::new("customer_id", ColumnType::Relation).required().relates_to("customers"))
        .column(Column::new("amount", ColumnType::Money).required())
        .column(Column::new("status", ColumnType::Text).required());
    for (cid, amount, status) in [
        (1, 4850000, "پرداخت شده"),
        (2, 1890000, "در انتظار پرداخت"),
        (3, 9270000, "پرداخت شده"),
        (4, 640000, "لغو شده"),
        (1, 2750000, "پرداخت شده"),
    ] {
        let mut m = Map::new();
        m.insert("customer_id".into(), json!(cid));
        m.insert("amount".into(), json!(amount));
        m.insert("status".into(), json!(status));
        orders.seed_rows.push(m);
    }
    let mut customers = Table::new("customers");
    customers.label = "Customers".into();
    customers = customers
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("email", ColumnType::Text).required())
        .column(Column::new("phone", ColumnType::Text));
    for (name, email, phone) in [
        ("سارا محمدی", "sara@example.ir", "09120000001"),
        ("علی رضایی", "ali@example.ir", "09120000002"),
        ("مریم احمدی", "maryam@example.ir", "09120000003"),
        ("رضا کریمی", "reza@example.ir", "09120000004"),
        ("زهرا حسینی", "zahra@example.ir", "09120000005"),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("email".into(), json!(email));
        m.insert("phone".into(), json!(phone));
        customers.seed_rows.push(m);
    }
    let mut products = Table::new("products");
    products.label = "Products".into();
    products = products
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("price", ColumnType::Money).required())
        .column(Column::new("stock", ColumnType::Integer).required());
    for (name, price, stock) in [
        ("هدفون بی سیم", 890000, 40),
        ("ساعت هوشمند", 640000, 25),
        ("کیبورد مکانیکی", 320000, 33),
        ("ماوس بی سیم", 185000, 60),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("price".into(), json!(price));
        m.insert("stock".into(), json!(stock));
        products.seed_rows.push(m);
    }
    let database = Database {
        tables: vec![orders, customers, products],
        relations: vec![Relation {
            name: "order_customer".into(),
            from_table: "orders".into(),
            from_column: "customer_id".into(),
            to_table: "customers".into(),
            to_column: "id".into(),
            kind: RelationKind::OneToMany,
        }],
    };
    let export = Flow {
        id: "export-report".into(),
        name: "Export orders report".into(),
        trigger: Event {
            kind: EventKind::Click,
            source_node: Some("saas-top-export".into()),
            page_route: Some("/".into()),
        },
        steps: vec![
            Step::Action {
                action: Action::CallApi { url: "/api/orders".into(), method: "GET".into() },
            },
            Step::Action {
                action: Action::ShowMessage { text: "گزارش سفارش ها آماده شد".into() },
            },
        ],
    };
    ProjectFile { project, database, flows: vec![export] }
}

// ------------------------------- Clinic -----------------------------

const C_TEAL: Color = Color::rgb(13, 148, 136);
const C_TEAL_DARK: Color = Color::rgb(15, 94, 88);
const C_TINT: Color = Color::rgb(224, 245, 242);
const C_INK: Color = Color::rgb(28, 38, 44);
const C_BG: Color = Color::rgb(246, 249, 248);

fn clinic_header(prefix: &str) -> Vec<Node> {
    vec![
        rect(&format!("{prefix}-bar"), "Header", 0.0, 0.0, 1440.0, 72.0, Color::WHITE, 0.0),
        t(&format!("{prefix}-brand"), 250.0, 20.0, 210.0, "کلینیک مهر", 22.0, C_TEAL_DARK, true),
        t(
            &format!("{prefix}-nav"),
            480.0,
            26.0,
            700.0,
            "خدمات            پزشکان            نوبت دهی            درباره ما            تماس",
            14.0,
            C_INK,
            false,
        ),
        btn(&format!("{prefix}-cta"), 60.0, 14.0, 170.0, 44.0, "رزرو نوبت", 14.0, C_TEAL, Color::WHITE, 10.0),
    ]
}

fn clinic_home() -> Page {
    let mut page = Page::new("clinic-home", "Clinic home", "/", 1440.0, 1352.0);
    page.background = Some(C_BG);
    let mut root = clinic_header("cl");

    let mut hero = rect("cl-hero", "Hero", 40.0, 96.0, 1360.0, 272.0, C_TINT, 18.0);
    hero.children = vec![
        ellipse("cl-hero-glow", 40.0, 40.0, 190.0, 190.0, Color::rgba(13, 148, 136, 40)),
        ellipse("cl-hero-glow2", 240.0, 150.0, 110.0, 110.0, Color::rgba(13, 148, 136, 26)),
        t("cl-hero-title", 620.0, 56.0, 700.0, "سلامتی شما، اولویت ما", 34.0, C_TEAL_DARK, true),
        t("cl-hero-sub", 620.0, 118.0, 700.0, "رزرو آنلاین نوبت از پزشکان متخصص، بدون انتظار و تماس تلفنی", 16.0, C_INK, false),
        btn("cl-hero-cta", 1070.0, 178.0, 250.0, 48.0, "رزرو آنلاین نوبت", 15.0, C_TEAL, Color::WHITE, 10.0),
    ];
    root.push(hero);

    root.push(t("cl-srv-title", 40.0, 400.0, 1360.0, "خدمات کلینیک", 21.0, C_INK, true));
    let services = [
        ("دندانپزشکی", "ایمپلنت، ارتودنسی و زیبایی دندان"),
        ("چشم پزشکی", "معاینه کامل و جراحی لیزیک"),
        ("قلب و عروق", "نوار قلب، اکو و تست ورزش"),
        ("پوست و مو", "درمان تخصصی پوست و کاشت مو"),
    ];
    for (i, (name, desc)) in services.iter().enumerate() {
        let mut card = outlined(&format!("cl-srv-{i}"), "Service card", 40.0 + i as f32 * 345.0, 444.0, 325.0, 150.0, Color::WHITE, 14.0);
        card.children = vec![
            ellipse(&format!("cl-srv-{i}-dot"), 237.0, 22.0, 56.0, 56.0, C_TINT),
            t(&format!("cl-srv-{i}-n"), 16.0, 96.0, 293.0, name, 16.0, C_INK, true),
            t(&format!("cl-srv-{i}-d"), 16.0, 124.0, 293.0, desc, 12.0, M_MUTED, false),
        ];
        root.push(card);
    }

    root.push(t("cl-doc-title", 40.0, 630.0, 1360.0, "پزشکان کلینیک", 21.0, C_INK, true));
    let doctors = [
        ("دکتر سارا محمدی", "متخصص دندانپزشکی", "اولین نوبت خالی: فردا ۱۰:۰۰"),
        ("دکتر علی رضایی", "متخصص چشم پزشکی", "اولین نوبت خالی: فردا ۱۶:۰۰"),
        ("دکتر مریم احمدی", "متخصص قلب و عروق", "اولین نوبت خالی: پس فردا ۰۹:۰۰"),
        ("دکتر رضا کریمی", "متخصص پوست و مو", "اولین نوبت خالی: امروز ۱۸:۰۰"),
    ];
    for (i, (name, spec, next)) in doctors.iter().enumerate() {
        let x = 40.0 + i as f32 * 345.0;
        let mut card = outlined(&format!("cl-doc-{i}"), "Doctor card", x, 674.0, 325.0, 252.0, Color::WHITE, 14.0);
        card.children = vec![
            ellipse(&format!("cl-doc-{i}-avatar"), 117.0, 22.0, 90.0, 90.0, C_TINT),
            t(&format!("cl-doc-{i}-n"), 16.0, 128.0, 293.0, name, 16.0, C_INK, true),
            t(&format!("cl-doc-{i}-s"), 16.0, 158.0, 293.0, spec, 13.0, C_TEAL_DARK, false),
            t(&format!("cl-doc-{i}-next"), 16.0, 186.0, 293.0, next, 12.0, M_MUTED, false),
            btn(&format!("cl-doc-{i}-btn"), 72.0, 214.0, 180.0, 0.0, "", 12.0, C_TEAL, Color::WHITE, 8.0),
        ];
        card.children[4].height = 30.0;
        card.children[4].text = Some("رزرو نوبت".into());
        card.children[4].font_size = 12.0;
        root.push(card);
    }

    let mut stats = rect("cl-stats", "Stats band", 40.0, 962.0, 1360.0, 120.0, C_TEAL_DARK, 16.0);
    stats.children = vec![
        t("cl-stats-1", 940.0, 28.0, 380.0, "۱۵ پزشک متخصص", 20.0, Color::WHITE, true),
        t("cl-stats-1s", 940.0, 66.0, 380.0, "در چهار بخش تخصصی", 13.0, C_TINT, false),
        t("cl-stats-2", 530.0, 28.0, 380.0, "امتیاز ۴٫۹ از مراجعان", 20.0, Color::WHITE, true),
        t("cl-stats-2s", 530.0, 66.0, 380.0, "بر اساس نظرسنجی داخلی کلینیک", 13.0, C_TINT, false),
        t("cl-stats-3", 120.0, 28.0, 380.0, "پذیرش همه روزه", 20.0, Color::WHITE, true),
        t("cl-stats-3s", 120.0, 66.0, 380.0, "از ۸ صبح تا ۱۰ شب", 13.0, C_TINT, false),
    ];
    root.push(stats);

    let mut footer = rect("cl-footer", "Footer", 0.0, 1122.0, 1440.0, 206.0, C_INK, 0.0);
    footer.children = vec![
        t("cl-footer-brand", 40.0, 26.0, 1360.0, "کلینیک مهر", 20.0, Color::WHITE, true),
        t("cl-footer-line", 40.0, 66.0, 1360.0, "نشانی نمونه: خیابان اصلی، پلاک ۱۲        تلفن نمونه: ۰۲۱-۰۰۰۰۰۰۰۰", 13.0, Color::rgb(170, 182, 188), false),
        t("cl-footer-col", 40.0, 110.0, 1360.0, "درباره ما        خدمات        پزشکان        سوالات متداول        حریم خصوصی", 13.0, Color::rgb(170, 182, 188), false),
    ];
    root.push(footer);

    page.root.children = root;
    page
}

fn clinic_appointment() -> Page {
    let mut page = Page::new("clinic-book", "Clinic appointment", "/appointment", 1440.0, 980.0);
    page.background = Some(C_BG);
    let mut root = clinic_header("clb");
    root.push(t("clb-title", 40.0, 104.0, 1360.0, "فرم رزرو نوبت", 26.0, C_INK, true));
    root.push(t("clb-sub", 40.0, 146.0, 1360.0, "اطلاعات خود را وارد کنید تا درخواست رزرو ثبت شود", 14.0, M_MUTED, false));

    let mut form = outlined("clb-form", "Booking form", 420.0, 196.0, 640.0, 640.0, Color::WHITE, 16.0);
    form.children = vec![
        t("clb-f-name-l", 32.0, 26.0, 576.0, "نام و نام خانوادگی", 13.0, C_INK, true),
        input_field("clb-f-name", 32.0, 54.0, 576.0, 46.0, "نام کامل شما", Color::WHITE, M_MUTED),
        t("clb-f-phone-l", 32.0, 122.0, 576.0, "شماره موبایل", 13.0, C_INK, true),
        input_field("clb-f-phone", 32.0, 150.0, 576.0, 46.0, "۰۹۱۲ ـــ ـــ ـــ", Color::WHITE, M_MUTED),
        t("clb-f-doc-l", 32.0, 218.0, 576.0, "پزشک", 13.0, C_INK, true),
        input_field("clb-f-doc", 32.0, 246.0, 576.0, 46.0, "دکتر سارا محمدی، دندانپزشکی", Color::WHITE, M_MUTED),
        t("clb-f-date-l", 330.0, 314.0, 278.0, "تاریخ", 13.0, C_INK, true),
        input_field("clb-f-date", 330.0, 342.0, 278.0, 46.0, "مثلا ۱۴۰۵/۰۸/۲۰", Color::WHITE, M_MUTED),
        t("clb-f-time-l", 32.0, 314.0, 278.0, "ساعت", 13.0, C_INK, true),
        input_field("clb-f-time", 32.0, 342.0, 278.0, 46.0, "مثلا ۱۰:۰۰", Color::WHITE, M_MUTED),
        t("clb-f-note-l", 32.0, 410.0, 576.0, "توضیحات (اختیاری)", 13.0, C_INK, true),
        input_field("clb-f-note", 32.0, 438.0, 576.0, 84.0, "علائم یا درخواست خاص خود را بنویسید", Color::WHITE, M_MUTED),
        btn("clinic-book-btn", 32.0, 548.0, 576.0, 52.0, "ثبت درخواست رزرو", 15.0, C_TEAL, Color::WHITE, 10.0),
    ];
    root.push(form);

    let mut side = outlined("clb-side", "How it works", 80.0, 196.0, 300.0, 330.0, Color::WHITE, 16.0);
    side.children = vec![
        t("clb-side-t", 20.0, 20.0, 260.0, "رزرو در سه قدم", 16.0, C_INK, true),
        t("clb-side-1", 20.0, 64.0, 260.0, "۱. پزشک و زمان را انتخاب کنید", 13.0, C_INK, false),
        t("clb-side-2", 20.0, 110.0, 260.0, "۲. فرم رزرو را ثبت کنید", 13.0, C_INK, false),
        t("clb-side-3", 20.0, 156.0, 260.0, "۳. پیامک تایید را دریافت کنید", 13.0, C_INK, false),
        t("clb-side-note", 20.0, 220.0, 260.0, "برای تغییر زمان، پشتیبانی\nکلینیک با شما تماس میگیرد.", 12.0, M_MUTED, false),
    ];
    root.push(side);

    page.root.children = root;
    page
}

pub fn clinic() -> ProjectFile {
    let mut project = Project::new("clinic", "Mehr Clinic");
    project.description =
        "Medical clinic template: services, doctors, online appointment booking form, doctors and appointments database.".into();
    project.theme.primary = C_TEAL;
    project.pages = vec![clinic_home(), clinic_appointment()];

    let mut doctors = Table::new("doctors");
    doctors.label = "Doctors".into();
    doctors = doctors
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("specialty", ColumnType::Text).required())
        .column(Column::new("next_available", ColumnType::Text));
    for (name, spec, next) in [
        ("دکتر سارا محمدی", "دندانپزشکی", "فردا ۱۰:۰۰"),
        ("دکتر علی رضایی", "چشم پزشکی", "فردا ۱۶:۰۰"),
        ("دکتر مریم احمدی", "قلب و عروق", "پس فردا ۰۹:۰۰"),
        ("دکتر رضا کریمی", "پوست و مو", "امروز ۱۸:۰۰"),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("specialty".into(), json!(spec));
        m.insert("next_available".into(), json!(next));
        doctors.seed_rows.push(m);
    }
    let mut appointments = Table::new("appointments");
    appointments.label = "Appointments".into();
    appointments = appointments
        .column(Column::new("patient_name", ColumnType::Text).required())
        .column(Column::new("phone", ColumnType::Text).required())
        .column(Column::new("doctor_id", ColumnType::Relation).required().relates_to("doctors"))
        .column(Column::new("date", ColumnType::Text).required())
        .column(Column::new("time", ColumnType::Text).required());
    for (patient, phone, doc, date, time) in [
        ("زهرا حسینی", "09120000011", 1, "۱۴۰۵/۰۸/۲۰", "۱۰:۰۰"),
        ("امیر صادقی", "09120000012", 3, "۱۴۰۵/۰۸/۲۱", "۰۹:۰۰"),
    ] {
        let mut m = Map::new();
        m.insert("patient_name".into(), json!(patient));
        m.insert("phone".into(), json!(phone));
        m.insert("doctor_id".into(), json!(doc));
        m.insert("date".into(), json!(date));
        m.insert("time".into(), json!(time));
        appointments.seed_rows.push(m);
    }
    let mut services = Table::new("services");
    services.label = "Services".into();
    services = services
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("price", ColumnType::Money));
    for (name, price) in [
        ("ویزیت دندانپزشکی", 450000),
        ("معاینه چشم پزشکی", 380000),
        ("نوار قلب", 290000),
        ("ویزیت پوست و مو", 420000),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("price".into(), json!(price));
        services.seed_rows.push(m);
    }
    let database = Database {
        tables: vec![doctors, appointments, services],
        relations: vec![Relation {
            name: "appointment_doctor".into(),
            from_table: "appointments".into(),
            from_column: "doctor_id".into(),
            to_table: "doctors".into(),
            to_column: "id".into(),
            kind: RelationKind::OneToMany,
        }],
    };
    let mut mapping = Map::new();
    mapping.insert("patient_name".into(), json!("$patient_name"));
    mapping.insert("phone".into(), json!("$phone"));
    mapping.insert("doctor_id".into(), json!("$doctor_id"));
    mapping.insert("date".into(), json!("$date"));
    mapping.insert("time".into(), json!("$time"));
    let booking = Flow {
        id: "book-appointment".into(),
        name: "Book appointment".into(),
        trigger: Event {
            kind: EventKind::Submit,
            source_node: Some("clinic-book-btn".into()),
            page_route: Some("/appointment".into()),
        },
        steps: vec![
            Step::Action {
                action: Action::SaveRecord { table: "appointments".into(), mapping },
            },
            Step::Action {
                action: Action::ShowMessage { text: "درخواست رزرو شما ثبت شد".into() },
            },
            Step::Action {
                action: Action::Navigate { route: "/".into() },
            },
        ],
    };
    ProjectFile { project, database, flows: vec![booking] }
}

// ----------------------------- Restaurant ---------------------------

const R_ACCENT: Color = Color::rgb(176, 84, 28);
const R_DARK: Color = Color::rgb(36, 28, 21);
const R_INK: Color = Color::rgb(43, 32, 24);
const R_BG: Color = Color::rgb(250, 246, 240);
const R_TINT: Color = Color::rgb(243, 230, 214);

fn restaurant_header(prefix: &str, dark: bool) -> Vec<Node> {
    let (bg, fg, sub) = if dark {
        (R_DARK, Color::WHITE, Color::rgb(214, 196, 175))
    } else {
        (Color::WHITE, R_INK, M_MUTED)
    };
    vec![
        rect(&format!("{prefix}-bar"), "Header", 0.0, 0.0, 1440.0, 72.0, bg, 0.0),
        t(&format!("{prefix}-brand"), 250.0, 20.0, 210.0, "رستوران طعم", 22.0, fg, true),
        t(
            &format!("{prefix}-nav"),
            480.0,
            26.0,
            700.0,
            "خانه            منو            رزرو میز            درباره ما            تماس",
            14.0,
            sub,
            false,
        ),
        btn(&format!("{prefix}-cta"), 60.0, 14.0, 170.0, 44.0, "رزرو میز", 14.0, R_ACCENT, Color::WHITE, 10.0),
    ]
}

struct Dish {
    name: &'static str,
    desc: &'static str,
    price: i64,
    category: &'static str,
    tint: Color,
}

fn dishes() -> Vec<Dish> {
    vec![
        Dish { name: "چلو کباب کوبیده", desc: "دو سیخ کوبیده گوسفندی با کره و گوجه کبابی", price: 485000, category: "غذای اصلی", tint: Color::rgb(243, 214, 190) },
        Dish { name: "زرشک پلو با مرغ", desc: "مرغ زعفرانی با زرشک تازه و خلال پسته", price: 395000, category: "غذای اصلی", tint: Color::rgb(250, 228, 200) },
        Dish { name: "قیمه نثار", desc: "برنج زعفرانی با گوشت و خلال بادام و پسته", price: 445000, category: "غذای اصلی", tint: Color::rgb(240, 220, 205) },
        Dish { name: "ماهی قزل آلا سرخ شده", desc: "ماهی تازه با سبزی پلو و لیموی تازه", price: 520000, category: "غذای اصلی", tint: Color::rgb(214, 234, 226) },
        Dish { name: "میرزا قاسمی", desc: "بادمجان دودی با سیر و گوجه تازه", price: 165000, category: "پیش غذا", tint: Color::rgb(226, 238, 214) },
        Dish { name: "کشک بادمجان", desc: "با کشک تازه، نعنا داغ و پیاز داغ", price: 155000, category: "پیش غذا", tint: Color::rgb(238, 226, 214) },
        Dish { name: "سوپ جو", desc: "سوپ جو با مرغ و جعفری تازه", price: 120000, category: "پیش غذا", tint: Color::rgb(250, 240, 205) },
        Dish { name: "سالاد شیرازی", desc: "خیار، گوجه و پیاز با آبغوره تازه", price: 95000, category: "پیش غذا", tint: Color::rgb(216, 240, 220) },
        Dish { name: "شله زرد", desc: "دسر سنتی با زعفران و گلاب", price: 85000, category: "دسر و نوشیدنی", tint: Color::rgb(252, 236, 180) },
        Dish { name: "فالوده شیرازی", desc: "با شربت لیموی تازه و گلاب", price: 90000, category: "دسر و نوشیدنی", tint: Color::rgb(226, 242, 250) },
        Dish { name: "دوغ محلی", desc: "دوغ سنتی با نعنای خشک", price: 45000, category: "دسر و نوشیدنی", tint: Color::rgb(238, 244, 226) },
        Dish { name: "چای زعفرانی", desc: "چای ایرانی با هل و زعفران", price: 40000, category: "دسر و نوشیدنی", tint: Color::rgb(250, 220, 190) },
    ]
}

fn dish_card(id: &str, x: f32, y: f32, d: &Dish) -> Node {
    let mut card = outlined(id, "Dish card", x, y, 325.0, 262.0, Color::WHITE, 14.0);
    card.children = vec![
        rect(&format!("{id}-img"), "Dish image", 16.0, 16.0, 293.0, 118.0, d.tint, 10.0),
        t(&format!("{id}-n"), 16.0, 148.0, 293.0, d.name, 16.0, R_INK, true),
        t(&format!("{id}-d"), 16.0, 180.0, 293.0, d.desc, 12.0, M_MUTED, false),
        t(&format!("{id}-p"), 16.0, 222.0, 293.0, &fa_price(d.price), 14.0, R_ACCENT, true),
    ];
    card
}

fn restaurant_home() -> Page {
    let all = dishes();
    let mut page = Page::new("rest-home", "Restaurant home", "/", 1440.0, 1560.0);
    page.background = Some(R_BG);
    let mut root = restaurant_header("rh", true);

    let mut hero = rect("rh-hero", "Hero", 40.0, 96.0, 1360.0, 300.0, R_DARK, 18.0);
    hero.children = vec![
        ellipse("rh-hero-glow", 60.0, 50.0, 200.0, 200.0, Color::rgba(255, 170, 80, 42)),
        ellipse("rh-hero-glow2", 300.0, 170.0, 120.0, 120.0, Color::rgba(255, 170, 80, 26)),
        t("rh-hero-title", 600.0, 66.0, 720.0, "طعم واقعی غذای ایرانی", 36.0, Color::WHITE, true),
        t("rh-hero-sub", 600.0, 132.0, 720.0, "دستورهای خانوادگی، مواد تازه روزانه و آتشی که از صبح روشن است", 16.0, Color::rgb(226, 208, 186), false),
        btn("rh-hero-menu", 1100.0, 200.0, 220.0, 48.0, "مشاهده منو", 15.0, R_ACCENT, Color::WHITE, 10.0),
        btn("rh-hero-book", 860.0, 200.0, 220.0, 48.0, "رزرو میز", 15.0, Color::WHITE, R_DARK, 10.0),
    ];
    root.push(hero);

    root.push(stat_chip("rh-stat-1", 40.0, 428.0, 325.0, "سال تجربه آشپزی سنتی", &fa_num(25), R_TINT));
    root.push(stat_chip("rh-stat-2", 385.0, 428.0, 325.0, "امتیاز مهمانان", "۴٫۸ از ۵", R_TINT));
    root.push(stat_chip("rh-stat-3", 730.0, 428.0, 325.0, "غذای متنوع در منو", &fa_num(120), R_TINT));
    root.push(stat_chip("rh-stat-4", 1075.0, 428.0, 325.0, "ظرفیت سالن اصلی", &format!("{} نفر", fa_num(80)), R_TINT));

    root.push(t("rh-feat-title", 40.0, 584.0, 1360.0, "غذاهای ویژه امروز", 21.0, R_INK, true));
    for (i, d) in [0usize, 1, 3].iter().map(|&i| &all[i]).enumerate() {
        root.push(dish_card(&format!("rh-dish-{i}"), 40.0 + i as f32 * 345.0, 628.0, d));
    }
    let mut banner = rect("rh-banner", "Chef banner", 1105.0, 628.0, 295.0, 262.0, R_ACCENT, 14.0);
    banner.children = vec![
        t("rh-banner-t", 20.0, 30.0, 255.0, "پیشنهاد سرآشپز", 18.0, Color::WHITE, true),
        t("rh-banner-s", 20.0, 70.0, 255.0, "هر روز یک غذای ویژه با دستور فصل و مواد تازه بازار", 13.0, Color::rgb(252, 228, 205), false),
        btn("rh-banner-btn", 20.0, 190.0, 180.0, 40.0, "دیدن منوی کامل", 13.0, Color::WHITE, R_ACCENT, 8.0),
    ];
    root.push(banner);

    // Reservation form band.
    root.push(t("rh-book-title", 40.0, 930.0, 1360.0, "رزرو میز", 21.0, R_INK, true));
    let mut form = outlined("rh-book-form", "Reservation form", 40.0, 974.0, 1360.0, 210.0, Color::WHITE, 16.0);
    form.children = vec![
        t("rh-book-name-l", 1060.0, 24.0, 268.0, "نام شما", 13.0, R_INK, true),
        input_field("rh-book-name", 1060.0, 52.0, 268.0, 46.0, "نام کامل", Color::WHITE, M_MUTED),
        t("rh-book-phone-l", 740.0, 24.0, 268.0, "شماره موبایل", 13.0, R_INK, true),
        input_field("rh-book-phone", 740.0, 52.0, 268.0, 46.0, "۰۹۱۲ ـــ ـــ ـــ", Color::WHITE, M_MUTED),
        t("rh-book-guests-l", 420.0, 24.0, 268.0, "تعداد مهمان", 13.0, R_INK, true),
        input_field("rh-book-guests", 420.0, 52.0, 268.0, 46.0, "مثلا ۴ نفر", Color::WHITE, M_MUTED),
        t("rh-book-date-l", 100.0, 24.0, 268.0, "تاریخ", 13.0, R_INK, true),
        input_field("rh-book-date", 100.0, 52.0, 268.0, 46.0, "مثلا ۱۴۰۵/۰۸/۲۰", Color::WHITE, M_MUTED),
        btn("rest-book-btn", 32.0, 128.0, 1296.0, 52.0, "ثبت رزرو میز", 15.0, R_ACCENT, Color::WHITE, 10.0),
    ];
    root.push(form);

    let mut footer = rect("rh-footer", "Footer", 0.0, 1240.0, 1440.0, 280.0, R_DARK, 0.0);
    footer.children = vec![
        t("rh-footer-brand", 40.0, 28.0, 1360.0, "رستوران طعم", 20.0, Color::WHITE, true),
        t("rh-footer-hours", 40.0, 72.0, 1360.0, "ساعت کاری نمونه: همه روزه ۱۱ صبح تا ۱۱ شب        نشانی نمونه: خیابان اصلی، پلاک ۲۴", 13.0, Color::rgb(200, 182, 160), false),
        t("rh-footer-col", 40.0, 122.0, 1360.0, "منو        رزرو میز        درباره ما        فرصت های شغلی        حریم خصوصی", 13.0, Color::rgb(200, 182, 160), false),
    ];
    root.push(footer);

    page.root.children = root;
    page
}

fn restaurant_menu() -> Page {
    let all = dishes();
    let mut page = Page::new("rest-menu", "Restaurant menu", "/menu", 1440.0, 1400.0);
    page.background = Some(R_BG);
    let mut root = restaurant_header("rm", false);
    root.push(t("rm-title", 40.0, 104.0, 1360.0, "منوی رستوران طعم", 27.0, R_INK, true));
    root.push(t("rm-sub", 40.0, 148.0, 1360.0, "همه قیمت ها به تومان است و روزانه با مواد تازه آماده میشوند", 14.0, M_MUTED, false));

    let cats = ["پیش غذا", "غذای اصلی", "دسر و نوشیدنی"];
    let mut y = 210.0f32;
    for cat in cats {
        root.push(t(&format!("rm-cat-{cat}"), 40.0, y, 1360.0, cat, 20.0, R_ACCENT, true));
        y += 46.0;
        for d in all.iter().filter(|d| d.category == cat) {
            root.push(t(&format!("rm-item-{}-n", d.name), 500.0, y, 900.0, d.name, 15.0, R_INK, true));
            root.push(t(&format!("rm-item-{}-d", d.name), 500.0, y + 26.0, 900.0, d.desc, 12.0, M_MUTED, false));
            root.push(rect(&format!("rm-item-{}-dots", d.name), "Leader dots", 260.0, y + 14.0, 820.0, 1.0, Color::rgb(214, 198, 178), 0.0));
            root.push(t(&format!("rm-item-{}-p", d.name), 40.0, y, 200.0, &fa_price(d.price), 14.0, R_INK, true));
            y += 74.0;
        }
        y += 26.0;
    }
    page.root.children = root;
    page
}

pub fn restaurant() -> ProjectFile {
    let mut project = Project::new("restaurant", "Taam Restaurant");
    project.description =
        "Restaurant template: hero, featured dishes, reservation form and a full Persian menu page backed by a menu items database.".into();
    project.theme.primary = R_ACCENT;
    project.pages = vec![restaurant_home(), restaurant_menu()];

    let mut items = Table::new("menu_items");
    items.label = "Menu items".into();
    items = items
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("category", ColumnType::Text).required())
        .column(Column::new("price", ColumnType::Money).required())
        .column(Column::new("description", ColumnType::Text));
    for d in dishes() {
        let mut m = Map::new();
        m.insert("name".into(), json!(d.name));
        m.insert("category".into(), json!(d.category));
        m.insert("price".into(), json!(d.price));
        m.insert("description".into(), json!(d.desc));
        items.seed_rows.push(m);
    }
    let mut reservations = Table::new("reservations");
    reservations.label = "Reservations".into();
    reservations = reservations
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("phone", ColumnType::Text).required())
        .column(Column::new("guests", ColumnType::Integer).required())
        .column(Column::new("date", ColumnType::Text).required());
    let mut m = Map::new();
    m.insert("name".into(), json!("زهرا حسینی"));
    m.insert("phone".into(), json!("09120000021"));
    m.insert("guests".into(), json!(4));
    m.insert("date".into(), json!("۱۴۰۵/۰۸/۲۰"));
    reservations.seed_rows.push(m);
    let database = Database { tables: vec![items, reservations], relations: Vec::new() };

    let mut mapping = Map::new();
    mapping.insert("name".into(), json!("$name"));
    mapping.insert("phone".into(), json!("$phone"));
    mapping.insert("guests".into(), json!("$guests"));
    mapping.insert("date".into(), json!("$date"));
    let book = Flow {
        id: "reserve-table".into(),
        name: "Reserve a table".into(),
        trigger: Event {
            kind: EventKind::Submit,
            source_node: Some("rest-book-btn".into()),
            page_route: Some("/".into()),
        },
        steps: vec![
            Step::Action {
                action: Action::SaveRecord { table: "reservations".into(), mapping },
            },
            Step::Action {
                action: Action::ShowMessage { text: "میز شما با موفقیت رزرو شد".into() },
            },
        ],
    };
    ProjectFile { project, database, flows: vec![book] }
}

// ----------------------------- Portfolio ----------------------------

const P_INK: Color = Color::rgb(24, 24, 27);
const P_MUTED: Color = Color::rgb(110, 110, 118);
const P_BG: Color = Color::rgb(250, 249, 246);
const P_COPPER: Color = Color::rgb(184, 110, 67);
const P_LINE: Color = Color::rgb(226, 222, 214);

fn portfolio_header(prefix: &str) -> Vec<Node> {
    vec![
        t(&format!("{prefix}-brand"), 40.0, 24.0, 300.0, "نام شما", 20.0, P_INK, true),
        t(
            &format!("{prefix}-nav"),
            480.0,
            28.0,
            700.0,
            "نمونه کارها            خدمات            درباره من            تماس",
            14.0,
            P_MUTED,
            false,
        ),
        btn(&format!("{prefix}-cta"), 60.0, 16.0, 170.0, 44.0, "شروع پروژه", 14.0, P_INK, Color::WHITE, 22.0),
    ]
}

fn work_card(id: &str, x: f32, y: f32, title: &str, tag: &str, tint: Color) -> Node {
    let mut card = Node::new(id, "Work card", NodeKind::Card, x, y, 426.0, 300.0);
    card.fill = Some(Color::WHITE);
    card.corner_radius = 6.0;
    card.stroke = Some(P_LINE);
    card.stroke_width = 1.0;
    card.children = vec![
        rect(&format!("{id}-img"), "Work image", 0.0, 0.0, 426.0, 200.0, tint, 6.0),
        t(&format!("{id}-t"), 20.0, 220.0, 386.0, title, 17.0, P_INK, true),
        t(&format!("{id}-tag"), 20.0, 254.0, 386.0, tag, 13.0, P_COPPER, false),
    ];
    card
}

fn portfolio_home() -> Page {
    let mut page = Page::new("pf-home", "Portfolio home", "/", 1440.0, 1740.0);
    page.background = Some(P_BG);
    let mut root = portfolio_header("pf");

    root.push(ellipse("pf-avatar", 90.0, 150.0, 300.0, 300.0, Color::rgb(238, 226, 211)));
    root.push(t("pf-avatar-t", 150.0, 276.0, 180.0, "عکس شما", 15.0, P_MUTED, false));
    root.push(t("pf-hero-kicker", 480.0, 168.0, 860.0, "پورتفولیوی شخصی", 14.0, P_COPPER, true));
    root.push(t("pf-hero-title", 480.0, 204.0, 860.0, "طراح و توسعه دهنده وب", 46.0, P_INK, true));
    root.push(t(
        "pf-hero-sub",
        480.0,
        286.0,
        860.0,
        "من وبسایت و محصول دیجیتال میسازم؛ ساده، سریع و با تمرکز بر تجربه واقعی کاربر. این متن نمونه است و با معرفی خودتان جایگزین میشود.",
        16.0,
        P_MUTED,
        false,
    ));
    root.push(btn("pf-hero-works", 1110.0, 400.0, 230.0, 50.0, "دیدن نمونه کارها", 15.0, P_INK, Color::WHITE, 25.0));
    root.push(btn("pf-hero-contact", 850.0, 400.0, 230.0, 50.0, "تماس با من", 15.0, Color::WHITE, P_INK, 25.0));

    root.push(rect("pf-works-line", "Divider", 40.0, 540.0, 1360.0, 1.0, P_LINE, 0.0));
    root.push(t("pf-works-title", 40.0, 566.0, 1360.0, "نمونه کارهای منتخب", 26.0, P_INK, true));
    let works = [
        ("وبسایت فروشگاهی سپیدار", "طراحی فروشگاه اینترنتی", Color::rgb(216, 236, 220)),
        ("داشبورد مدیریت مالی", "محصول تحت وب", Color::rgb(220, 228, 246)),
        ("اپلیکیشن رزرو کلینیک", "طراحی محصول", Color::rgb(222, 242, 238)),
        ("سایت شرکتی معماری", "وبسایت شرکتی", Color::rgb(238, 228, 214)),
        ("فروشگاه پوشاک", "تجارت الکترونیک", Color::rgb(246, 226, 232)),
        ("مجله آنلاین فرهنگ", "وبسایت محتوایی", Color::rgb(232, 230, 246)),
    ];
    for (i, (title, tag, tint)) in works.iter().enumerate() {
        let x = 40.0 + (i % 3) as f32 * 467.0;
        let y = 622.0 + (i / 3) as f32 * 330.0;
        root.push(work_card(&format!("pf-work-{i}"), x, y, title, tag, *tint));
    }

    root.push(t("pf-srv-title", 40.0, 1320.0, 1360.0, "خدمات", 26.0, P_INK, true));
    let services = [
        ("۰۱", "طراحی وبسایت", "وبسایت شرکتی و فروشگاهی سریع و راستچین، از ایده تا انتشار."),
        ("۰۲", "طراحی محصول", "رابط و تجربه کاربری برای اپلیکیشن و داشبوردهای تحت وب."),
        ("۰۳", "توسعه فرانت اند", "پیاده سازی دقیق و سبک با تمرکز بر کارایی و دسترس پذیری."),
    ];
    for (i, (num, name, desc)) in services.iter().enumerate() {
        let x = 40.0 + i as f32 * 467.0;
        root.push(t(&format!("pf-srv-{i}-num"), x, 1372.0, 426.0, num, 20.0, P_COPPER, true));
        root.push(t(&format!("pf-srv-{i}-n"), x, 1410.0, 426.0, name, 18.0, P_INK, true));
        root.push(t(&format!("pf-srv-{i}-d"), x, 1446.0, 426.0, desc, 13.0, P_MUTED, false));
    }

    let mut cta = rect("pf-ctaband", "CTA band", 40.0, 1540.0, 1360.0, 140.0, P_INK, 10.0);
    cta.children = vec![
        t("pf-cta-t", 420.0, 34.0, 900.0, "برای پروژه بعدی شما آماده ام", 24.0, Color::WHITE, true),
        t("pf-cta-s", 420.0, 78.0, 900.0, "از ایده تا محصول نهایی، قدم به قدم کنار شما", 14.0, Color::rgb(196, 196, 204), false),
        btn("pf-cta-btn", 60.0, 45.0, 220.0, 50.0, "شروع گفتگو", 15.0, P_COPPER, Color::WHITE, 25.0),
    ];
    root.push(cta);

    page.root.children = root;
    page
}

fn portfolio_contact() -> Page {
    let mut page = Page::new("pf-contact", "Portfolio contact", "/contact", 1440.0, 920.0);
    page.background = Some(P_BG);
    let mut root = portfolio_header("pc");
    root.push(t("pc-title", 40.0, 120.0, 1360.0, "تماس با من", 32.0, P_INK, true));
    root.push(t("pc-sub", 40.0, 172.0, 1360.0, "برای شروع یک پروژه یا یک گفتگوی کوتاه، فرم زیر را پر کنید", 15.0, P_MUTED, false));

    let mut form = Node::new("pc-form", "Contact form", NodeKind::Card, 440.0, 230.0, 620.0, 600.0);
    form.fill = Some(Color::WHITE);
    form.corner_radius = 8.0;
    form.stroke = Some(P_LINE);
    form.stroke_width = 1.0;
    form.children = vec![
        t("pc-f-name-l", 30.0, 26.0, 560.0, "نام شما", 13.0, P_INK, true),
        input_field("pc-f-name", 30.0, 54.0, 560.0, 46.0, "نام کامل", Color::WHITE, P_MUTED),
        t("pc-f-email-l", 30.0, 120.0, 560.0, "ایمیل", 13.0, P_INK, true),
        input_field("pc-f-email", 30.0, 148.0, 560.0, 46.0, "you@example.com", Color::WHITE, P_MUTED),
        t("pc-f-subject-l", 30.0, 214.0, 560.0, "موضوع پیام", 13.0, P_INK, true),
        input_field("pc-f-subject", 30.0, 242.0, 560.0, 46.0, "مثلا طراحی وبسایت فروشگاهی", Color::WHITE, P_MUTED),
        t("pc-f-msg-l", 30.0, 308.0, 560.0, "متن پیام", 13.0, P_INK, true),
        input_field("pc-f-msg", 30.0, 336.0, 560.0, 110.0, "درباره پروژه تان بنویسید", Color::WHITE, P_MUTED),
        btn("pf-send-btn", 30.0, 486.0, 560.0, 52.0, "ارسال پیام", 15.0, P_INK, Color::WHITE, 26.0),
    ];
    root.push(form);

    let mut info = Node::new("pc-info", "Contact info", NodeKind::Card, 80.0, 230.0, 300.0, 300.0);
    info.fill = Some(Color::WHITE);
    info.corner_radius = 8.0;
    info.stroke = Some(P_LINE);
    info.stroke_width = 1.0;
    info.children = vec![
        t("pc-info-t", 20.0, 20.0, 260.0, "راه های تماس", 16.0, P_INK, true),
        t("pc-info-email", 20.0, 62.0, 260.0, "ایمیل نمونه:\nhello@example.com", 13.0, P_MUTED, false),
        t("pc-info-city", 20.0, 130.0, 260.0, "شهر نمونه: تهران، ایران", 13.0, P_MUTED, false),
        t("pc-info-time", 20.0, 176.0, 260.0, "زمان پاسخگویی نمونه:\nکمتر از یک روز کاری", 13.0, P_MUTED, false),
    ];
    root.push(info);

    page.root.children = root;
    page
}

pub fn portfolio() -> ProjectFile {
    let mut project = Project::new("portfolio", "Personal Portfolio");
    project.description =
        "Personal portfolio template in a light editorial style: hero, selected works grid, services and a contact page.".into();
    project.theme.primary = P_COPPER;
    project.pages = vec![portfolio_home(), portfolio_contact()];

    let mut works = Table::new("works");
    works.label = "Works".into();
    works = works
        .column(Column::new("title", ColumnType::Text).required())
        .column(Column::new("category", ColumnType::Text).required())
        .column(Column::new("year", ColumnType::Integer));
    for (title, cat, year) in [
        ("وبسایت فروشگاهی سپیدار", "فروشگاه اینترنتی", 1404),
        ("داشبورد مدیریت مالی", "محصول تحت وب", 1404),
        ("اپلیکیشن رزرو کلینیک", "طراحی محصول", 1403),
        ("سایت شرکتی معماری", "وبسایت شرکتی", 1403),
        ("فروشگاه پوشاک", "تجارت الکترونیک", 1402),
        ("مجله آنلاین فرهنگ", "وبسایت محتوایی", 1402),
    ] {
        let mut m = Map::new();
        m.insert("title".into(), json!(title));
        m.insert("category".into(), json!(cat));
        m.insert("year".into(), json!(year));
        works.seed_rows.push(m);
    }
    let mut messages = Table::new("messages");
    messages.label = "Messages".into();
    messages = messages
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("email", ColumnType::Text).required())
        .column(Column::new("subject", ColumnType::Text))
        .column(Column::new("message", ColumnType::Text).required());
    let mut services = Table::new("services");
    services.label = "Services".into();
    services = services
        .column(Column::new("name", ColumnType::Text).required())
        .column(Column::new("description", ColumnType::Text));
    for (name, desc) in [
        ("طراحی وبسایت", "وبسایت شرکتی و فروشگاهی"),
        ("طراحی محصول", "رابط و تجربه کاربری"),
        ("توسعه فرانت اند", "پیاده سازی دقیق و سبک"),
    ] {
        let mut m = Map::new();
        m.insert("name".into(), json!(name));
        m.insert("description".into(), json!(desc));
        services.seed_rows.push(m);
    }
    let database = Database { tables: vec![works, messages, services], relations: Vec::new() };

    let mut mapping = Map::new();
    mapping.insert("name".into(), json!("$name"));
    mapping.insert("email".into(), json!("$email"));
    mapping.insert("subject".into(), json!("$subject"));
    mapping.insert("message".into(), json!("$message"));
    let send = Flow {
        id: "send-message".into(),
        name: "Send contact message".into(),
        trigger: Event {
            kind: EventKind::Submit,
            source_node: Some("pf-send-btn".into()),
            page_route: Some("/contact".into()),
        },
        steps: vec![
            Step::Action {
                action: Action::SaveRecord { table: "messages".into(), mapping },
            },
            Step::Action {
                action: Action::ShowMessage { text: "پیام شما ارسال شد، به زودی پاسخ میدهم".into() },
            },
        ],
    };
    ProjectFile { project, database, flows: vec![send] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_texts(p: &ProjectFile) -> Vec<String> {
        fn walk(n: &Node, out: &mut Vec<String>) {
            if let Some(tx) = &n.text {
                out.push(tx.clone());
            }
            for c in &n.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        for page in &p.project.pages {
            walk(&page.root, &mut out);
        }
        out
    }

    #[test]
    fn digikala_is_valid_and_rich() {
        let d = digikala_marketplace();
        assert!(d.validate().is_empty(), "{:?}", d.validate());
        assert_eq!(d.project.pages.len(), 4);
        let routes: Vec<&str> = d.project.pages.iter().map(|p| p.route.as_str()).collect();
        assert_eq!(routes, ["/", "/category", "/product", "/cart"]);
        assert_eq!(d.database.tables.len(), 4);
        let products = d.database.tables.iter().find(|t| t.name == "products").unwrap();
        assert!(products.seed_rows.len() >= 9);
        assert_eq!(d.flows.len(), 2);
        assert!(d.project.total_nodes() >= 150, "nodes: {}", d.project.total_nodes());
    }

    #[test]
    fn digikala_home_has_persian_digit_prices() {
        let d = digikala_marketplace();
        let texts = all_texts(&d);
        assert!(texts.iter().any(|s| s.contains('۴') && s.contains("تومان")));
        assert!(texts.iter().any(|s| s.contains("بازارینو")));
        assert!(texts.iter().any(|s| s.contains("شگفت‌انگیز")));
    }

    #[test]
    fn digikala_references_existing_image_assets() {
        let d = digikala_marketplace();
        let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/assets/digikala");
        let mut referenced: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        fn walk(n: &Node, out: &mut std::collections::BTreeSet<String>) {
            if let Some(src) = n.image_src() {
                out.insert(src.to_string());
            }
            for c in &n.children {
                walk(c, out);
            }
        }
        for page in &d.project.pages {
            walk(&page.root, &mut referenced);
        }
        assert!(referenced.len() >= 20, "photo references: {}", referenced.len());
        for src in &referenced {
            let rel = src.strip_prefix("assets/digikala/").expect("asset path shape");
            let file = assets_dir.join(rel);
            assert!(file.exists(), "missing asset file for {src}");
            assert!(std::fs::metadata(&file).unwrap().len() > 5_000, "asset {rel} is suspiciously small");
        }
        // The embedded copies the CLI ships must cover every reference.
        let embedded: std::collections::BTreeSet<&str> =
            digikala_assets().iter().map(|(name, _)| *name).collect();
        for src in &referenced {
            let rel = src.strip_prefix("assets/digikala/").unwrap();
            assert!(embedded.contains(rel), "{rel} is not embedded for `chida new`");
        }
    }

    #[test]
    fn liquid_glass_is_valid() {
        let g = liquid_glass();
        assert!(g.validate().is_empty(), "{:?}", g.validate());
        assert_eq!(g.project.pages.len(), 4);
        let mobile = g.project.pages.iter().find(|p| p.route == "/mobile").unwrap();
        assert_eq!(mobile.width, 390.0);
        let texts = all_texts(&g);
        assert!(texts.iter().any(|s| s.contains("Glass Store")));
    }

    #[test]
    fn saas_dashboard_is_valid() {
        let s = saas_dashboard();
        assert!(s.validate().is_empty(), "{:?}", s.validate());
        assert_eq!(s.project.pages.len(), 2);
        assert_eq!(s.database.tables.len(), 3);
        let orders = s.database.tables.iter().find(|t| t.name == "orders").unwrap();
        assert_eq!(orders.seed_rows.len(), 5);
        assert_eq!(s.flows.len(), 1);
        assert!(s.project.total_nodes() >= 60, "nodes: {}", s.project.total_nodes());
        let texts = all_texts(&s);
        assert!(texts.iter().any(|x| x.contains("داشبورد فروش")));
        assert!(texts.iter().any(|x| x.contains("تومان")));
    }

    #[test]
    fn clinic_is_valid_with_booking_flow() {
        let c = clinic();
        assert!(c.validate().is_empty(), "{:?}", c.validate());
        let routes: Vec<&str> = c.project.pages.iter().map(|p| p.route.as_str()).collect();
        assert_eq!(routes, ["/", "/appointment"]);
        let doctors = c.database.tables.iter().find(|t| t.name == "doctors").unwrap();
        assert_eq!(doctors.seed_rows.len(), 4);
        assert_eq!(c.flows.len(), 1);
        assert_eq!(c.flows[0].trigger.source_node.as_deref(), Some("clinic-book-btn"));
        let texts = all_texts(&c);
        assert!(texts.iter().any(|x| x.contains("کلینیک مهر")));
        assert!(texts.iter().any(|x| x.contains("دندانپزشکی")));
    }

    #[test]
    fn restaurant_is_valid_with_full_menu() {
        let r = restaurant();
        assert!(r.validate().is_empty(), "{:?}", r.validate());
        let routes: Vec<&str> = r.project.pages.iter().map(|p| p.route.as_str()).collect();
        assert_eq!(routes, ["/", "/menu"]);
        let items = r.database.tables.iter().find(|t| t.name == "menu_items").unwrap();
        assert_eq!(items.seed_rows.len(), 12);
        assert_eq!(r.flows.len(), 1);
        let texts = all_texts(&r);
        assert!(texts.iter().any(|x| x.contains("چلو کباب کوبیده")));
        assert!(texts.iter().any(|x| x.contains("تومان")));
    }

    #[test]
    fn portfolio_is_valid_with_contact_flow() {
        let p = portfolio();
        assert!(p.validate().is_empty(), "{:?}", p.validate());
        let routes: Vec<&str> = p.project.pages.iter().map(|pp| pp.route.as_str()).collect();
        assert_eq!(routes, ["/", "/contact"]);
        let works = p.database.tables.iter().find(|t| t.name == "works").unwrap();
        assert_eq!(works.seed_rows.len(), 6);
        assert_eq!(p.flows.len(), 1);
        assert_eq!(p.flows[0].trigger.source_node.as_deref(), Some("pf-send-btn"));
        let texts = all_texts(&p);
        assert!(texts.iter().any(|x| x.contains("نمونه کارهای منتخب")));
    }

    #[test]
    fn fa_num_formats_persian_digits() {
        assert_eq!(fa_num(480000), "۴۸۰٬۰۰۰");
        assert_eq!(fa_num(50), "۵۰");
        assert_eq!(fa_price(1850000), "۱٬۸۵۰٬۰۰۰ تومان");
        assert_eq!(fa_rating(4.4), "۴٫۴");
    }
}
