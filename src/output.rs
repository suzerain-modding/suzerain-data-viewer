use crate::cli::{Format, Output};
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
        let absolute = |p: &std::path::Path| -> Result<std::path::PathBuf> {
            if p.exists() {
                return Ok(p.canonicalize()?);
            }
            let parent = p
                .parent()
                .filter(|v| !v.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            Ok(parent.canonicalize()?.join(
                p.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Output must name a file"))?,
            ))
        };
        if absolute(path)? == absolute(&output.database)? {
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
