use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;
use log::warn;
use serde_json::Value;
use std::fs::write;

pub fn list_to_html(
    v: &Value,
    name: &str,
    subfn: impl Fn(&Value) -> Result<String>,
    header_fn: impl Fn(&Value) -> String,
) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'List<{name}>' to be an object.");
    }

    let mut result = String::new();
    result.push_str(&format!(r#"<div class="list" data-list-type="{name}">"#));

    // Collect object entries and sort by numeric value of the key when possible.
    let obj = v.as_object().unwrap();
    let mut entries: Vec<(&String, &Value)> =
        obj.iter().filter(|(k, _)| k.as_str() != "_type").collect();
    entries.sort_by(|a, b| {
        let a_num = a.0.parse::<i64>();
        let b_num = b.0.parse::<i64>();
        match (a_num, b_num) {
            (Ok(ia), Ok(ib)) => ia.cmp(&ib),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a.0.cmp(b.0),
        }
    });

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

pub fn field_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'Field' to be an object.");
    }

    let title = v["title"]
        .as_str()
        .context("Expected 'title' in 'Field' to be string.")?;
    let type_str = v["typeString"]
        .as_str()
        .context("Expected 'typeString' in 'Field' to be string.")?;
    let value = v["value"]
        .as_str()
        .context("Expected 'value' in 'Field' to be string.")?;

    Ok(format!(
        r#"<div class="field">
  <div class="field-header">
    <span class="field-title">{}</span>
    <span class="field-type-badge">{}</span>
  </div>
  {}
</div>"#,
        escape_html(title),
        if type_str.is_empty() {
            "(empty type)".to_owned()
        } else {
            escape_html(type_str)
        },
        if value.is_empty() {
            r#"<span class="field-value field-value-empty">(empty)</span>"#.to_string()
        } else {
            format!(r#"<span class="field-value">{}</span>"#, escape_html(value))
        }
    ))
}

pub fn fields_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "Field", field_to_html, |element| {
        element["title"].as_str().unwrap_or("").to_string()
    })
}

pub fn link_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'Link' to be an object.");
    }

    let destination_conversation_id = v["destinationConversationID"]
        .as_i64()
        .context("Expected 'destinationConversationID' in 'Link' to be i64.")?;
    let destination_dialogue_id = v["destinationDialogueID"]
        .as_i64()
        .context("Expected 'destinationDialogueID' in 'Link' to be i64.")?;
    let is_connector = v["isConnector"]
        .as_bool()
        .context("Expected 'isConnector' in 'Link' to be bool.")?;
    let origin_conversation_id = v["originConversationID"]
        .as_i64()
        .context("Expected 'originConversationID' in 'Link' to be i64.")?;
    let origin_dialogue_id = v["originDialogueID"]
        .as_i64()
        .context("Expected 'originDialogueID' in 'Link' to be i64.")?;
    let priority = v["priority"]
        .as_str()
        .context("Expected 'priority' in 'Link' to be string.")?;

    let href = format!(
        "conversation{destination_conversation_id}.html#DialogueEntry-{destination_dialogue_id}"
    );

    Ok(format!(
        r#"<a class="link" href="{href}">
  <div class="link-arrow-row">
    <span class="link-endpoint">Conv <strong>{origin_conversation_id}</strong> · Entry <strong>{origin_dialogue_id}</strong></span>
    <svg class="link-arrow" width="20" height="16" viewBox="0 0 20 16" fill="none"><path d="M1 8h17M12 2l6 6-6 6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
    <span class="link-endpoint">Conv <strong>{destination_conversation_id}</strong> · Entry <strong>{destination_dialogue_id}</strong></span>
  </div>
  <div class="link-meta">
    {}{}
  </div>
</a>"#,
        prop_row_bool("isConnector", is_connector),
        prop_row("priority", priority),
        href = escape_html(&href),
    ))
}

pub fn links_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "Link", link_to_html, |element| {
        let origin = element["originDialogueID"]
            .as_i64()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "?".to_string());
        let destination = element["destinationDialogueID"]
            .as_i64()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "?".to_string());
        format!("Link {} → {}", origin, destination)
    })
}

