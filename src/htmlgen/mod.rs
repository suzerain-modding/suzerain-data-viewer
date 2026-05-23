mod conversations;
mod entity_data;

use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use log::warn;
use serde_json::Value;
use std::fs::write;

pub use conversations::*;
pub use entity_data::*;

/// Render a generic object keyed list into a collapsible HTML list.
pub fn list_to_html(
    v: &Value,
    name: &str,
    subfn: impl Fn(&Value) -> Result<String>,
    header_fn: impl Fn(&Value) -> String,
) -> Result<String> {
    let entries = object_entries(v)?;
    let mut result = String::new();
    result.push_str(&format!(r#"<div class="list" data-list-type="{name}">"#));

    for (index_str, element) in entries {
        match subfn(element) {
            Ok(element_html) => {
                let header_text = header_fn(element);
                let label_html = if header_text.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<span class="list-item-label">{}</span>"#,
                        escape_html(&header_text)
                    )
                };

                result.push_str(&format!(
                    r#"<div class="list-item collapsible collapsed">
  <div class="list-item-header">
    <button class="toggle" aria-expanded="false" aria-label="Toggle item {index_str}">
      <svg class="chevron" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
    </button>
    <span class="index-badge">[{index_str}]</span>
    {label_html}
  </div>
  <div class="collapsible-content">
    {element_html}
  </div>
</div>"#
                ));
            }
            Err(e) => {
                warn!("Failed to generate HTML for '{name}': {e}");
                result.push_str(&format!(
                    r#"<div class="list-item error"><span class="error-msg">⚠ Failed to render item [{index_str}]</span></div>"#
                ));
            }
        }
    }

    result.push_str("</div>");
    Ok(result)
}

fn object_entries(v: &Value) -> Result<Vec<(&String, &Value)>> {
    let obj = v.as_object().context("Expected object to be an object.")?;
    let mut entries: Vec<_> = obj.iter().filter(|(k, _)| k.as_str() != "_type").collect();
    entries.sort_by(|a, b| compare_keys(a.0, b.0));
    Ok(entries)
}

fn compare_keys(a: &str, b: &str) -> std::cmp::Ordering {
    let a_num = a.parse::<i64>();
    let b_num = b.parse::<i64>();
    match (a_num, b_num) {
        (Ok(ia), Ok(ib)) => ia.cmp(&ib),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => a.cmp(b),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn prop_row(label: &str, value: &str) -> String {
    let escaped = escape_html(value);
    format!(
        r#"<div class="prop-row"><span class="prop-label">{label}</span><span class="prop-value">{escaped}</span></div>"#
    )
}

fn prop_row_bool(label: &str, value: bool) -> String {
    let cls = if value {
        "prop-bool-true"
    } else {
        "prop-bool-false"
    };
    let text = if value { "true" } else { "false" };
    format!(
        r#"<div class="prop-row"><span class="prop-label">{label}</span><span class="prop-value prop-bool {cls}">{text}</span></div>"#
    )
}

fn prop_row_num(label: &str, value: i64) -> String {
    format!(
        r#"<div class="prop-row"><span class="prop-label">{label}</span><span class="prop-value prop-num">{value}</span></div>"#
    )
}

fn string_list_to_html(v: &Value) -> String {
    if !v.is_object() {
        return r#"<span class="field-value-empty">(not an object)</span>"#.to_string();
    }

    let mut items: Vec<_> = v
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| k.as_str() != "_type")
        .collect();

    if items.is_empty() {
        return r#"<span class="field-value-empty">(empty)</span>"#.to_string();
    }

    items.sort_by(|a, b| compare_keys(a.0, b.0));

    let tags = items
        .iter()
        .filter_map(|(_, v)| v.as_str())
        .map(|s| format!(r#"<span class="string-tag">{}</span>"#, escape_html(s)))
        .collect::<Vec<_>>()
        .join(" ");

    format!(r#"<div class="string-tag-list">{tags}</div>"#)
}

fn optional_code_section(label: &str, value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    format!(
        r#"<div class="entry-section">
  <h4 class="entry-section-label">{label}</h4>
  <pre class="code-block">{}</pre>
</div>"#,
        escape_html(value)
    )
}

fn collapsible_section(label: &str, content_html: &str) -> String {
    format!(
        r#"<div class="entry-section">
  <h4 class="entry-section-label collapsible-section-label">
    <button class="toggle section-toggle" aria-expanded="false">
      <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none"
           stroke="currentColor" stroke-width="2.5" stroke-linecap="round"
           stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
      {label}
    </button>
  </h4>
  <div class="collapsible-section-content collapsible collapsed">
    {content_html}
  </div>
</div>"#
    )
}

pub fn html_document(body: &str, title: &str, is_index: bool) -> String {
    let escaped_title = escape_html(title);
    let extra_script = if is_index {
        r#"<script>
  const searchInput = document.getElementById('search');
  const grid = document.getElementById('index-grid');
  const noResults = document.getElementById('no-results');
  if (searchInput && grid) {
    searchInput.addEventListener('input', () => {
      const q = searchInput.value.toLowerCase();
      let visible = 0;
      grid.querySelectorAll('.index-card').forEach(card => {
        const text = card.textContent.toLowerCase();
        const show = text.includes(q);
        card.style.display = show ? '' : 'none';
        if (show) visible++;
      });
      noResults.hidden = visible > 0;
    });
  }
</script>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{escaped_title}</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300..700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <div class="container">
    {body}
  </div>
  <script src="script.js"></script>
  {extra_script}
</body>
</html>"#
    )
}
