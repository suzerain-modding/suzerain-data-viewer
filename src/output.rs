use crate::{
    cli::{Format, Output},
    index::absolute_destination,
};
use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::{self, Write},
};

pub fn encode(value: &Value, format: Format) -> Result<Vec<u8>> {
    let json = serde_json::to_string_pretty(value)?;
    let title = value["command"].as_str().unwrap_or("response");
    Ok(match format {
        Format::Json => format!("{json}\n"),
        Format::Text => format!("Suzerain Data Viewer: {title}\n{json}\n"),
        Format::Markdown => format!("# Suzerain Data Viewer: {title}\n\n```json\n{json}\n```\n"),
    }
    .into_bytes())
}
pub fn check_destination(output: &Output) -> Result<()> {
    if let Some(path) = &output.output {
        if path.exists() {
            bail!(
                "Response output '{}' already exists; choose a new filename.",
                path.display()
            );
        }
        if absolute_destination(path)? == absolute_destination(&output.database)? {
            bail!("The response output and database must be different files.");
        }
    }
    Ok(())
}
pub fn emit(value: &Value, output: &Output) -> Result<()> {
    let bytes = encode(value, output.format)?;
    if bytes.len() as u64 > output.max_bytes {
        bail!("Response exceeds --max-bytes {}.", output.max_bytes);
    }
    if let Some(path) = &output.output {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&bytes)?;
    } else {
        io::stdout().lock().write_all(&bytes)?;
    }
    Ok(())
}
