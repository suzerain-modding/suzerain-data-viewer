use crate::{cli::Output, index::*};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::{collections::HashSet, fs, path::PathBuf};

pub fn run(inputs: &[PathBuf], output: &Output, replace: bool) -> Result<Value> {
    let paths = input_files(inputs)?;
    let database = absolute_destination(&output.database)?;
    if paths.contains(&database) {
        bail!("The index must not overwrite an input dump.");
    }
    let mut db = open(&output.database, true)?;
    let tx = db.transaction()?;
    tx.execute_batch(SCHEMA)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", INDEX_VERSION)?;
    if replace {
        tx.execute_batch("DELETE FROM record_fts; DELETE FROM sources;")?;
    }
    let mut results = Vec::new();
    let mut changed = replace;
    for path in paths {
        eprintln!("Importing {}", path.display());
        let before = fs::metadata(&path)?;
        let data = read_json(&path)?;
        let hash = hash_file(&path)?;
        let after = fs::metadata(&path)?;
        if before.len() != after.len() || before.modified()? != after.modified()? {
            bail!(
                "Source changed while importing '{}'; retry with a completed dump.",
                path.display()
            );
        }
        let root = required_text(&data, "_type")?;
        let pack = match root {
            "<conversations Sordland>" => Some("Sordland"),
            "<conversations Rizia>" => Some("Rizia"),
            "<entity data>" | "<actor names>" => None,
            _ => bail!(
                "Unsupported dump type '{root}' in '{}'. This importer reads legacy Dumper exports.",
                path.display()
            ),
        };
        let file = path_string(&path);
        let existing = tx
            .query_row(
                "SELECT id FROM sources WHERE path=? AND sha256=?",
                params![file, hash],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(id) = existing {
            results
                .push(json!({"status":"already_imported","source":source(&tx,id,None,None,None)?}));
            continue;
        }
        tx.execute("INSERT INTO sources(path,sha256,root_type,pack,bytes,imported_unix,html_dir) VALUES(?,?,?,?,?,?,?)",
            params![file,hash,root,pack,before.len() as i64,stamp(),path_string(&path.with_extension(""))])?;
        let source_id = tx.last_insert_rowid();
        match root {
            "<entity data>" => import_entities(&tx, source_id, &data),
            "<actor names>" => import_actors(&tx, source_id, &data),
            _ => import_conversations(&tx, source_id, pack.unwrap(), &data),
        }
        .with_context(|| {
            format!(
                "Import failed for '{}'; the transaction was rolled back",
                path.display()
            )
        })?;
        results.push(json!({"status":"imported","source":source(&tx,source_id,None,None,None)?}));
        changed = true;
    }
    if changed {
        eprintln!("Updating full-text index...");
        tx.execute_batch("DELETE FROM record_fts; INSERT INTO record_fts(rowid,title,text,scripts) SELECT id,title,text,scripts FROM records;")?;
        tx.execute("INSERT INTO meta(key,value) VALUES('revision',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[stamp()])?;
    }
    let details = totals(&tx)?;
    let response = json!({"command":"import","index_version":INDEX_VERSION,"index_revision":revision(&tx)?,"results":results,"totals":details});
    if crate::output::encode(&response, output.format)?.len() as u64 > output.max_bytes {
        bail!(
            "Import summary exceeds --max-bytes {}; the import was rolled back. Increase the limit or import fewer files.",
            output.max_bytes
        );
    }
    tx.commit()?;
    Ok(response)
}

fn input_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for input in inputs {
        if input.is_dir() {
            for entry in fs::read_dir(input)? {
                let path = entry?.path();
                if path.is_file()
                    && path
                        .extension()
                        .is_some_and(|s| s.eq_ignore_ascii_case("json"))
                {
                    paths.push(path.canonicalize()?);
                }
            }
        } else {
            paths.push(
                input
                    .canonicalize()
                    .with_context(|| format!("Cannot find '{}'", input.display()))?,
            );
        }
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        bail!("No JSON input files were found.");
    }
    Ok(paths)
}

struct NewRecord<'a> {
    source: i64,
    kind: &'a str,
    packs: Vec<String>,
    conversation: Option<i64>,
    entry: Option<i64>,
    name: &'a str,
    title: &'a str,
    articy: Option<&'a str>,
    text: String,
    scripts: String,
    pointer: &'a str,
    raw: &'a Value,
    dialogue: Option<&'a str>,
}
fn insert_record(db: &Connection, record: NewRecord<'_>) -> Result<i64> {
    let pack = if record.packs.len() == 1 {
        record.packs.first().map(String::as_str)
    } else {
        None
    };
    db.prepare_cached("INSERT INTO records(source_id,kind,pack,conversation_id,entry_id,name,title,articy_id,text,scripts,json_pointer,raw_json,dialogue_title) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)")?
        .execute(params![record.source,record.kind,pack,record.conversation,record.entry,record.name,record.title,record.articy,
            record.text,record.scripts,record.pointer,serde_json::to_string(record.raw)?,record.dialogue])
        .with_context(||format!("Invalid or duplicate record at {}",record.pointer))?;
    let id = db.last_insert_rowid();
    for pack in &record.packs {
        db.prepare_cached("INSERT OR IGNORE INTO record_packs(record_id,pack) VALUES(?,?)")?
            .execute(params![id, pack])?;
    }
    Ok(id)
}
fn insert_script(
    db: &Connection,
    record: i64,
    language: &str,
    field: &str,
    pointer: &str,
    text: &str,
) -> Result<()> {
    if !text.is_empty() {
        db.prepare_cached(
            "INSERT INTO scripts(record_id,language,field,json_pointer,text) VALUES(?,?,?,?,?)",
        )?
        .execute(params![record, language, field, pointer, text])?;
    }
    Ok(())
}