pub fn dialogue_entry_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DialogueEntry' to be an object.");
    }

    let id = v["id"]
        .as_i64()
        .context("Expected 'id' in 'DialogueEntry' to be i64.")?;
    let actor_id = v["ActorID"]
        .as_i64()
        .context("Expected 'ActorID' in 'DialogueEntry' to be i64.")?;
    let conversant_id = v["ConversantID"]
        .as_i64()
        .context("Expected 'ConversantID' in 'DialogueEntry' to be i64.")?;
    let conversation_id = v["conversationID"]
        .as_i64()
        .context("Expected 'conversationID' in 'DialogueEntry' to be i64.")?;
    let title = v["Title"]
        .as_str()
        .context("Expected 'Title' in 'DialogueEntry' to be string.")?;
    let conditions = v["conditionsString"]
        .as_str()
        .context("Expected 'conditionsString' in 'DialogueEntry' to be string.")?;
    let user_script = v["userScript"]
        .as_str()
        .context("Expected 'userScript' in 'DialogueEntry' to be string.")?;

    let fields_html = fields_to_html(&v["fields"])
        .context("Failed to generate HTML for 'fields' in 'DialogueEntry'.")?;
    let outgoing_links_html = links_to_html(&v["outgoingLinks"])
        .context("Failed to generate HTML for 'outgoingLinks' in 'DialogueEntry'.")?;

    let conditions_section = if conditions.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="entry-section">
  <h4 class="entry-section-label">conditionsString</h4>
  <pre class="code-block">{}</pre>
</div>"#,
            escape_html(conditions)
        )
    };

    let script_section = if user_script.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="entry-section">
  <h4 class="entry-section-label">userScript</h4>
  <pre class="code-block">{}</pre>
</div>"#,
            escape_html(user_script)
        )
    };

    Ok(format!(
        r#"<div class="dialogue-entry" id="DialogueEntry-{id}">
  <div class="entry-top-row">
    <span class="entry-title">{title_escaped}</span>
    <div class="entry-meta-badges">
      <span class="meta-badge">id: {id}</span>
      <span class="meta-badge">ActorID: {actor_id}</span>
      <span class="meta-badge">ConversantID: {conversant_id}</span>
      <span class="meta-badge">conversationID: {conversation_id}</span>
    </div>
  </div>
  {conditions_section}
  {script_section}
  <div class="entry-section">
    <h4 class="entry-section-label collapsible-section-label">
      <button class="toggle section-toggle" aria-expanded="false">
        <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
        fields
      </button>
    </h4>
    <div class="collapsible-section-content collapsible collapsed">
      {fields_html}
    </div>
  </div>
  <div class="entry-section">
    <h4 class="entry-section-label collapsible-section-label">
      <button class="toggle section-toggle" aria-expanded="false">
        <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>
        outgoingLinks
      </button>
    </h4>
    <div class="collapsible-section-content collapsible collapsed">
      {outgoing_links_html}
    </div>
  </div>
</div>"#,
        title_escaped = escape_html(title),
    ))
}

pub fn dialogue_entries_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DialogueEntry", dialogue_entry_to_html, |element| {
        element["Title"].as_str().unwrap_or("").to_string()
    })
}

pub fn conversation_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'Conversation' to be an object.");
    }

    let id = v["id"]
        .as_i64()
        .context("Expected 'id' in 'Conversation' to be i64.")?;
    let title = v["Title"]
        .as_str()
        .context("Expected 'Title' in 'Conversation' to be string.")?;

    let dialogue_entries_html = dialogue_entries_to_html(&v["dialogueEntries"])
        .context("Failed to generate HTML for 'dialogueEntries' in 'Conversation'.")?;

    Ok(format!(
        r#"<div class="conversation">
  <h2 class="conversation-title">{}</h2>
  <div class="entry-meta-badges">
        <span class="meta-badge">id: {id}</span>
  </div>
  <div class="section-group">
    <h3 class="section-group-label">dialogueEntries</h3>
    {}
  </div>
</div>"#,
        escape_html(title),
        dialogue_entries_html,
    ))
}

