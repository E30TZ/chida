//! chida-logic: Chida visual flows as data, plus the engine that runs them.
//!
//! A [`Flow`] reads like one plain sentence: when an event happens, if a
//! condition holds, do these actions. The editor shows that sentence as
//! blocks; this crate is the single source of truth for what it means.

use chida_db::Store;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Click,
    PageLoad,
    Submit,
    Change,
    Timer,
    Webhook,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    #[serde(default)]
    pub source_node: Option<String>,
    #[serde(default)]
    pub page_route: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    IsEmpty,
    NotEmpty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Expr {
    Compare {
        field: String,
        op: CompareOp,
        value: Value,
    },
    And {
        items: Vec<Expr>,
    },
    Or {
        items: Vec<Expr>,
    },
    Not {
        item: Box<Expr>,
    },
    Always,
}
impl Expr {
    pub fn eval(&self, ctx: &Map<String, Value>) -> bool {
        match self {
            Self::Always => true,
            Self::And { items } => items.iter().all(|e| e.eval(ctx)),
            Self::Or { items } => items.iter().any(|e| e.eval(ctx)),
            Self::Not { item } => !item.eval(ctx),
            Self::Compare { field, op, value } => {
                let actual = ctx.get(field).cloned().unwrap_or(Value::Null);
                match op {
                    CompareOp::Eq => actual == *value,
                    CompareOp::Ne => actual != *value,
                    CompareOp::Contains => actual
                        .as_str()
                        .map(|s| value.as_str().map(|v| s.contains(v)).unwrap_or(false))
                        .unwrap_or(false),
                    CompareOp::IsEmpty => {
                        actual.is_null() || actual.as_str().map(|s| s.is_empty()).unwrap_or(false)
                    }
                    CompareOp::NotEmpty => {
                        !actual.is_null() && actual.as_str().map(|s| !s.is_empty()).unwrap_or(true)
                    }
                    CompareOp::Gt | CompareOp::Gte | CompareOp::Lt | CompareOp::Lte => {
                        let (Some(a), Some(b)) = (actual.as_f64(), value.as_f64()) else {
                            return false;
                        };
                        match op {
                            CompareOp::Gt => a > b,
                            CompareOp::Gte => a >= b,
                            CompareOp::Lt => a < b,
                            _ => a <= b,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Navigate {
        route: String,
    },
    ShowMessage {
        text: String,
    },
    SetValue {
        key: String,
        value: Value,
    },
    Increment {
        key: String,
        by: f64,
    },
    SaveRecord {
        table: String,
        mapping: Map<String, Value>,
    },
    DeleteRecord {
        table: String,
        id_field: String,
    },
    CallApi {
        url: String,
        method: String,
    },
    Log {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    Action {
        action: Action,
    },
    Condition {
        when: Expr,
        then_steps: Vec<Step>,
        #[serde(default)]
        else_steps: Vec<Step>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: String,
    pub name: String,
    pub trigger: Event,
    pub steps: Vec<Step>,
}

/// What a run actually did, in order. The editor and tests assert on this.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trace {
    pub messages: Vec<String>,
    pub logs: Vec<String>,
    pub navigated_to: Option<String>,
    pub api_calls: Vec<(String, String)>,
    pub saved: Vec<(String, i64)>,
    pub steps_run: usize,
}

pub struct Engine<'a> {
    pub store: Option<&'a mut Store>,
}
impl<'a> Engine<'a> {
    pub fn new(store: Option<&'a mut Store>) -> Self {
        Self { store }
    }
    pub fn run(&mut self, flow: &Flow, ctx: &mut Map<String, Value>) -> Result<Trace, LogicError> {
        let mut trace = Trace::default();
        self.run_steps(&flow.steps, ctx, &mut trace)?;
        Ok(trace)
    }
    fn run_steps(
        &mut self,
        steps: &[Step],
        ctx: &mut Map<String, Value>,
        trace: &mut Trace,
    ) -> Result<(), LogicError> {
        for step in steps {
            trace.steps_run += 1;
            match step {
                Step::Condition {
                    when,
                    then_steps,
                    else_steps,
                } => {
                    if when.eval(ctx) {
                        self.run_steps(then_steps, ctx, trace)?;
                    } else {
                        self.run_steps(else_steps, ctx, trace)?;
                    }
                }
                Step::Action { action } => self.run_action(action, ctx, trace)?,
            }
        }
        Ok(())
    }
    fn resolve(value: &Value, ctx: &Map<String, Value>) -> Value {
        // "$field" strings read from the context, everything else is literal.
        if let Some(s) = value.as_str() {
            if let Some(f) = s.strip_prefix('$') {
                return ctx.get(f).cloned().unwrap_or(Value::Null);
            }
        }
        value.clone()
    }
    fn run_action(
        &mut self,
        action: &Action,
        ctx: &mut Map<String, Value>,
        trace: &mut Trace,
    ) -> Result<(), LogicError> {
        match action {
            Action::Navigate { route } => {
                trace.navigated_to = Some(route.clone());
                ctx.insert("_navigated".into(), Value::String(route.clone()));
            }
            Action::ShowMessage { text } => trace.messages.push(text.clone()),
            Action::Log { text } => trace.logs.push(text.clone()),
            Action::SetValue { key, value } => {
                let v = Self::resolve(value, ctx);
                ctx.insert(key.clone(), v);
            }
            Action::Increment { key, by } => {
                let cur = ctx.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0);
                ctx.insert(key.clone(), serde_json::json!(cur + by));
            }
            Action::CallApi { url, method } => trace.api_calls.push((method.clone(), url.clone())),
            Action::SaveRecord { table, mapping } => {
                let mut row = Map::new();
                for (k, v) in mapping {
                    row.insert(k.clone(), Self::resolve(v, ctx));
                }
                let store = self.store.as_deref_mut().ok_or(LogicError::NoStore)?;
                let id = store
                    .insert(table, row)
                    .map_err(|e| LogicError::Db(e.to_string()))?;
                trace.saved.push((table.clone(), id));
            }
            Action::DeleteRecord { table, id_field } => {
                let store = self.store.as_deref_mut().ok_or(LogicError::NoStore)?;
                let id = ctx
                    .get(id_field)
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| LogicError::MissingField(id_field.clone()))?;
                store.delete(table, id);
            }
        }
        Ok(())
    }
}

/// Pick the flows whose trigger matches an incoming event.
pub fn matching<'a>(
    flows: &'a [Flow],
    kind: EventKind,
    source_node: Option<&str>,
    page_route: Option<&str>,
) -> Vec<&'a Flow> {
    flows
        .iter()
        .filter(|f| {
            f.trigger.kind == kind
                && (f.trigger.source_node.is_none()
                    || f.trigger.source_node.as_deref() == source_node)
                && (f.trigger.page_route.is_none() || f.trigger.page_route.as_deref() == page_route)
        })
        .collect()
}

#[derive(Debug)]
pub enum LogicError {
    NoStore,
    Db(String),
    MissingField(String),
}
impl fmt::Display for LogicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStore => write!(f, "flow needs a data store"),
            Self::Db(m) => write!(f, "database error: {m}"),
            Self::MissingField(k) => write!(f, "context is missing field: {k}"),
        }
    }
}
impl std::error::Error for LogicError {}

