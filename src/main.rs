mod htmlgen;

use crate::htmlgen::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::info;
use serde_json::Value;
use simplelog::TermLogger;
use std::{
    fs::{File, copy, create_dir_all},
    io::BufReader,
};

fn main() {
    let log_level = log::LevelFilter::Info;
    let multi = MultiProgress::new();
    let logger = TermLogger::new(
        log_level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    LogWrapper::new(multi.clone(), logger).try_init().unwrap();
    log::set_max_level(log_level);

    info!("Reading file...");
    let file = File::open(
        r#"C:\Users\shawn\MyFiles\suzerainmods\suzerain_data\suzerain_data_dumper\SuzerainDataDumper.conversations_Sordland.json"#,
    ).unwrap();
    let reader = BufReader::new(file);

    info!("Preparing out directory...");
    create_dir_all("out").unwrap();
    copy("assets/script.js", "out/script.js").unwrap();
    copy("assets/style.css", "out/style.css").unwrap();

    info!("Parsing JSON...");
    let v: Value = serde_json::from_reader(reader).unwrap();

    let conversations = &v["conversations"];

    info!("Generating index...");
    generate_index(conversations).unwrap();

    let total = conversations
        .as_object()
        .map(|obj| obj.len().saturating_sub(1) as u64)
        .unwrap_or(0);
    let progress = multi.add(ProgressBar::new(total));
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {msg} {bar:40.cyan/blue} {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    progress.set_message("Writing conversation pages");

    info!("Generating conversation pages...");
    conversations_to_html_files(conversations, &progress).unwrap();
    progress.finish_with_message("Finished writing conversation pages");

    info!("Done. Open out/index.html to browse.");
}