pub fn conversations_to_html_files(v: &Value, progress: &ProgressBar) -> Result<()> {
    if !v.is_object() {
        bail!("Expected 'List' to be an object.");
    }

    let obj = v.as_object().unwrap();
    let mut entries: Vec<(&String, &Value)> =
        obj.iter().filter(|(k, _)| k.as_str() != "_type").collect();
    entries.sort_by(|a, b| {
        let a_num = a.0.parse::<i64>();
        let b_num = b.0.parse::<i64>();
        match (a_num, b_num) {
            (Ok(ia), Ok(ib)) => ia.cmp(&ib),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a.0.cmp(b.0),
        }
    });

    for (index_str, element) in entries {
        match conversation_to_html(element) {
            Ok(element_html) => {
                // conversation_to_html should have already checked id, so this is safe
                // to unwrap.
                let id = element["id"].as_i64().unwrap();
                let title = element["Title"].as_str().unwrap_or(index_str);
                let file_path = format!("out/conversation{id}.html");
                let body = format!(
                    r#"<div class="page-header">
  <a href="index.html" class="back-link">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
    Back
  </a>
  <div class="page-header-meta">
    <span class="index-badge large">[{index_str}]</span>
  </div>
</div>
{element_html}"#
                );
                let content = html_document(&body, &format!("[{index_str}] {title}"), false);
                write(file_path, content)?;
            }
            Err(e) => {
                warn!("Failed to generate HTML for 'Conversation': {e}");
            }
        }
        progress.inc(1);
    }

    Ok(())
}

pub fn generate_conversations_index(v: &Value, root_type: &str) -> Result<()> {
    if !v.is_object() {
        bail!("Expected 'List' to be an object.");
    }

    let root_type_escaped = escape_html(root_type);

    let obj = v.as_object().unwrap();
    let mut entries: Vec<(&String, &Value)> =
        obj.iter().filter(|(k, _)| k.as_str() != "_type").collect();
    entries.sort_by(|a, b| {
        let a_num = a.0.parse::<i64>();
        let b_num = b.0.parse::<i64>();
        match (a_num, b_num) {
            (Ok(ia), Ok(ib)) => ia.cmp(&ib),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a.0.cmp(b.0),
        }
    });

    let mut items_html = String::new();
    for (index_str, element) in &entries {
        let title = element["Title"].as_str().unwrap_or(index_str);
        let id = element["id"]
            .as_i64()
            .context("Expected 'id' in 'Conversation' to be i64.")?;
        let entry_count = element["dialogueEntries"]
            .as_object()
            .map(|o| o.len().saturating_sub(1)) // subtract _type key
            .unwrap_or(0);
        items_html.push_str(&format!(
            r#"<a href="conversation{id}.html" class="index-card">
  <div class="index-card-inner">
    <span class="index-card-num">[{index_str}] id: {id}</span>
    <span class="index-card-title">{title_escaped}</span>
    <span class="index-card-count">{entry_count} entries</span>
  </div>
  <svg class="index-card-arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
</a>"#,
            title_escaped = escape_html(title),
        ));
    }

    let total = entries.len();
    let body = format!(
        r#"<div class="index-page">
  <header class="index-header">
    <div class="index-header-icon">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
    </div>
    <div>
      <h1 class="index-heading">{root_type_escaped}</h1>
      <p class="index-subheading">{total} conversations</p>
    </div>
    <button class="theme-toggle" data-theme-toggle aria-label="Toggle theme">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
    </button>
  </header>
  <div class="search-bar-wrap">
    <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
    <input type="search" id="search" class="search-input" placeholder="Filter conversations…" aria-label="Filter conversations">
  </div>
  <div class="index-grid" id="index-grid">
    {items_html}
  </div>
  <p class="no-results" id="no-results" hidden>No conversations match your search.</p>
</div>"#
    );

    let content = html_document(&body, &format!("Index {root_type}"), true);
    write("out/index.html", content)?;
    Ok(())
}