fn import_conversations(db: &Connection, source_id: i64, pack: &str, data: &Value) -> Result<()> {
    for (ci, conversation) in members(&data["conversations"])? {
        let cp = format!("/conversations/{}", pointer_part(ci));
        let cid = required_int(conversation, "id")?;
        let title = required_text(conversation, "Title")?;
        let mut header = conversation.clone();
        header
            .as_object_mut()
            .context("Conversation must be an object")?
            .remove("dialogueEntries");
        insert_record(
            db,
            NewRecord {
                source: source_id,
                kind: "conversation",
                packs: vec![pack.into()],
                conversation: Some(cid),
                entry: None,
                name: title,
                title,
                articy: None,
                text: String::new(),
                scripts: String::new(),
                pointer: &cp,
                raw: &header,
                dialogue: None,
            },
        )?;
        for (ni, node) in members(&conversation["dialogueEntries"])? {
            let np = format!("{cp}/dialogueEntries/{}", pointer_part(ni));
            let nid = required_int(node, "id")?;
            let title = required_text(node, "Title")?;
            required_int(node, "ActorID")?;
            required_int(node, "ConversantID")?;
            if required_int(node, "conversationID")? != cid {
                bail!("Node parent conversation mismatch at {np}");
            }
            let condition = required_text(node, "conditionsString")?;
            let script = required_text(node, "userScript")?;
            let fields = members(&node["fields"])?;
            let mut text = Vec::new();
            let mut script_text = vec![condition, script];
            let mut articy = None;
            let mut articy_count = 0;
            let mut titles = HashSet::new();
            for (fi, field) in &fields {
                let name = required_text(field, "title")?;
                required_text(field, "typeString")?;
                if !field["value"].is_string() && !field["value"].is_null() {
                    bail!("Expected string or null field value at {np}/fields/{fi}");
                }
                let value = field["value"].as_str();
                if name == "Articy Id" {
                    articy_count += 1;
                    articy = value;
                }
                if name.starts_with("Sequence") {
                    if let Some(value) = value {
                        script_text.push(value);
                    }
                } else if let Some(value) = value
                    && name != "Actor"
                    && name != "Conversant"
                    && !name.ends_with("Id")
                {
                    text.push(value);
                }
                if !titles.insert(name) {
                    warn(
                        db,
                        source_id,
                        "duplicate_field_title",
                        &np,
                        &format!("Repeated field title '{name}'; raw values are retained."),
                    )?;
                }
            }
            if articy_count > 1 {
                articy = None;
            }
            let id = insert_record(
                db,
                NewRecord {
                    source: source_id,
                    kind: "node",
                    packs: vec![pack.into()],
                    conversation: Some(cid),
                    entry: Some(nid),
                    name: title,
                    title,
                    articy,
                    text: text.join("\n"),
                    scripts: script_text.join("\n"),
                    pointer: &np,
                    raw: node,
                    dialogue: None,
                },
            )?;
            insert_script(
                db,
                id,
                "lua",
                "conditionsString",
                &format!("{np}/conditionsString"),
                condition,
            )?;
            insert_script(
                db,
                id,
                "lua",
                "userScript",
                &format!("{np}/userScript"),
                script,
            )?;
            for (fi, field) in fields {
                let name = required_text(field, "title")?;
                let value = field["value"].as_str();
                if let Some(value) = value {
                    if name.starts_with("Sequence") {
                        insert_script(
                            db,
                            id,
                            "sequence",
                            name,
                            &format!("{np}/fields/{}/value", pointer_part(fi)),
                            value,
                        )?;
                    }
                    if value == "<error>" {
                        warn(
                            db,
                            source_id,
                            "serialization_sentinel",
                            &format!("{np}/fields/{fi}/value"),
                            "Literal <error> value; the legacy exporter may have failed to serialize this field.",
                        )?;
                    }
                }
            }
            for (li, link) in members(&node["outgoingLinks"])? {
                let lp = format!("{np}/outgoingLinks/{}", pointer_part(li));
                let dc = required_int(link, "destinationConversationID")?;
                let de = required_int(link, "destinationDialogueID")?;
                if required_int(link, "originConversationID")? != cid
                    || required_int(link, "originDialogueID")? != nid
                {
                    warn(
                        db,
                        source_id,
                        "link_origin_mismatch",
                        &lp,
                        "The link's explicit origin differs from its containing node; raw link retained.",
                    )?;
                }
                let connector = link["isConnector"]
                    .as_bool()
                    .context("Link isConnector must be boolean")?;
                db.prepare_cached("INSERT INTO edges(source_id,origin_record_id,destination_conversation_id,destination_entry_id,priority,is_connector,json_pointer,raw_json) VALUES(?,?,?,?,?,?,?,?)")?
                    .execute(params![source_id,id,dc,de,required_text(link,"priority")?,connector,lp,serde_json::to_string(link)?])?;
            }
        }
    }
    Ok(())
}

