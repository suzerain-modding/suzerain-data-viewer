use anyhow::{Context, Result, bail};
use log::warn;
use serde_json::Value;
use std::fs::write;

pub fn list_to_html(
    v: &Value,
    name: &str,
    subfn: impl Fn(&Value) -> Result<String>,
) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'List<{name}>' to be an object.");
    }

    let mut result = String::new();
    result.push_str(
        "\
        <div class=\"list collapsible collapsed\">\
            <button class=\"toggle\" aria-expanded=\"false\">\
                <span class=\"chev\">▾</span>\
            </button>\
            <div class=\"collapsible-content\">",
        //  </div> added at end
        //</div> added at end
    );

    // Collect object entries and sort by numeric value of the key when possible.
    let obj = v.as_object().unwrap();
    let mut entries: Vec<(&String, &Value)> = obj.iter().collect();
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
                result.push_str(&format!(
                    "<div class=\"list-item collapsible collapsed\"> \
                        <button class=\"toggle\" aria-expanded=\"false\"> \
                            <span class=\"index\">[{index_str}]</span> \
                            <span class=\"chev\">▾</span> \
                        </button> \
                        <div class=\"collapsible-content\"> \
                            {element_html} \
                        </div> \
                    </div>",
                ));
            }
            Err(e) => {
                warn!("Failed to generate HTML for '{name}': {e}");
            }
        }
    }
    result.push_str("</div></div>");

    Ok(result)
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
        "<div class=\"field\"><p class=\"label\">title: {title}</p><p class=\"label\">typeString: {type_str}</p><p class=\"value\">{value}</p></div>"
    ))
}

pub fn fields_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "Field", field_to_html)
}

pub fn link_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'Link' to be an object.");
    }

    let destination_dialogue_id = v["destinationDialogueID"]
        .as_i64()
        .context("Expected 'destinationDialogueID' in 'Link' to be i64.")?;
    let origin_dialogue_id = v["originDialogueID"]
        .as_i64()
        .context("Expected 'originDialogueID' in 'Link' to be i64.")?;
    let priority = v["priority"]
        .as_str()
        .context("Expected 'priority' in 'Link' to be string.")?;

    Ok(format!(
        "<div class=\"link\"><p class=\"label\">destinationDialogueID: {destination_dialogue_id}</p><p class=\"label\">originDialogueID: {origin_dialogue_id}</p><p class=\"value\">{priority}</p></div>"
    ))
}

pub fn links_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "Link", link_to_html)
}

pub fn dialogue_entry_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DialogueEntry' to be an object.");
    }

    let fields_html = fields_to_html(&v["fields"])
        .context("Failed to generate HTML for 'fields' in 'DialogueEntry'.")?;
    let outgoing_links_html = links_to_html(&v["outgoingLinks"])
        .context("Failed to generate HTML for 'outgoingLinks' in 'DialogueEntry'.")?;
    let title = v["Title"]
        .as_str()
        .context("Expected 'Title' in 'DialogueEntry' to be string.")?;

    Ok(format!(
        "<div class=\"dialogue-entry\"><h3 class=\"title\">{title}</h3><div class=\"subsection\"><h4>fields</h4>{fields_html}</div><div class=\"subsection\"><h4>outgoingLinks</h4>{outgoing_links_html}</div></div>"
    ))
}

pub fn dialogue_entries_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DialogueEntry", dialogue_entry_to_html)
}

pub fn conversation_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'Conversation' to be an object.");
    }

    let dialogue_entries_html = dialogue_entries_to_html(&v["dialogueEntries"])
        .context("Failed to generate HTML for 'dialogueEntries' in 'Conversation'.")?;
    let title = v["Title"]
        .as_str()
        .context("Expected 'Title' in 'Conversation' to be string.")?;

    Ok(format!(
        "<div class=\"conversation\"><h2 class=\"title\">{title}</h2><div class=\"subsection\"><h4>dialogueEntries</h4>{dialogue_entries_html}</div></div>"
    ))
}

pub fn conversations_to_html_files(v: &Value) -> Result<()> {
    if !v.is_object() {
        bail!("Expected 'List<Conversation>' to be an object.");
    }

    for (index_str, element) in v.as_object().unwrap() {
        match conversation_to_html(element) {
            Ok(element_html) => {
                let file_path = format!("out/List_Conversation_{index_str}.html");
                let body = format!(
                    "<div class=\"file-item\"><p class=\"index\">[{index_str}]</p>{element_html}</div>"
                );
                let content = html_document(&body, &format!("Conversation {index_str}"));
                write(file_path, content)?;
            }
            Err(e) => {
                warn!("Failed to generate HTML for 'Conversation': {e}");
            }
        }
    }

    Ok(())
}

fn html_document(body: &str, title: &str) -> String {
    format!(
        "<!doctype html> \
        <html> \
            <head> \
                <meta charset=\"utf-8\"> \
                <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"> \
                <link href=\"./style.css\" rel=\"stylesheet\" /> \
                <title>{title}</title> \
            </head> \
            <body> \
                <main class=\"container\"> \
                    {body} \
                </main> \
                <script src=\"./script.js\"></script> \
            </body> \
        </html>"
    )
}
