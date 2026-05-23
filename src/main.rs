mod htmlgen;

use crate::htmlgen::conversations_to_html_files;
use log::{LevelFilter, info};
use serde_json::Value;
use simplelog::SimpleLogger;
use std::{
    fs::{File, copy, create_dir},
    io::BufReader,
};

fn main() {
    SimpleLogger::init(LevelFilter::Info, simplelog::Config::default()).unwrap();

    info!("Reading file...");
    let file = File::open(
        r#"C:\Users\shawn\MyFiles\suzerainmods\suzerain_data\suzerain_data_dumper\SuzerainDataDumper.conversations_Sordland.json"#,
    ).unwrap();
    let reader = BufReader::new(file);

    info!("Preparing out directory...");
    create_dir("out").unwrap();
    copy("assets/script.js", "out/script.js").unwrap();
    copy("assets/style.css", "out/style.css").unwrap();

    info!("Parsing JSON...");
    let v: Value = serde_json::from_reader(reader).unwrap();

    info!("Generating HTML...");
    conversations_to_html_files(&v["conversations"]).unwrap();

    info!("Done.");
}