fn html_document(body: &str, title: &str, is_index: bool) -> String {
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
<html lang="en" data-theme="dark">
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

// ════════════════════════════════════════════════════════════════
// ENTITY DATA
// ════════════════════════════════════════════════════════════════

// ── Shared sub-structures ─────────────────────────────────────────────────────

fn story_fragment_properties_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'StoryFragmentProperties' to be an object.");
    }
    let begin = v["OnStoryFragmentBeginInstruction"].as_str().unwrap_or("");
    let end = v["OnStoryFragmentEndInstruction"].as_str().unwrap_or("");
    let condition = v["StoryFragmentCondition"].as_str().unwrap_or("");
    Ok(format!(
        r#"<div class="props-block">{}{}{}</div>"#,
        optional_code_section("OnStoryFragmentBeginInstruction", begin),
        optional_code_section("OnStoryFragmentEndInstruction", end),
        optional_code_section("StoryFragmentCondition", condition),
    ))
}

fn app_bundle_properties_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'AppBundleProperties' to be an object.");
    }
    let app_bundle = v["AppBundle"].as_str().unwrap_or("");
    let story_packs_html = string_list_to_html(&v["StoryPacks"]);
    Ok(format!(
        r#"<div class="props-block">
  {}
  <div class="entry-section">
    <h4 class="entry-section-label">StoryPacks</h4>
    {}
  </div>
</div>"#,
        prop_row("AppBundle", app_bundle),
        story_packs_html,
    ))
}

fn string_list_to_html(v: &Value) -> String {
    if !v.is_object() {
        return r#"<span class="field-value-empty">(not an object)</span>"#.to_string();
    }
    let obj = v.as_object().unwrap();
    let mut items: Vec<(&String, &Value)> =
        obj.iter().filter(|(k, _)| k.as_str() != "_type").collect();
    if items.is_empty() {
        return r#"<span class="field-value-empty">(empty)</span>"#.to_string();
    }
    items.sort_by(|a, b| {
        let a_num = a.0.parse::<i64>();
        let b_num = b.0.parse::<i64>();
        match (a_num, b_num) {
            (Ok(ia), Ok(ib)) => ia.cmp(&ib),
            _ => a.0.cmp(b.0),
        }
    });
    let tags: String = items
        .iter()
        .filter_map(|(_, v)| v.as_str())
        .map(|s| format!(r#"<span class="string-tag">{}</span>"#, escape_html(s)))
        .collect::<Vec<_>>()
        .join(" ");
    format!(r#"<div class="string-tag-list">{tags}</div>"#)
}

// ── Helper re-used across all entity renderers ────────────────────────────────

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

fn prop_row_num(label: &str, value: i64) -> String {
    format!(
        r#"<div class="prop-row"><span class="prop-label">{label}</span><span class="prop-value prop-num">{value}</span></div>"#
    )
}

// ── BillData ──────────────────────────────────────────────────────────────────

fn bill_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'BillData' to be an object.");
    }
    let name = v["NameInDatabase"].as_str().unwrap_or("");
    let path = v["Path"].as_str().unwrap_or("");
    let bp = &v["BillProperties"];
    let title = bp["Title"].as_str().unwrap_or("");
    let hub_title = bp["HubTitle"].as_str().unwrap_or("");
    let description = bp["Description"].as_str().unwrap_or("");
    let hub_desc = bp["HubDescription"].as_str().unwrap_or("");
    let is_veto_cond = bp["IsVetoDisabledCondition"].as_str().unwrap_or("");
    let sign_vars = bp["SignVariables"].as_str().unwrap_or("");
    let veto_vars = bp["VetoVariables"].as_str().unwrap_or("");

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let sfp_html = story_fragment_properties_to_html(&v["StoryFragmentProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
    <div class="entry-meta-badges">
      <span class="meta-badge meta-badge-path">{name_e}</span>
    </div>
  </div>
  <div class="props-grid">
    {}{}{}{}{}{}{} 
  </div>
  {}{}
</div>"#,
        prop_row("Path", path),
        prop_row("HubTitle", hub_title),
        prop_row("Description", description),
        prop_row("HubDescription", hub_desc),
        optional_code_section("IsVetoDisabledCondition", is_veto_cond),
        optional_code_section("SignVariables", sign_vars),
        optional_code_section("VetoVariables", veto_vars),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        collapsible_section("StoryFragmentProperties", &sfp_html),
        title_e = escape_html(title),
        name_e = escape_html(name),
    ))
}

fn bills_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "BillData", bill_data_to_html, |el| {
        el["BillProperties"]["Title"]
            .as_str()
            .unwrap_or("")
            .to_string()
    })
}