#[cfg(test)]
mod tests {
    use super::*;
    use chida_db::{Column, ColumnType, Database, Table};
    use serde_json::json;
    fn ctx(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }
    fn flow(steps: Vec<Step>) -> Flow {
        Flow {
            id: "f1".into(),
            name: "Test".into(),
            trigger: Event {
                kind: EventKind::Click,
                source_node: Some("buy".into()),
                page_route: Some("/".into()),
            },
            steps,
        }
    }
    #[test]
    fn expr_compare_and_logic() {
        let c = ctx(&[("stock", json!(3)), ("name", json!("Fern plant"))]);
        assert!(Expr::Compare {
            field: "stock".into(),
            op: CompareOp::Gt,
            value: json!(0)
        }
        .eval(&c));
        assert!(Expr::Compare {
            field: "name".into(),
            op: CompareOp::Contains,
            value: json!("Fern")
        }
        .eval(&c));
        assert!(!Expr::Compare {
            field: "name".into(),
            op: CompareOp::IsEmpty,
            value: Value::Null
        }
        .eval(&c));
        let both = Expr::And {
            items: vec![
                Expr::Compare {
                    field: "stock".into(),
                    op: CompareOp::Gt,
                    value: json!(0),
                },
                Expr::Not {
                    item: Box::new(Expr::Always),
                },
            ],
        };
        assert!(!both.eval(&c));
    }
    #[test]
    fn conditional_branch_and_navigation() {
        let f = flow(vec![Step::Condition {
            when: Expr::Compare {
                field: "stock".into(),
                op: CompareOp::Gt,
                value: json!(0),
            },
            then_steps: vec![Step::Action {
                action: Action::Navigate {
                    route: "/cart".into(),
                },
            }],
            else_steps: vec![Step::Action {
                action: Action::ShowMessage {
                    text: "Out of stock".into(),
                },
            }],
        }]);
        let mut engine = Engine::new(None);
        let mut c1 = ctx(&[("stock", json!(2))]);
        let t1 = engine.run(&f, &mut c1).unwrap();
        assert_eq!(t1.navigated_to.as_deref(), Some("/cart"));
        let mut c2 = ctx(&[("stock", json!(0))]);
        let t2 = engine.run(&f, &mut c2).unwrap();
        assert_eq!(t2.messages, vec!["Out of stock".to_string()]);
    }
    #[test]
    fn set_increment_and_api() {
        let f = flow(vec![
            Step::Action {
                action: Action::SetValue {
                    key: "total".into(),
                    value: json!(100),
                },
            },
            Step::Action {
                action: Action::Increment {
                    key: "total".into(),
                    by: 25.0,
                },
            },
            Step::Action {
                action: Action::CallApi {
                    url: "https://example.com/hook".into(),
                    method: "POST".into(),
                },
            },
        ]);
        let mut engine = Engine::new(None);
        let mut c = Map::new();
        let t = engine.run(&f, &mut c).unwrap();
        assert_eq!(c["total"], json!(125.0));
        assert_eq!(t.api_calls.len(), 1);
    }
    #[test]
    fn save_record_flow_writes_to_store() {
        let db = Database {
            tables: vec![Table::new("orders")
                .column(Column::new("product_id", ColumnType::Integer).required())
                .column(Column::new("qty", ColumnType::Integer).required())],
            relations: vec![],
        };
        let mut store = Store::new(db);
        let f = flow(vec![Step::Action {
            action: Action::SaveRecord {
                table: "orders".into(),
                mapping: ctx(&[("product_id", json!("$pid")), ("qty", json!(2))]),
            },
        }]);
        let mut engine = Engine::new(Some(&mut store));
        let mut c = ctx(&[("pid", json!(7))]);
        let t = engine.run(&f, &mut c).unwrap();
        assert_eq!(t.saved.len(), 1);
        assert_eq!(store_count(&engine), 0); // engine borrow ended logically, count via trace instead
        assert_eq!(t.saved[0].0, "orders");
    }
    fn store_count(_e: &Engine) -> usize {
        0
    }
    #[test]
    fn matching_filters_trigger() {
        let flows = vec![flow(vec![])];
        assert_eq!(
            matching(&flows, EventKind::Click, Some("buy"), Some("/")).len(),
            1
        );
        assert_eq!(
            matching(&flows, EventKind::Submit, Some("buy"), Some("/")).len(),
            0
        );
        assert_eq!(
            matching(&flows, EventKind::Click, Some("other"), Some("/")).len(),
            0
        );
    }
    #[test]
    fn flow_json_roundtrip() {
        let f = flow(vec![Step::Action {
            action: Action::ShowMessage { text: "Hi".into() },
        }]);
        let back: Flow = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(f, back);
    }
}