fn import_actors(db: &Connection, source_id: i64, data: &Value) -> Result<()> {
    for (ordinal, value) in members(&data["actorNames"])? {
        let name = value.as_str().context("Actor name must be a string")?;
        let pointer = format!("/actorNames/{}", pointer_part(ordinal));
        insert_record(
            db,
            NewRecord {
                source: source_id,
                kind: "ActorName",
                packs: vec![],
                conversation: None,
                entry: None,
                name,
                title: name,
                articy: None,
                text: name.into(),
                scripts: String::new(),
                pointer: &pointer,
                raw: value,
                dialogue: None,
            },
        )?;
    }
    Ok(())
}
fn entity_packs(record: &Value) -> Result<Vec<String>> {
    let mut packs = Vec::new();
    if let Some(pack) = record["StoryPack"].as_str() {
        packs.push(pack_name(pack));
    }
    if !record["AppBundleProperties"]["StoryPacks"].is_null() {
        for (_, value) in members(&record["AppBundleProperties"]["StoryPacks"])? {
            packs.push(pack_name(
                value.as_str().context("Story pack must be a string")?,
            ));
        }
    }
    packs.sort();
    packs.dedup();
    Ok(packs)
}

fn entity_text(value: &Value) -> String {
    let mut parts = Vec::new();
    fn walk(value: &Value, parts: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    if matches!(
                        key.as_str(),
                        "Title"
                            | "HubTitle"
                            | "Description"
                            | "HubDescription"
                            | "Subtitle"
                            | "Text"
                            | "TransitionTitle"
                    ) {
                        if let Some(text) = value.as_str() {
                            parts.push(text.to_string());
                        }
                    } else {
                        walk(value, parts);
                    }
                }
            }
            Value::Array(array) => {
                for value in array {
                    walk(value, parts);
                }
            }
            _ => {}
        }
    }
    walk(value, &mut parts);
    parts.join("\n")
}
fn entity_scripts(
    value: &Value,
    pointer: &str,
    result: &mut Vec<(String, String, String, String)>,
) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                let next = format!("{pointer}/{}", pointer_part(key));
                if let Some(text) = value.as_str() {
                    let language = if key.ends_with("Instruction")
                        || key.ends_with("Condition")
                        || matches!(key.as_str(), "SignVariables" | "VetoVariables")
                    {
                        Some("articy")
                    } else if key == "IsEnabledVariable" {
                        Some("variable")
                    } else {
                        None
                    };
                    if let Some(language) = language
                        && !text.is_empty()
                    {
                        result.push((language.into(), key.clone(), next, text.into()));
                    }
                } else {
                    entity_scripts(value, &next, result);
                }
            }
        }
        Value::Array(array) => {
            for (i, value) in array.iter().enumerate() {
                entity_scripts(value, &format!("{pointer}/{i}"), result);
            }
        }
        _ => {}
    }
}
fn import_entities(db: &Connection, source_id: i64, data: &Value) -> Result<()> {
    let categories = [
        ("AllBillsData", "BillData"),
        ("AllConversationsData", "ConversationData"),
        ("AllDecisionsData", "DecisionData"),
        ("conditionalInstructionData", "ConditionalInstructionData"),
        ("GameFlowData", "GameFlowData"),
        ("NewsData", "NewsData"),
    ];
    for (category, kind) in categories {
        for (ordinal, record) in members(&data[category])? {
            let pointer = format!("/{category}/{}", pointer_part(ordinal));
            let name = required_text(
                record,
                if kind == "GameFlowData" {
                    "StoryPack"
                } else {
                    "NameInDatabase"
                },
            )?;
            let mut scripts = Vec::new();
            entity_scripts(record, &pointer, &mut scripts);
            let title = match kind {
                "BillData" => record["BillProperties"]["Title"].as_str(),
                "ConversationData" => record["ConversationProperties"]["Title"].as_str(),
                "DecisionData" => record["DecisionProperties"]["Title"].as_str(),
                "NewsData" => record["NewsProperties"]["Title"].as_str(),
                _ => None,
            }
            .unwrap_or(name);
            let id = insert_record(
                db,
                NewRecord {
                    source: source_id,
                    kind,
                    packs: entity_packs(record)?,
                    conversation: None,
                    entry: None,
                    name,
                    title,
                    articy: None,
                    text: entity_text(record),
                    scripts: scripts
                        .iter()
                        .map(|s| s.3.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    pointer: &pointer,
                    raw: record,
                    dialogue: record["ConversationProperties"]["Dialogue"].as_str(),
                },
            )?;
            for (language, field, pointer, text) in scripts {
                insert_script(db, id, &language, &field, &pointer, &text)?;
            }
            if kind == "GameFlowData" {
                for (ti, turn) in members(&record["Turns"])? {
                    for (si, step) in members(&turn["Steps"])? {
                        for (fi, fragment) in members(&step["Fragments"])? {
                            db.prepare_cached("INSERT INTO schedule(source_id,flow_record_id,story_pack,turn_index,step_index,fragment,json_pointer) VALUES(?,?,?,?,?,?,?)")?
                                .execute(params![source_id,id,name,ti,si,fragment.as_str().context("Fragment name must be a string")?,
                                    format!("{pointer}/Turns/{}/Steps/{}/Fragments/{}",pointer_part(ti),pointer_part(si),pointer_part(fi))])?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
