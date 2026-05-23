mod htmlgen;

use crate::htmlgen::*;
use anyhow::{Context, Result, bail};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use inquire::Select;
use log::info;
use serde_json::Value;
use simplelog::TermLogger;
use std::{
    fs::{File, copy, create_dir_all, read_dir},
    io::BufReader,
    time::Instant,
};

fn main() -> Result<()> {
    let multi = MultiProgress::new();
    init_logger(&multi)?;
    let json_files = find_json_files()?;
    let chosen_file = select_json_file(json_files)?;

    info!("Reading '{}'...", chosen_file);
    let json = load_json(&chosen_file)?;
    let root_type = json["_type"]
        .as_str()
        .context("Expected root '_type' field in JSON.")?;

    info!("Root type: {}", root_type);
    prepare_output()?;

    let start = Instant::now();
    route_root(&json, root_type, &multi)?;
    info!(
        "Done in {:.2?}. Open out/index.html to browse.",
        start.elapsed()
    );
    Ok(())
}

fn init_logger(multi: &MultiProgress) -> Result<()> {
    let log_level = log::LevelFilter::Info;
    let logger = TermLogger::new(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    LogWrapper::new(multi.clone(), logger)
        .try_init()
        .context("Failed to initialize logger")?;
    log::set_max_level(log_level);
    Ok(())
}

fn find_json_files() -> Result<Vec<String>> {
    let mut files: Vec<String> = read_dir(".")
        .context("Could not read current directory.")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                path.file_name().and_then(|n| n.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();

    if files.is_empty() {
        bail!("No .json files found in the current directory.");
    }

    files.sort_unstable();
    Ok(files)
}

fn select_json_file(json_files: Vec<String>) -> Result<String> {
    Select::new("Select a JSON file to generate HTML for:", json_files)
        .prompt()
        .context("File selection cancelled.")
}

fn load_json(path: &str) -> Result<Value> {
    let file = File::open(path).with_context(|| format!("Failed to open '{path}'"))?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).context("Failed to parse JSON")
}

fn prepare_output() -> Result<()> {
    create_dir_all("out").context("Failed to create 'out/' directory.")?;
    copy("assets/style.css", "out/style.css").context("Failed to copy assets/style.css")?;
    copy("assets/script.js", "out/script.js").context("Failed to copy assets/script.js")?;
    Ok(())
}

fn route_root(json: &Value, root_type: &str, multi: &MultiProgress) -> Result<()> {
    match root_type {
        "<conversations Sordland>" | "<conversations Rizia>" => {
            generate_conversations(json, root_type, multi)
        }
        "<entity data>" => generate_entity_data(json, root_type, multi),
        other => bail!("Unknown root type: '{other}'"),
    }
}

fn generate_conversations(json: &Value, root_type: &str, multi: &MultiProgress) -> Result<()> {
    let conversations = &json["conversations"];
    if !conversations.is_object() {
        bail!("Expected 'conversations' to be an object.");
    }

    let count = conversations
        .as_object()
        .map(|o| o.len().saturating_sub(1))
        .unwrap_or(0) as u64;

    info!("Generating index...");
    generate_conversations_index(conversations, root_type)
        .context("Failed to generate conversations index.")?;

    let progress = multi.add(make_progress_bar(count));
    progress.set_message(format!("Writing conversation pages ({root_type})"));

    info!("Generating conversation pages...");
    conversations_to_html_files(conversations, &progress)
        .context("Failed to generate conversation pages.")?;

    progress.finish_with_message("Finished writing conversation pages.");
    Ok(())
}

fn generate_entity_data(json: &Value, root_type: &str, multi: &MultiProgress) -> Result<()> {
    info!("Generating entity data pages...");

    let progress = multi.add(make_progress_bar(4));
    progress.set_message("Generating entity data...");

    generate_entity_data_files(json, &progress, root_type)
        .context("Failed to generate entity data pages.")?;

    progress.finish_with_message("Finished writing entity data pages.");
    Ok(())
}

fn make_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {msg} {bar:40.cyan/blue} {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb
}
