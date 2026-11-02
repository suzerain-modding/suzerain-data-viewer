use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, Row, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const APPLICATION_ID: i64 = 0x53445631;
pub const INDEX_VERSION: i64 = 1;
pub const RECORD_COLUMNS: &str = "r.id,r.source_id,r.kind,r.pack,r.conversation_id,r.entry_id,r.name,r.title,r.articy_id,r.text,r.json_pointer,r.raw_json,r.dialogue_title";

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS sources(
 id INTEGER PRIMARY KEY,path TEXT NOT NULL,sha256 TEXT NOT NULL,root_type TEXT NOT NULL,
 pack TEXT,bytes INTEGER NOT NULL,imported_unix TEXT NOT NULL,html_dir TEXT NOT NULL,
 UNIQUE(path,sha256));
CREATE TABLE IF NOT EXISTS records(
 id INTEGER PRIMARY KEY,source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
 kind TEXT NOT NULL,pack TEXT,conversation_id INTEGER,entry_id INTEGER,name TEXT NOT NULL,
 title TEXT NOT NULL,articy_id TEXT,text TEXT NOT NULL,scripts TEXT NOT NULL,
 json_pointer TEXT NOT NULL,raw_json TEXT NOT NULL,dialogue_title TEXT,
 UNIQUE(source_id,json_pointer));
CREATE UNIQUE INDEX IF NOT EXISTS node_identity ON records(source_id,conversation_id,entry_id) WHERE kind='node';
CREATE UNIQUE INDEX IF NOT EXISTS conversation_identity ON records(source_id,conversation_id) WHERE kind='conversation';
CREATE INDEX IF NOT EXISTS articy_lookup ON records(articy_id COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS entity_lookup ON records(name,kind);
CREATE INDEX IF NOT EXISTS dialogue_lookup ON records(dialogue_title);
CREATE TABLE IF NOT EXISTS record_packs(record_id INTEGER REFERENCES records(id) ON DELETE CASCADE,pack TEXT,PRIMARY KEY(record_id,pack));
CREATE TABLE IF NOT EXISTS fields(record_id INTEGER REFERENCES records(id) ON DELETE CASCADE,ordinal TEXT,title TEXT,field_type TEXT,value TEXT);
CREATE INDEX IF NOT EXISTS fields_record ON fields(record_id);
CREATE TABLE IF NOT EXISTS edges(
 id INTEGER PRIMARY KEY,source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
 origin_record_id INTEGER NOT NULL REFERENCES records(id) ON DELETE CASCADE,
 destination_conversation_id INTEGER NOT NULL,destination_entry_id INTEGER NOT NULL,
 priority TEXT NOT NULL,is_connector INTEGER NOT NULL,json_pointer TEXT NOT NULL,raw_json TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS edges_origin ON edges(origin_record_id);
CREATE INDEX IF NOT EXISTS edges_destination ON edges(source_id,destination_conversation_id,destination_entry_id);
CREATE TABLE IF NOT EXISTS scripts(
 id INTEGER PRIMARY KEY,record_id INTEGER NOT NULL REFERENCES records(id) ON DELETE CASCADE,
 language TEXT NOT NULL,field TEXT NOT NULL,json_pointer TEXT NOT NULL,text TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS scripts_record ON scripts(record_id);
CREATE TABLE IF NOT EXISTS schedule(
 source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
 flow_record_id INTEGER NOT NULL REFERENCES records(id) ON DELETE CASCADE,
 story_pack TEXT NOT NULL,turn_index TEXT NOT NULL,step_index TEXT NOT NULL,fragment TEXT NOT NULL,json_pointer TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS schedule_fragment ON schedule(source_id,fragment);
CREATE TABLE IF NOT EXISTS issues(
 id INTEGER PRIMARY KEY,source_id INTEGER REFERENCES sources(id) ON DELETE CASCADE,
 code TEXT NOT NULL,json_pointer TEXT NOT NULL,message TEXT NOT NULL);
CREATE VIRTUAL TABLE IF NOT EXISTS record_fts USING fts5(title,text,scripts);
"#;

pub fn open(path: &Path, writable: bool) -> Result<Connection> {
    let flags = if writable {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let db = Connection::open_with_flags(path, flags)
        .with_context(|| format!("Cannot open index '{}'. Run import first.", path.display()))?;
    db.busy_timeout(std::time::Duration::from_secs(10))?;
    let app: i64 = db.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let tables: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table'",
        [],
        |r| r.get(0),
    )?;
    if app != APPLICATION_ID && !(writable && app == 0 && tables == 0) {
        bail!(
            "'{}' is not a Data Viewer index; it will not be overwritten.",
            path.display()
        );
    }
    if app == APPLICATION_ID && version != INDEX_VERSION {
        bail!("Unsupported index version {version}; import into a new database path.");
    }
    db.pragma_update(None, "foreign_keys", true)?;
    if !writable {
        db.execute_batch("BEGIN DEFERRED")?;
    }
    Ok(db)
}

pub fn read_json(path: &Path) -> Result<Value> {
    let mut reader = BufReader::new(File::open(path)?);
    if reader.fill_buf()?.starts_with(b"\xef\xbb\xbf") {
        reader.consume(3);
    }
    serde_json::from_reader(reader).with_context(|| format!("Invalid JSON in '{}'", path.display()))
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
pub fn stamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string()
}
pub fn path_string(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(tail) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{tail}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
    }
}
pub fn pointer_part(text: &str) -> String {
    text.replace('~', "~0").replace('/', "~1")
}

pub fn members(value: &Value) -> Result<Vec<(&str, &Value)>> {
    let object = value
        .as_object()
        .context("Expected a legacy object-keyed collection")?;
    let mut entries: Vec<_> = object
        .iter()
        .filter(|(k, _)| k.as_str() != "_type")
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    entries.sort_by(|a, b| match (a.0.parse::<u64>(), b.0.parse::<u64>()) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        _ => a.0.cmp(b.0),
    });
    Ok(entries)
}
pub fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .with_context(|| format!("Expected string '{key}'"))
}
pub fn required_int(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .with_context(|| format!("Expected integer '{key}'"))
}
pub fn pack_name(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "storypack_main" | "sordland" => "Sordland".into(),
        "storypack_rizia" | "rizia" => "Rizia".into(),
        _ => value.to_string(),
    }
}

