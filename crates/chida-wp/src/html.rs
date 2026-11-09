//! A small, tolerant HTML parser producing a DOM the importer
//! walks. It is built for template markup (already stripped of PHP
//! by the importer preprocessor), not for arbitrary web pages:
//! mismatched close tags are ignored, `li` and `p` elements close
//! implicitly, and script/style bodies are kept as raw text so the
//! importer can drop them wholesale.

/// One element in the parsed tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Dom {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<DomChild>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DomChild {
    El(Dom),
    Text(String),
}

impl Dom {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn classes(&self) -> Vec<String> {
        self.attr("class")
            .map(|c| c.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }
    /// All descendant text, whitespace collapsed, in document order.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        collect_text(self, &mut out);
        collapse_ws(&out)
    }
}

fn collect_text(dom: &Dom, out: &mut String) {
    for child in &dom.children {
        match child {
            DomChild::Text(t) => {
                out.push_str(t);
                out.push(' ');
            }
            DomChild::El(el) => collect_text(el, out),
        }
    }
}

/// Collapse every run of whitespace to a single space and trim.
pub fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Parse markup into a synthetic root element (tag `#root`).
pub fn parse_html(input: &str) -> Dom {
    let mut root = Dom {
        tag: "#root".into(),
        attrs: Vec::new(),
        children: Vec::new(),
    };
    let mut stack: Vec<Dom> = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut text_start = 0usize;

    let flush_text = |stack: &mut Vec<Dom>, root: &mut Dom, text: &str| {
        if !text.is_empty() {
            push_child(stack, root, DomChild::Text(decode_entities(text)));
        }
    };

    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Comments and declarations are skipped entirely.
        if input[i..].starts_with("<!--") {
            flush_text(&mut stack, &mut root, &input[text_start..i]);
            i = input[i + 4..]
                .find("-->")
                .map(|p| i + 4 + p + 3)
                .unwrap_or(bytes.len());
            text_start = i;
            continue;
        }
        if input[i..].starts_with("<!") || input[i..].starts_with("<?") {
            flush_text(&mut stack, &mut root, &input[text_start..i]);
            i = input[i..]
                .find('>')
                .map(|p| i + p + 1)
                .unwrap_or(bytes.len());
            text_start = i;
            continue;
        }
        let Some(gt) = find_tag_end(input, i) else {
            break;
        };
        let tag_src = &input[i + 1..gt];
        flush_text(&mut stack, &mut root, &input[text_start..i]);
        i = gt + 1;
        text_start = i;
        let tag_src = tag_src.trim();
        if let Some(name) = tag_src.strip_prefix('/') {
            let name = name.split_whitespace().next().unwrap_or("").to_lowercase();
            close_element(&mut stack, &mut root, &name);
            continue;
        }
        let self_closing = tag_src.ends_with('/');
        let tag_src = tag_src.trim_end_matches('/');
        let mut parts = tag_src.splitn(2, |c: char| c.is_whitespace());
        let name = parts.next().unwrap_or("").to_lowercase();
        if name.is_empty() {
            continue;
        }
        let attrs = parse_attrs(parts.next().unwrap_or(""));
        // `li` and `p` close implicitly when a sibling opens.
        if (name == "li" || name == "p")
            && stack.last().map(|d| d.tag.as_str()) == Some(name.as_str())
        {
            let done = stack.pop().unwrap();
            push_child(&mut stack, &mut root, DomChild::El(done));
        }
        let mut el = Dom {
            tag: name.clone(),
            attrs,
            children: Vec::new(),
        };
        if name == "script" || name == "style" {
            // Raw text body up to the matching close tag.
            let close = format!("</{name}");
            match input[i..].find(&close) {
                Some(p) => {
                    let body = &input[i..i + p];
                    if !body.is_empty() {
                        el.children.push(DomChild::Text(body.to_string()));
                    }
                    i = input[i + p..]
                        .find('>')
                        .map(|q| i + p + q + 1)
                        .unwrap_or(bytes.len());
                    text_start = i;
                }
                None => {
                    i = bytes.len();
                    text_start = i;
                }
            }
            push_child(&mut stack, &mut root, DomChild::El(el));
            continue;
        }
        if self_closing || VOID_TAGS.contains(&name.as_str()) {
            push_child(&mut stack, &mut root, DomChild::El(el));
        } else {
            stack.push(el);
        }
    }
    flush_text(&mut stack, &mut root, &input[text_start..]);
    while let Some(done) = stack.pop() {
        push_child(&mut stack, &mut root, DomChild::El(done));
    }
    root
}

fn push_child(stack: &mut Vec<Dom>, root: &mut Dom, child: DomChild) {
    match stack.last_mut() {
        Some(top) => top.children.push(child),
        None => root.children.push(child),
    }
}

fn close_element(stack: &mut Vec<Dom>, root: &mut Dom, name: &str) {
    if let Some(pos) = stack.iter().rposition(|d| d.tag == name) {
        while stack.len() > pos {
            let done = stack.pop().unwrap();
            push_child(stack, root, DomChild::El(done));
        }
    }
    // Unmatched close tags are ignored.
}

/// Find the `>` ending a tag, honouring quoted attribute values.
fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = start + 1;
    while i < bytes.len() {
        match quote {
            Some(q) => {
                if bytes[i] == q {
                    quote = None;
                }
            }
            None => match bytes[i] {
                b'"' | b'\'' => quote = Some(bytes[i]),
                b'>' => return Some(i),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

fn parse_attrs(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let mut name = String::new();
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' {
            name.push(chars[i]);
            i += 1;
        }
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let q = chars[i];
                i += 1;
                while i < chars.len() && chars[i] != q {
                    value.push(chars[i]);
                    i += 1;
                }
                i += 1;
            } else {
                while i < chars.len() && !chars[i].is_whitespace() {
                    value.push(chars[i]);
                    i += 1;
                }
            }
        }
        if !name.is_empty() {
            out.push((name.to_lowercase(), decode_entities(&value)));
        }
    }
    out
}

/// Decode the small entity set templates actually use.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_void() {
        let dom = parse_html(r#"<div class="a b"><p>Hi <b>there</b></p><img src="x.jpg"></div>"#);
        let div = match &dom.children[0] {
            DomChild::El(d) => d,
            _ => panic!("expected element"),
        };
        assert_eq!(div.tag, "div");
        assert_eq!(div.classes(), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(div.children.len(), 2);
        assert_eq!(div.text_content(), "Hi there");
    }

    #[test]
    fn tolerates_mismatch_and_implied_close() {
        let dom = parse_html("<ul><li>one<li>two</ul><p>tail");
        let ul = match &dom.children[0] {
            DomChild::El(d) => d,
            _ => panic!("expected ul"),
        };
        assert_eq!(ul.children.len(), 2);
        assert_eq!(dom.children.len(), 2);
    }
}
