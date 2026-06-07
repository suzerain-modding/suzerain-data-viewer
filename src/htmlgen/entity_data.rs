use super::*;

fn story_fragment_properties_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'StoryFragmentProperties' to be an object.");
    }

    let begin = v["OnStoryFragmentBeginInstruction"]
        .as_str()
        .unwrap_or_default();
    let end = v["OnStoryFragmentEndInstruction"]
        .as_str()
        .unwrap_or_default();
    let condition = v["StoryFragmentCondition"].as_str().unwrap_or_default();

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

    let app_bundle = v["AppBundle"].as_str().unwrap_or_default();
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

fn bill_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'BillData' to be an object.");
    }

    let name = v["NameInDatabase"].as_str().unwrap_or_default();
    let path = v["Path"].as_str().unwrap_or_default();
    let bp = &v["BillProperties"];
    let title = bp["Title"].as_str().unwrap_or_default();
    let hub_title = bp["HubTitle"].as_str().unwrap_or_default();
    let description = bp["Description"].as_str().unwrap_or_default();
    let hub_desc = bp["HubDescription"].as_str().unwrap_or_default();
    let is_veto_cond = bp["IsVetoDisabledCondition"].as_str().unwrap_or_default();
    let sign_vars = bp["SignVariables"].as_str().unwrap_or_default();
    let veto_vars = bp["VetoVariables"].as_str().unwrap_or_default();

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let sfp_html = story_fragment_properties_to_html(&v["StoryFragmentProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
  </div>
  <div class="props-grid">
    {}{}{}{}{}{}{}{}
  </div>
  {}{}
</div>"#,
        prop_row("NameInDatabase", name),
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
    ))
}

fn bills_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "BillData", bill_data_to_html, |el| {
        el["BillProperties"]["Title"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    })
}

fn conditional_instruction_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'ConditionalInstruction' to be an object.");
    }

    Ok(format!(
        r#"<div class="decision-option">{}{}</div>"#,
        optional_code_section("Condition", v["Condition"].as_str().unwrap_or_default()),
        optional_code_section("Instruction", v["Instruction"].as_str().unwrap_or_default()),
    ))
}

fn conditional_instructions_to_html(v: &Value) -> Result<String> {
    list_to_html(
        v,
        "ConditionalInstruction",
        conditional_instruction_to_html,
        |el| el["Instruction"].as_str().unwrap_or_default().to_owned(),
    )
}

fn conditional_instruction_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'ConditionalInstructionData' to be an object.");
    }

    let name = v["NameInDatabase"].as_str().unwrap_or_default();
    let path = v["Path"].as_str().unwrap_or_default();
    let p = &v["ConditionalInstructionProperties"];
    let check_on_step_no = p["CheckOnStepNo"].as_i64().unwrap_or(0);
    let check_on_turn_no = p["CheckOnTurnNo"].as_i64().unwrap_or(0);
    let check_per_step = p["CheckPerStep"].as_bool().unwrap_or(false);
    let check_per_story_fragment = p["CheckPerStoryFragment"].as_bool().unwrap_or(false);
    let check_per_turn = p["CheckPerTurn"].as_bool().unwrap_or(false);
    let is_one_time = p["IsOneTime"].as_bool().unwrap_or(false);
    let priority = p["Priority"].as_i64().unwrap_or(0);

    let instructions_html = conditional_instructions_to_html(&p["ConditionalInstructions"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{name_e}</span>
  </div>
  <div class="props-grid">
    {}{}{}{}{}{}{}{}{}
  </div>
  {}{}
</div>"#,
        prop_row("NameInDatabase", name),
        prop_row("Path", path),
        prop_row_num("Priority", priority),
        prop_row_num("CheckOnStepNo", check_on_step_no),
        prop_row_num("CheckOnTurnNo", check_on_turn_no),
        prop_row_bool("CheckPerStep", check_per_step),
        prop_row_bool("CheckPerStoryFragment", check_per_story_fragment),
        prop_row_bool("CheckPerTurn", check_per_turn),
        prop_row_bool("IsOneTime", is_one_time),
        collapsible_section("ConditionalInstructions", &instructions_html),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        name_e = escape_html(name),
    ))
}

fn conditional_instruction_data_list_to_html(v: &Value) -> Result<String> {
    list_to_html(
        v,
        "ConditionalInstructionData",
        conditional_instruction_data_to_html,
        |el| el["NameInDatabase"].as_str().unwrap_or_default().to_owned(),
    )
}

fn conversation_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'ConversationData' to be an object.");
    }

    let name = v["NameInDatabase"].as_str().unwrap_or_default();
    let path = v["Path"].as_str().unwrap_or_default();
    let p = &v["ConversationProperties"];
    let title = p["Title"].as_str().unwrap_or_default();
    let subtitle = p["Subtitle"].as_str().unwrap_or_default();
    let dialogue = p["Dialogue"].as_str().unwrap_or_default();
    let type_string = p["TypeString"].as_str().unwrap_or_default();
    let is_on_start = p["IsOnStart"].as_bool().unwrap_or(false);

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));
    let sfp_html = story_fragment_properties_to_html(&v["StoryFragmentProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
  </div>
  <div class="props-grid">
    {}{}{}{}{}{}
  </div>
  {}{}
</div>"#,
        prop_row("NameInDatabase", name),
        prop_row("Path", path),
        prop_row("Subtitle", subtitle),
        prop_row("TypeString", type_string),
        prop_row_bool("IsOnStart", is_on_start),
        optional_code_section("Dialogue", dialogue),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        collapsible_section("StoryFragmentProperties", &sfp_html),
        title_e = escape_html(title),
    ))
}