// ── ConversationData ──────────────────────────────────────────────────────────

fn conversation_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'ConversationData' to be an object.");
    }
    let name = v["NameInDatabase"].as_str().unwrap_or("");
    let path = v["Path"].as_str().unwrap_or("");
    let cp = &v["ConversationProperties"];
    let title = cp["Title"].as_str().unwrap_or("");
    let subtitle = cp["Subtitle"].as_str().unwrap_or("");
    let dialogue = cp["Dialogue"].as_str().unwrap_or("");
    let type_string = cp["TypeString"].as_str().unwrap_or("");
    let is_on_start = cp["IsOnStart"].as_bool().unwrap_or(false);

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let sfp_html = story_fragment_properties_to_html(&v["StoryFragmentProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
    <div class="entry-meta-badges">
      <span class="meta-badge meta-badge-path">{name_e}</span>
    </div>
  </div>
  <div class="props-grid">
    {}{}{}{}{}
  </div>
  {}{}
</div>"#,
        prop_row("Path", path),
        prop_row("Subtitle", subtitle),
        prop_row("TypeString", type_string),
        prop_row_bool("IsOnStart", is_on_start),
        optional_code_section("Dialogue", dialogue),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        collapsible_section("StoryFragmentProperties", &sfp_html),
        title_e = escape_html(title),
        name_e = escape_html(name),
    ))
}

fn conversation_data_list_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "ConversationData", conversation_data_to_html, |el| {
        el["ConversationProperties"]["Title"]
            .as_str()
            .unwrap_or("")
            .to_string()
    })
}

// ── DecisionData ──────────────────────────────────────────────────────────────

fn decision_option_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DecisionOption' to be an object.");
    }
    Ok(format!(
        r#"<div class="decision-option">{}{}{}</div>"#,
        prop_row("Text", v["Text"].as_str().unwrap_or("")),
        optional_code_section("Condition", v["Condition"].as_str().unwrap_or("")),
        optional_code_section("Instruction", v["Instruction"].as_str().unwrap_or("")),
    ))
}

fn decision_options_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DecisionOption", decision_option_to_html, |el| {
        el["Text"].as_str().unwrap_or("").to_string()
    })
}

fn decision_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DecisionData' to be an object.");
    }
    let name = v["NameInDatabase"].as_str().unwrap_or("");
    let path = v["Path"].as_str().unwrap_or("");
    let dp = &v["DecisionProperties"];
    let title = dp["Title"].as_str().unwrap_or("");
    let hub_title = dp["HubTitle"].as_str().unwrap_or("");
    let description = dp["Description"].as_str().unwrap_or("");
    let hub_desc = dp["HubDescription"].as_str().unwrap_or("");

    let options_html = decision_options_to_html(&dp["Options"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let sfp_html = story_fragment_properties_to_html(&v["StoryFragmentProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
    <div class="entry-meta-badges">
      <span class="meta-badge meta-badge-path">{name_e}</span>
    </div>
  </div>
  <div class="props-grid">
    {}{}{}{}
  </div>
  {}{}{}
</div>"#,
        prop_row("Path", path),
        prop_row("HubTitle", hub_title),
        prop_row("Description", description),
        prop_row("HubDescription", hub_desc),
        collapsible_section("Options", &options_html),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        collapsible_section("StoryFragmentProperties", &sfp_html),
        title_e = escape_html(title),
        name_e = escape_html(name),
    ))
}

fn decisions_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DecisionData", decision_data_to_html, |el| {
        el["DecisionProperties"]["Title"]
            .as_str()
            .unwrap_or("")
            .to_string()
    })
}

// ── NewsData ──────────────────────────────────────────────────────────────────