#[derive(Debug)]
pub struct Record {
    pub id: i64,
    pub source_id: i64,
    pub kind: String,
    pub pack: Option<String>,
    pub conversation_id: Option<i64>,
    pub entry_id: Option<i64>,
    pub name: String,
    pub title: String,
    pub articy_id: Option<String>,
    pub text: String,
    pub pointer: String,
    pub raw: String,
    pub dialogue: Option<String>,
}
impl Record {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            source_id: row.get(1)?,
            kind: row.get(2)?,
            pack: row.get(3)?,
            conversation_id: row.get(4)?,
            entry_id: row.get(5)?,
            name: row.get(6)?,
            title: row.get(7)?,
            articy_id: row.get(8)?,
            text: row.get(9)?,
            pointer: row.get(10)?,
            raw: row.get(11)?,
            dialogue: row.get(12)?,
        })
    }
    pub fn source(&self, db: &Connection) -> Result<Value> {
        source(
            db,
            self.source_id,
            Some(&self.pointer),
            self.conversation_id,
            self.entry_id,
        )
    }
    pub fn summary(&self, db: &Connection) -> Result<Value> {
        let (excerpt, truncated) = excerpt(&self.text, 400);
        let conversation_title: Option<String> = if self.kind == "node" {
            Some(db.query_row("SELECT name FROM records WHERE source_id=? AND kind='conversation' AND conversation_id=?",
                params![self.source_id,self.conversation_id],|r|r.get(0))?)
        } else if self.kind == "conversation" {
            Some(self.name.clone())
        } else {
            None
        };
        Ok(
            json!({"record_id":self.id,"kind":self.kind,"pack":self.pack,"name":self.name,"title":self.title,
            "conversation_id":self.conversation_id,"entry_id":self.entry_id,"articy_id":self.articy_id,
            "conversation_title":conversation_title,
            "text_excerpt":excerpt,"excerpt_truncated":truncated,"source":self.source(db)?}),
        )
    }
}
pub fn excerpt(text: &str, limit: usize) -> (String, bool) {
    let mut chars = text.chars();
    let result: String = chars.by_ref().take(limit).collect();
    (result, chars.next().is_some())
}

pub fn source(
    db: &Connection,
    id: i64,
    pointer: Option<&str>,
    conversation: Option<i64>,
    entry: Option<i64>,
) -> Result<Value> {
    let mut result=db.query_row("SELECT path,sha256,root_type,pack,bytes,imported_unix,html_dir FROM sources WHERE id=?",[id],|row| {
        Ok(json!({"id":id,"file":row.get::<_,String>(0)?,"sha256":row.get::<_,String>(1)?,"root_type":row.get::<_,String>(2)?,
            "pack":row.get::<_,Option<String>>(3)?,"bytes":row.get::<_,i64>(4)?,"imported_unix_ns":row.get::<_,String>(5)?,
            "html_directory":row.get::<_,String>(6)?,"game_version":Value::Null,"schema_version":Value::Null,"capture_time":Value::Null}))
    })?;
    if let Some(pointer) = pointer {
        result["json_pointer"] = json!(pointer);
    }
    if let Some(conversation) = conversation {
        let directory = result["html_directory"].as_str().unwrap_or_default();
        let mut html = format!("{directory}/conversation{conversation}.html");
        if let Some(entry) = entry {
            html.push_str(&format!("#DialogueEntry-{entry}"));
        }
        result["html"] = json!(html);
    } else if let Some(pointer) = pointer {
        let category = pointer.split('/').nth(1).unwrap_or_default();
        if matches!(
            category,
            "AllBillsData"
                | "AllConversationsData"
                | "AllDecisionsData"
                | "conditionalInstructionData"
                | "GameFlowData"
                | "NewsData"
        ) {
            result["html"] = json!(format!(
                "{}/entity_{}.html",
                result["html_directory"].as_str().unwrap_or_default(),
                category.to_lowercase()
            ));
        }
    }
    if let Some(html) = result["html"].as_str() {
        result["html_exists"] = json!(Path::new(html.split('#').next().unwrap_or(html)).is_file());
        result["html_location_basis"] = json!(
            "Conventional sibling directory named after the input file stem; not capture metadata."
        );
    }
    Ok(result)
}

pub fn revision(db: &Connection) -> Result<String> {
    db.query_row("SELECT value FROM meta WHERE key='revision'", [], |r| {
        r.get(0)
    })
    .context("Index is empty; run import first")
}
pub fn warn(db: &Connection, source: i64, code: &str, pointer: &str, message: &str) -> Result<()> {
    db.prepare_cached("INSERT INTO issues(source_id,code,json_pointer,message) VALUES(?,?,?,?)")?
        .execute(params![source, code, pointer, message])?;
    Ok(())
}