fn conversation_data_list_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "ConversationData", conversation_data_to_html, |el| {
        el["ConversationProperties"]["Title"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    })
}

fn decision_option_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DecisionOption' to be an object.");
    }

    Ok(format!(
        r#"<div class="decision-option">{}{}{}</div>"#,
        prop_row("Text", v["Text"].as_str().unwrap_or_default()),
        optional_code_section("Condition", v["Condition"].as_str().unwrap_or_default()),
        optional_code_section("Instruction", v["Instruction"].as_str().unwrap_or_default()),
    ))
}

fn decision_options_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DecisionOption", decision_option_to_html, |el| {
        el["Text"].as_str().unwrap_or_default().to_string()
    })
}

fn decision_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'DecisionData' to be an object.");
    }

    let name = v["NameInDatabase"].as_str().unwrap_or_default();
    let path = v["Path"].as_str().unwrap_or_default();
    let dp = &v["DecisionProperties"];
    let title = dp["Title"].as_str().unwrap_or_default();
    let hub_title = dp["HubTitle"].as_str().unwrap_or_default();
    let description = dp["Description"].as_str().unwrap_or_default();
    let hub_desc = dp["HubDescription"].as_str().unwrap_or_default();

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
  </div>
  <div class="props-grid">
    {}{}{}{}{}
  </div>
  {}{}{}
</div>"#,
        prop_row("NameInDatabase", name),
        prop_row("Path", path),
        prop_row("HubTitle", hub_title),
        prop_row("Description", description),
        prop_row("HubDescription", hub_desc),
        collapsible_section("Options", &options_html),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        collapsible_section("StoryFragmentProperties", &sfp_html),
        title_e = escape_html(title),
    ))
}

fn decisions_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "DecisionData", decision_data_to_html, |el| {
        el["DecisionProperties"]["Title"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    })
}

fn news_data_to_html(v: &Value) -> Result<String> {
    if !v.is_object() {
        bail!("Expected 'NewsData' to be an object.");
    }

    let name = v["NameInDatabase"].as_str().unwrap_or_default();
    let path = v["Path"].as_str().unwrap_or_default();
    let np = &v["NewsProperties"];
    let title = np["Title"].as_str().unwrap_or_default();
    let description = np["Description"].as_str().unwrap_or_default();
    let newspaper = np["Newspaper"].as_str().unwrap_or_default();
    let is_enabled_var = np["IsEnabledVariable"].as_str().unwrap_or_default();
    let index = np["Index"].as_i64().unwrap_or(0);
    let turn_no = np["TurnNo"].as_i64().unwrap_or(0);

    let app_bundle_html = app_bundle_properties_to_html(&v["AppBundleProperties"])
        .unwrap_or_else(|e| format!(r#"<span class="error-msg">⚠ {e}</span>"#));

    Ok(format!(
        r#"<div class="entity-card">
  <div class="entity-top-row">
    <span class="entity-title">{title_e}</span>
  </div>
  <div class="props-grid">
    {}{}{}{}{}{}{}
  </div>
  {}
</div>"#,
        prop_row("NameInDatabase", name),
        prop_row("Newspaper", newspaper),
        prop_row("Path", path),
        prop_row("Description", description),
        prop_row_num("Index", index),
        prop_row_num("TurnNo", turn_no),
        prop_row("IsEnabledVariable", is_enabled_var),
        collapsible_section("AppBundleProperties", &app_bundle_html),
        title_e = escape_html(title),
    ))
}

fn news_list_to_html(v: &Value) -> Result<String> {
    list_to_html(v, "NewsData", news_data_to_html, |el| {
        el["NewsProperties"]["Title"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    })
}

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
        (
            "conditionalInstructionData",
            "Conditional Instructions",
            "❔",
            conditional_instruction_data_list_to_html,
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
                let file_path = format!(
                    "out/entity_{key_lower}.html",
                    key_lower = key.to_lowercase()
                );
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
</div>"#,
                    label = label,
                    count = count,
                    content_html = content_html,
                );
                let content = html_document(&body, label);
                write(&file_path, content)?;

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
                    icon = icon,
                    label = label,
                    count = count,
                ));
            }
            Err(e) => {
                warn!("Failed to generate entity section '{key}': {e}");
            }
        }
        progress.inc(1);
    }

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
  </header>
  <div class="index-grid">
    {index_cards}
  </div>
</div>"#,
        root_type_escaped = root_type_escaped,
        index_cards = index_cards,
    );

    let content = html_document(&body, &format!("Index {root_type}"));
    write("out/index.html", content)?;

    Ok(())
}
