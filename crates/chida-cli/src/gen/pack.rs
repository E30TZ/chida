//! Data schema for generated templates: one JSON pack per template.
//! See `~/workspace/chida-templates/PACK-SCHEMA.md` for the author guide.
//! Every field has a serde default so packs stay terse; the engine and the
//! `tools/check-pack.py` validator enforce the real requirements.

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct Pack {
    pub id: String,
    pub category: String,
    pub name_fa: String,
    pub name_en: String,
    pub brand: String,
    pub tagline: String,
    pub description: String,
    pub palette: String,
    #[serde(default)]
    pub dark: bool,
    #[serde(default)]
    pub nav_cta: Option<NavCta>,
    pub pages: Vec<PackPage>,
    #[serde(default)]
    pub tables: Vec<PackTable>,
    #[serde(default)]
    pub flow: Option<PackFlow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NavCta {
    pub label: String,
    pub route: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackPage {
    pub id: String,
    pub name: String,
    pub route: String,
    #[serde(default)]
    pub chrome: Option<bool>,
    #[serde(default)]
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Section {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub cta: Option<String>,
    #[serde(default)]
    pub cta_route: Option<String>,
    #[serde(default)]
    pub cta2: Option<String>,
    #[serde(default)]
    pub cta2_route: Option<String>,
    #[serde(default)]
    pub button: Option<String>,
    #[serde(default)]
    pub button_route: Option<String>,
    #[serde(default)]
    pub points: Vec<String>,
    #[serde(default)]
    pub items: Vec<Item>,
    #[serde(default)]
    pub tiers: Vec<Tier>,
    #[serde(default)]
    pub kpis: Vec<Kpi>,
    #[serde(default)]
    pub paragraphs: Vec<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub submit: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub hours: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub a: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub day: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub meta: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub price: Option<i64>,
    #[serde(default)]
    pub old_price: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tier {
    pub name: String,
    #[serde(default)]
    pub price: i64,
    #[serde(default)]
    pub period: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub featured: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Kpi {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackTable {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    pub columns: Vec<PackColumn>,
    #[serde(default)]
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackFlow {
    pub button: String,
    pub kind: String,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub fields: Vec<FlowField>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub navigate: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlowField {
    pub column: String,
    pub value: Value,
}