fn news_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'NewsData' to be an object.");
    }
    let name = v["NameInDatabase"].as_str().unwrap_or("");
    let path = v["Path"].as_str().unwrap_or("");
    let np = &v["NewsProperties"];
    let title = np["Title"].as_str().unwrap_or("");
    let description = np["Description"].as_str().unwrap_or("");
    let newspaper = np["Newspaper"].as_str().unwrap_or("");
    let is_enabled_var = np["IsEnabledVariable"].as_str().unwrap_or("");
    let index = np["Index"].as_i64().unwrap_or(0);
    let turn_no = np["TurnNo"].as_i64().unwrap_or(0);

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
    <div class="entry-meta-badges">
      <span class="meta-badge">{newspaper_e}</span>
      <span class="meta-badge meta-badge-path">{name_e}</span>
    </div>
  </div>
  <div class="props-grid">
    {}{}{}{}{}
  </div>
  {}
</div>"#,
        prop_row("Path", path),
        prop_row("Description", description),
        prop_row_num("Index", index),
        prop_row_num("TurnNo", turn_no),
        prop_row("IsEnabledVariable", is_enabled_var),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        title_e = escape_html(title),
        name_e = escape_html(name),
        newspaper_e = escape_html(newspaper),
    ))
}

fn news_list_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "NewsData", news_data_to_html, |el| {
        el["NewsProperties"]["Title"]
            .as_str()
            .unwrap_or("")
            .to_string()
    })
}

// ── Entity data index + section pages (public entry point) ───────────────────

pub fn generate_entity_data_files(
    v: &Value,
    progress: &ProgressBar,
    root_type: &str,
) -> Result<()> {
    if !v.is_object() {
        bail!("Expected entity data root to be an object.");
    }

    let root_type_escaped = escape_html(root_type);

    type GenFn = fn(&Value) -> Result<String>;
    let sections: &[(&str, &str, &str, GenFn)] = &[
        ("AllBillsData", "Bills", "📜", bills_to_html),
        (
            "AllConversationsData",
            "Conversations",
            "💬",
            conversation_data_list_to_html,
        ),
        ("AllDecisionsData", "Decisions", "⚖", decisions_to_html),
        ("NewsData", "News", "📰", news_list_to_html),
    ];

    let mut index_cards = String::new();

    for (key, label, icon, gen_fn) in sections {
        let section_data = &v[key];
        let count = section_data
            .as_object()
            .map(|o| o.len().saturating_sub(1))
            .unwrap_or(0);

        progress.set_message(format!("Generating {label}…"));

        match gen_fn(section_data) {
            Ok(content_html) => {
                let file_path = format!("out/entity_{}.html", key.to_lowercase());
                let body = format!(
                    r#"<div class="page-header">
  <a href="index.html" class="back-link">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor"
         stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
      <polyline points="15 18 9 12 15 6"/>
    </svg>
    Back
  </a>
</div>
<div class="section-group" style="margin-top:var(--space-2)">
  <h2 class="conversation-title">{label}</h2>
  <p class="index-subheading" style="margin-bottom:var(--space-4)">{count} items</p>
  {content_html}
</div>"#
                );
                let content = html_document(&body, label, false);
                std::fs::write(&file_path, content)?;

                index_cards.push_str(&format!(
                    r#"<a href="entity_{key_lower}.html" class="index-card">
  <div class="index-card-inner">
    <span class="index-card-num entity-section-icon">{icon}</span>
    <span class="index-card-title">{label}</span>
    <span class="index-card-count">{count} items</span>
  </div>
  <svg class="index-card-arrow" width="16" height="16" viewBox="0 0 24 24" fill="none"
       stroke="currentColor" stroke-width="2.5" stroke-linecap="round"
       stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
</a>"#,
                    key_lower = key.to_lowercase(),
                ));
            }
            Err(e) => {
                log::warn!("Failed to generate entity section '{key}': {e}");
            }
        }
        progress.inc(1);
    }

    // Entity data index (no search bar needed — only 4 cards)
    let body = format!(
        r#"<div class="index-page">
  <header class="index-header">
    <div class="index-header-icon">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
      </svg>
    </div>
    <div>
      <h1 class="index-heading">{root_type_escaped}</h1>
      <p class="index-subheading">Select a category</p>
    </div>
    <button class="theme-toggle" data-theme-toggle aria-label="Toggle theme">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
           stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
    </button>
  </header>
  <div class="index-grid">
    {index_cards}
  </div>
</div>"#
    );

    let content = html_document(&body, &format!("Index {root_type}"), false);
    std::fs::write("out/index.html", content)?;

    Ok(())
}
