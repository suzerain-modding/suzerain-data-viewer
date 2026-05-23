use super::*;
use anyhow::Context;

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
        element["title"].as_str().unwrap_or_default().to_string()
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
        conditions_section = optional_code_section("conditionsString", conditions),
        script_section = optional_code_section("userScript", user_script),
    ))
}

pub fn dialogue_entries_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DialogueEntry", dialogue_entry_to_html, |element| {
        element["Title"].as_str().unwrap_or_default().to_string()
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
    {dialogue_entries_html}
  </div>
</div>"#,
        escape_html(title),
    ))
}

pub fn conversations_to_html_files(v: &Value, progress: &ProgressBar) -> Result<()> {
    let entries = object_entries(v)?;
    for (index_str, element) in entries {
        match conversation_to_html(element) {
            Ok(element_html) => {
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
{element_html}"#,
                    index_str = index_str,
                    element_html = element_html,
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
    let root_type_escaped = escape_html(root_type);
    let entries = object_entries(v)?;

    let mut items_html = String::new();
    for (index_str, element) in entries {
        let title = element["Title"].as_str().unwrap_or(index_str);
        let id = element["id"]
            .as_i64()
            .context("Expected 'id' in 'Conversation' to be i64.")?;
        let entry_count = element["dialogueEntries"]
            .as_object()
            .map(|o| o.len().saturating_sub(1))
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

    let total = object_entries(v)?.len();
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
