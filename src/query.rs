use crate::{
    cli::{Direction, Language, NodeSelector, Output, Page, Read, SearchKind},
    import::totals,
    index::*,
    output,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params, params_from_iter, types::Value as SqlValue};
use serde_json::{Value, json};

const FILTER: &str = "(?1 IS NULL OR r.source_id=?1) AND (?2 IS NULL OR EXISTS(SELECT 1 FROM record_packs p WHERE p.record_id=r.id AND p.pack=?2))";
const SOURCE_FILTER: &str = "(?1 IS NULL OR s.id=?1) AND (?2 IS NULL OR EXISTS(SELECT 1 FROM records r JOIN record_packs p ON p.record_id=r.id WHERE r.source_id=s.id AND p.pack=?2))";
const ORDER: &str = "r.source_id,r.kind,r.conversation_id,r.entry_id,r.name,r.id";

fn filters(read: &Read) -> Vec<SqlValue> {
    vec![
        read.source.map(SqlValue::Integer).unwrap_or(SqlValue::Null),
        read.pack
            .as_deref()
            .map(pack_name)
            .map(SqlValue::Text)
            .unwrap_or(SqlValue::Null),
    ]
}
struct Paging {
    offset: u64,
    signature: String,
    limit: u32,
}
fn paging(db: &Connection, read: &Read, page: &Page, key: Value) -> Result<Paging> {
    let signature = fingerprint(&format!(
        "{}:{}:{}:{}",
        revision(db)?,
        read.source.map(|v| v.to_string()).unwrap_or_default(),
        read.pack.as_deref().map(pack_name).unwrap_or_default(),
        key
    ));
    let offset = if let Some(cursor) = &page.cursor {
        let (offset, hash) = cursor.split_once('.').context("Invalid cursor")?;
        if hash != signature {
            bail!(
                "Cursor belongs to a different query or index revision; restart without --cursor."
            );
        }
        let offset = offset.parse::<u64>().context("Invalid cursor offset")?;
        if offset > i64::MAX as u64 {
            bail!("Cursor offset is too large.");
        }
        offset
    } else {
        0
    };
    Ok(Paging {
        offset,
        signature,
        limit: page.limit,
    })
}
fn finish(
    command: &str,
    db: &Connection,
    output: &Output,
    paging: Option<&Paging>,
    mut results: Vec<Value>,
    mut more: bool,
    details: Value,
) -> Result<Value> {
    loop {
        let next = paging
            .filter(|_| more)
            .map(|p| format!("{}.{}", p.offset + results.len() as u64, p.signature));
        let response = json!({"command":command,"index_version":INDEX_VERSION,"index_revision":revision(db)?,
            "results":results,"details":details,"page":{"returned":results.len(),"limit":paging.map(|p|p.limit),
                "offset":paging.map(|p|p.offset),"next_cursor":next,"truncated":more}});
        if output::encode(&response, output.format)?.len() as u64 <= output.max_bytes {
            return Ok(response);
        }
        if paging.is_some() && results.len() > 1 {
            results.pop();
            more = true;
        } else {
            bail!(
                "Response exceeds --max-bytes {}. Increase it (up to 16777216) or request a smaller record/view. No record was silently clipped.",
                output.max_bytes
            );
        }
    }
}
fn records(db: &Connection, sql: &str, arguments: Vec<SqlValue>) -> Result<Vec<Record>> {
    Ok(db
        .prepare(sql)?
        .query_map(params_from_iter(arguments), Record::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
fn page_records(
    db: &Connection,
    sql: &str,
    mut arguments: Vec<SqlValue>,
    page: &Paging,
) -> Result<(Vec<Record>, bool)> {
    arguments.push(SqlValue::Integer(page.limit as i64 + 1));
    arguments.push(SqlValue::Integer(page.offset as i64));
    let mut result = records(db, &format!("{sql} LIMIT ? OFFSET ?"), arguments)?;
    let more = result.len() > page.limit as usize;
    if more {
        result.pop();
    }
    Ok((result, more))
}

pub fn sources(read: &Read, page: &Page) -> Result<Value> {
    let db = open(&read.output.database, false)?;
    let paging = paging(&db, read, page, json!("sources"))?;
    let mut arguments = filters(read);
    arguments.push((page.limit as i64 + 1).into());
    arguments.push((paging.offset as i64).into());
    let mut ids = db
        .prepare(&format!(
            "SELECT s.id FROM sources s WHERE {SOURCE_FILTER} ORDER BY s.id LIMIT ? OFFSET ?"
        ))?
        .query_map(params_from_iter(arguments), |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = ids.len() > page.limit as usize;
    if more {
        ids.pop();
    }
    let mut results = Vec::new();
    for id in ids {
        let mut item = source(&db, id, None, None, None)?;
        let count: i64 = db.query_row(
            "SELECT count(*) FROM records WHERE source_id=?",
            [id],
            |r| r.get(0),
        )?;
        item["record_count"] = json!(count);
        results.push(item);
    }
    finish(
        "sources",
        &db,
        &read.output,
        Some(&paging),
        results,
        more,
        totals(&db)?,
    )
}

pub fn search(read: &Read, page: &Page, text: Option<&str>, kind: SearchKind) -> Result<Value> {
    let db = open(&read.output.database, false)?;
    let text = text.map(str::trim).filter(|v| !v.is_empty());
    let paging = paging(
        &db,
        read,
        page,
        json!(["search", text, format!("{kind:?}")]),
    )?;
    let mut sql = format!("SELECT {RECORD_COLUMNS} FROM records r WHERE {FILTER}");
    let mut arguments = filters(read);
    match kind {
        SearchKind::Conversations => sql.push_str(" AND r.kind='conversation'"),
        SearchKind::Nodes => sql.push_str(" AND r.kind='node'"),
        SearchKind::Entities => sql.push_str(" AND r.kind NOT IN ('conversation','node')"),
        SearchKind::All => {}
    }
    if let Some(text) = text {
        sql.push_str(" AND (r.id IN (SELECT rowid FROM record_fts WHERE record_fts MATCH ?) OR instr(lower(r.name),lower(?))>0 OR instr(lower(r.title),lower(?))>0)");
        arguments.extend([
            SqlValue::Text(format!("\"{}\"", text.replace('"', "\"\""))),
            text.to_string().into(),
            text.to_string().into(),
        ]);
    }
    sql.push_str(&format!(" ORDER BY {ORDER}"));
    let (rows, more) = page_records(&db, &sql, arguments, &paging)?;
    let results = rows
        .iter()
        .map(|r| r.summary(&db))
        .collect::<Result<Vec<_>>>()?;
    finish(
        "search",
        &db,
        &read.output,
        Some(&paging),
        results,
        more,
        json!({"query":text,"kind":format!("{kind:?}").to_lowercase(),"search":"literal full-text phrase plus title/name substring"}),
    )
}

fn resolve_node(db: &Connection, read: &Read, selector: &NodeSelector) -> Result<Record> {
    let mut sql =
        format!("SELECT {RECORD_COLUMNS} FROM records r WHERE {FILTER} AND r.kind='node'");
    let mut arguments = filters(read);
    if let Some(cid) = selector.conversation_id {
        sql.push_str(" AND r.conversation_id=?");
        arguments.push(cid.into());
    }
    if let Some(entry) = selector.entry_id {
        sql.push_str(" AND r.entry_id=?");
        arguments.push(entry.into());
    } else if let Some(articy) = &selector.articy_id {
        sql.push_str(" AND r.articy_id=? COLLATE NOCASE");
        arguments.push(articy.clone().into());
    } else {
        bail!("Supply --conversation-id and --entry-id, or --articy-id.");
    }
    sql.push_str(&format!(" ORDER BY {ORDER} LIMIT 2"));
    let mut rows = records(db, &sql, arguments)?;
    if rows.is_empty() {
        bail!("No node matched the supplied identity and source/pack filters.");
    }
    if rows.len() > 1 {
        bail!(
            "Ambiguous node identity (source IDs {} and {}). Use --source and/or --conversation-id; captures are never chosen implicitly.",
            rows[0].source_id,
            rows[1].source_id
        );
    }
    Ok(rows.remove(0))
}
fn node_key(selector: &NodeSelector) -> Value {
    json!({"conversation_id":selector.conversation_id,"entry_id":selector.entry_id,"articy_id":selector.articy_id})
}

pub fn node(read: &Read, selector: &NodeSelector, raw: bool) -> Result<Value> {
    let db = open(&read.output.database, false)?;
    let record = resolve_node(&db, read, selector)?;
    let result = if raw {
        json!({"record":serde_json::from_str::<Value>(&record.raw)?,"source":record.source(&db)?})
    } else {
        node_view(&db, &record)?
    };
    finish(
        "node",
        &db,
        &read.output,
        None,
        vec![result],
        false,
        json!({"raw":raw}),
    )
}

fn links(db: &Connection, record: &Record) -> Result<Value> {
    let mut rows=db.prepare("SELECT e.raw_json,e.json_pointer,EXISTS(SELECT 1 FROM records n WHERE n.source_id=e.source_id AND n.kind='node' AND n.conversation_id=e.destination_conversation_id AND n.entry_id=e.destination_entry_id) FROM edges e WHERE e.origin_record_id=? ORDER BY e.id LIMIT 201")?
        .query_map([record.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,bool>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let truncated = rows.len() > 200;
    if truncated {
        rows.pop();
    }
    let mut result = Vec::new();
    for (raw, pointer, resolved) in rows {
        let mut value: Value = serde_json::from_str(&raw)?;
        value["source_json_pointer"] = json!(pointer);
        value["resolved_in_source"] = json!(resolved);
        result.push(value);
    }
    Ok(json!({"results":result,"truncated":truncated,"limit":200}))
}
fn node_view(db: &Connection, record: &Record) -> Result<Value> {
    let raw: Value = serde_json::from_str(&record.raw)?;
    let fields = members(&raw["fields"])?;
    let field = |name: &str| -> Value {
        let values: Vec<_> = fields
            .iter()
            .filter(|(_, v)| v["title"].as_str() == Some(name))
            .map(|(_, v)| v["value"].clone())
            .collect();
        match values.len() {
            0 => Value::Null,
            1 => values[0].clone(),
            _ => json!({"ambiguous_values":values}),
        }
    };
    let mut text = serde_json::Map::new();
    let mut menu = serde_json::Map::new();
    menu.insert("default".into(), field("Menu Text"));
    let mut sequences = serde_json::Map::new();
    let mut localized_title = serde_json::Map::new();
    for (_, value) in &fields {
        let name = value["title"].as_str().unwrap_or_default();
        if name == "Sequence" || name.starts_with("Sequence ") {
            sequences.insert(name.into(), field(name));
        } else if let Some(language) = name.strip_prefix("Menu Text ") {
            menu.insert(language.into(), field(name));
        } else if let Some(language) = name.strip_prefix("Title ") {
            localized_title.insert(language.into(), field(name));
        } else if value["typeString"].as_str() == Some("CustomFieldType_Localization") {
            text.insert(name.into(), field(name));
        }
    }
    let conversation_title: String = db.query_row(
        "SELECT name FROM records WHERE source_id=? AND kind='conversation' AND conversation_id=?",
        params![record.source_id, record.conversation_id],
        |r| r.get(0),
    )?;
    let mut result = record.summary(db)?;
    result["conversation_title"] = json!(conversation_title);
    result["actor_id"] = raw["ActorID"].clone();
    result["actor_name"] = Value::Null;
    result["conversant_id"] = raw["ConversantID"].clone();
    result["text"] = Value::Object(text);
    result["default_dialogue_text"] = field("Dialogue Text");
    result["menu_text"] = Value::Object(menu);
    result["localized_title"] = Value::Object(localized_title);
    result["sequences"] = Value::Object(sequences);
    result["condition"] = json!({"language":"lua","source":raw["conditionsString"]});
    result["script"] = json!({"language":"lua","source":raw["userScript"]});
    result["outgoing_links"] = links(db, record)?;
    let context = records(
        db,
        &format!(
            "SELECT {RECORD_COLUMNS} FROM records r WHERE kind='ConversationData' AND dialogue_title=? ORDER BY source_id,id LIMIT 21"
        ),
        vec![conversation_title.into()],
    )?;
    let more = context.len() > 20;
    result["entity_context"] = json!({"results":context.iter().take(20).map(|r|entity_view(db,r)).collect::<Result<Vec<_>>>()?,
        "truncated":more,"capture_relationship":"Matched by title only; sources retain independent provenance."});
    Ok(result)
}

pub fn neighbors(
    read: &Read,
    page: &Page,
    selector: &NodeSelector,
    depth: u8,
    direction: Direction,
) -> Result<Value> {
    let db = open(&read.output.database, false)?;
    let root = resolve_node(&db, read, selector)?;
    let paging = paging(
        &db,
        read,
        page,
        json!([
            "neighbors",
            node_key(selector),
            depth,
            format!("{direction:?}")
        ]),
    )?;
    let mut branches = Vec::new();
    if matches!(direction, Direction::Outgoing | Direction::Both) {
        branches.push("SELECT n.id,w.depth+1 FROM walk w JOIN edges e ON e.origin_record_id=w.id JOIN records n ON n.source_id=e.source_id AND n.kind='node' AND n.conversation_id=e.destination_conversation_id AND n.entry_id=e.destination_entry_id WHERE w.depth<?2");
    }
    if matches!(direction, Direction::Incoming | Direction::Both) {
        branches.push("SELECT e.origin_record_id,w.depth+1 FROM walk w JOIN records n ON n.id=w.id JOIN edges e ON e.source_id=n.source_id AND e.destination_conversation_id=n.conversation_id AND e.destination_entry_id=n.entry_id WHERE w.depth<?2");
    }
    let sql = format!(
        "WITH RECURSIVE walk(id,depth) AS (SELECT ?1,0 UNION {}) SELECT {RECORD_COLUMNS},min(w.depth) FROM walk w JOIN records r ON r.id=w.id WHERE w.id!=?1 GROUP BY r.id ORDER BY min(w.depth),r.conversation_id,r.entry_id,r.id LIMIT ?3 OFFSET ?4",
        branches.join(" UNION ")
    );
    let mut rows = db
        .prepare(&sql)?
        .query_map(
            params![
                root.id,
                depth as i64,
                page.limit as i64 + 1,
                paging.offset as i64
            ],
            |r| Ok((Record::from_row(r)?, r.get::<_, i64>(13)?)),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = rows.len() > page.limit as usize;
    if more {
        rows.pop();
    }
    let mut results = Vec::new();
    for (record, distance) in rows {
        let mut result = record.summary(&db)?;
        result["distance"] = json!(distance);
        result["outgoing_links"] = links(&db, &record)?;
        results.push(result);
    }
    finish(
        "neighbors",
        &db,
        &read.output,
        Some(&paging),
        results,
        more,
        json!({"root":root.summary(&db)?,"root_outgoing_links":links(&db,&root)?,"depth":depth,"direction":format!("{direction:?}").to_lowercase(),
        "graph_scope":"Originating conversation dump only; independent captures are not combined."}),
    )
}

pub fn references(
    read: &Read,
    page: &Page,
    variable: &str,
    language: Option<Language>,
) -> Result<Value> {
    if variable.is_empty() {
        bail!("Reference text cannot be empty.");
    }
    let db = open(&read.output.database, false)?;
    let paging = paging(
        &db,
        read,
        page,
        json!(["references", variable, language.map(Language::name)]),
    )?;
    let sql = format!(
        "SELECT {RECORD_COLUMNS} FROM records r WHERE {FILTER} AND EXISTS(SELECT 1 FROM scripts s WHERE s.record_id=r.id AND instr(s.text,?3)>0 AND (?4 IS NULL OR s.language=?4)) ORDER BY {ORDER}"
    );
    let mut arguments = filters(read);
    arguments.push(variable.to_string().into());
    arguments.push(
        language
            .map(|l| l.name().to_string().into())
            .unwrap_or(SqlValue::Null),
    );
    let (rows, more) = page_records(&db, &sql, arguments, &paging)?;
    let mut results = Vec::new();
    for record in rows {
        let mut result = record.summary(&db)?;
        let mut matches=db.prepare("SELECT language,field,json_pointer,text FROM scripts WHERE record_id=? AND instr(text,?)>0 AND (? IS NULL OR language=?) ORDER BY id LIMIT 21")?
            .query_map(params![record.id,variable,language.map(Language::name),language.map(Language::name)],|r| {
                let full=r.get::<_,String>(3)?;let (text,truncated)=excerpt(&full,800);
                Ok(json!({"language":r.get::<_,String>(0)?,"field":r.get::<_,String>(1)?,"json_pointer":r.get::<_,String>(2)?,"text_excerpt":text,"excerpt_truncated":truncated,"analysis":"literal textual match; read/write role not inferred"}))
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
        let truncated = matches.len() > 20;
        if truncated {
            matches.pop();
        }
        result["matches"] = json!({"results":matches,"truncated":truncated});
        results.push(result);
    }
    finish(
        "references",
        &db,
        &read.output,
        Some(&paging),
        results,
        more,
        json!({"text":variable,"case_sensitive":true,"language":language.map(Language::name),"semantics":"Textual matches are not proof that a script executes or reads/writes a variable."}),
    )
}

fn entity_view(db: &Connection, record: &Record) -> Result<Value> {
    let mut result = record.summary(db)?;
    let raw: Value = serde_json::from_str(&record.raw)?;
    result["properties"] = raw;
    let mut locations=db.prepare("SELECT story_pack,turn_index,step_index,json_pointer FROM schedule WHERE source_id=? AND fragment=? ORDER BY CAST(turn_index AS INTEGER),CAST(step_index AS INTEGER),json_pointer LIMIT 21")?
        .query_map(params![record.source_id,record.name],|r|Ok(json!({"story_pack":r.get::<_,String>(0)?,"turn_list_index":r.get::<_,String>(1)?,"step_list_index":r.get::<_,String>(2)?,"json_pointer":r.get::<_,String>(3)?})))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = locations.len() > 20;
    if more {
        locations.pop();
    }
    result["gameflow_locations"] = json!({"results":locations,"truncated":more,"indices":"Raw zero-based list indices, not displayed game turn/step numbers."});
    let mut scripts=db.prepare("SELECT language,field,json_pointer,text FROM scripts WHERE record_id=? ORDER BY id LIMIT 21")?
        .query_map([record.id],|r|Ok(json!({"language":r.get::<_,String>(0)?,"field":r.get::<_,String>(1)?,"json_pointer":r.get::<_,String>(2)?,"source":r.get::<_,String>(3)?})))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = scripts.len() > 20;
    if more {
        scripts.pop();
    }
    result["scripts"] = json!({"results":scripts,"truncated":more});
    if let Some(dialogue) = &record.dialogue {
        let matches = records(
            db,
            &format!(
                "SELECT {RECORD_COLUMNS} FROM records r WHERE kind='conversation' AND name=? ORDER BY source_id,id LIMIT 21"
            ),
            vec![dialogue.clone().into()],
        )?;
        result["dialogue_matches"] = json!({"results":matches.iter().take(20).map(|r|r.summary(db)).collect::<Result<Vec<_>>>()?,"truncated":matches.len()>20,
            "capture_relationship":"Matched by title; each source is a separate capture."});
    }
    if record.kind == "ActorName" {
        result["actor_id"] = Value::Null;
        result["export_ordinal"] = json!(record.pointer.rsplit('/').next());
    }
    Ok(result)
}
pub fn entity(read: &Read, name: &str, kind: Option<&str>, raw: bool) -> Result<Value> {
    let db = open(&read.output.database, false)?;
    let sql = format!(
        "SELECT {RECORD_COLUMNS} FROM records r WHERE {FILTER} AND r.kind NOT IN ('node','conversation') AND r.name=?3 AND (?4 IS NULL OR r.kind=?4) ORDER BY {ORDER} LIMIT 2"
    );
    let mut arguments = filters(read);
    arguments.push(name.to_string().into());
    arguments.push(kind.map(|k| k.to_string().into()).unwrap_or(SqlValue::Null));
    let mut rows = records(&db, &sql, arguments)?;
    if rows.is_empty() {
        bail!("No entity matched '{name}'. Use search --kind entities to find names.");
    }
    if rows.len() > 1 {
        bail!("Ambiguous entity '{name}'. Use --source and --kind to select the capture/type.");
    }
    let record = rows.remove(0);
    let result = if raw {
        json!({"record":serde_json::from_str::<Value>(&record.raw)?,"source":record.source(&db)?})
    } else {
        entity_view(&db, &record)?
    };
    finish(
        "entity",
        &db,
        &read.output,
        None,
        vec![result],
        false,
        json!({"raw":raw}),
    )
}

pub fn validate(read: &Read, page: &Page, check_files: bool) -> Result<(Value, bool)> {
    let db = open(&read.output.database, false)?;
    let paging = paging(&db, read, page, json!(["validate", check_files]))?;
    let cte=format!("WITH selected AS (SELECT s.id FROM sources s WHERE {SOURCE_FILTER}), diagnostics AS (
        SELECT source_id,code,json_pointer,message FROM issues WHERE source_id IN (SELECT id FROM selected)
        UNION ALL SELECT e.source_id,'unresolved_link',e.json_pointer,'Target conversation '||e.destination_conversation_id||', entry '||e.destination_entry_id||' is not present in this source export.' FROM edges e
        WHERE e.source_id IN (SELECT id FROM selected) AND NOT EXISTS(SELECT 1 FROM records n WHERE n.source_id=e.source_id AND n.kind='node' AND n.conversation_id=e.destination_conversation_id AND n.entry_id=e.destination_entry_id)
        UNION ALL SELECT source_id,'duplicate_articy_id',min(json_pointer),'Articy ID '||articy_id||' appears on '||count(*)||' nodes in conversation '||conversation_id||'; use conversation and entry IDs for exact lookup.' FROM records
        WHERE source_id IN (SELECT id FROM selected) AND kind='node' AND articy_id IS NOT NULL GROUP BY source_id,conversation_id,articy_id COLLATE NOCASE HAVING count(*)>1)");
    let warnings: i64 = db.query_row(
        &format!("{cte} SELECT count(*) FROM diagnostics"),
        params_from_iter(filters(read)),
        |r| r.get(0),
    )?;
    let mut warning_counts = serde_json::Map::new();
    for row in db
        .prepare(&format!(
            "{cte} SELECT code,count(*) FROM diagnostics GROUP BY code ORDER BY code"
        ))?
        .query_map(params_from_iter(filters(read)), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
    {
        let (code, count) = row?;
        warning_counts.insert(code, json!(count));
    }
    let mut arguments = filters(read);
    arguments.push((page.limit as i64 + 1).into());
    arguments.push((paging.offset as i64).into());
    let mut results=db.prepare(&format!("{cte} SELECT source_id,code,json_pointer,message FROM diagnostics ORDER BY CASE code WHEN 'unresolved_link' THEN 0 WHEN 'duplicate_articy_id' THEN 2 ELSE 1 END,source_id,code,json_pointer,message LIMIT ? OFFSET ?"))?
        .query_map(params_from_iter(arguments),|r|Ok(json!({"severity":"warning","source_id":r.get::<_,i64>(0)?,"code":r.get::<_,String>(1)?,"json_pointer":r.get::<_,String>(2)?,"message":r.get::<_,String>(3)?})))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let more = results.len() > page.limit as usize;
    if more {
        results.pop();
    }
    for result in &mut results {
        result["source"] = source(
            &db,
            result["source_id"]
                .as_i64()
                .context("Missing diagnostic source")?,
            result["json_pointer"].as_str(),
            None,
            None,
        )?;
    }
    let fk_errors: i64 =
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    let mut file_checks = Vec::new();
    let mut file_errors = 0;
    if check_files {
        let sources = db
            .prepare(&format!(
                "SELECT s.id,s.path,s.sha256 FROM sources s WHERE {SOURCE_FILTER} ORDER BY s.id"
            ))?
            .query_map(params_from_iter(filters(read)), |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if sources.len() > 200 {
            bail!("File verification is limited to 200 sources; select --source or --pack.");
        }
        for (id, path, expected) in sources {
            let actual = hash_file(std::path::Path::new(&path));
            let ok = actual.as_ref().is_ok_and(|value| value == &expected);
            if !ok {
                file_errors += 1;
            }
            file_checks.push(json!({"source_id":id,"file":path,"matches":ok,"error":actual.err().map(|e|e.to_string())}));
        }
    }
    let valid = fk_errors == 0 && file_errors == 0;
    let response = finish(
        "validate",
        &db,
        &read.output,
        Some(&paging),
        results,
        more,
        json!({"valid":valid,"warning_count":warnings,"warning_counts":warning_counts,
        "foreign_key_errors":fk_errors,"source_file_checks":file_checks,"totals":totals(&db)?,"notes":"Unresolved links are warnings about export coverage, not proof of a game bug. Legacy capture/game versions remain unknown."}),
    )?;
    Ok((response, valid))
}
