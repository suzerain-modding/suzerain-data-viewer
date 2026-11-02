mod cli;
mod htmlgen;
mod import;
mod index;
mod output;
mod query;
mod render;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() {
    match run() {
        Ok(valid) => {
            if !valid {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("Error: {error:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<bool> {
    let Some(command) = Cli::parse().command else {
        render::interactive()?;
        return Ok(true);
    };
    let (response, options, valid) = match command {
        Command::Html {
            input,
            output,
            overwrite,
        } => {
            render::run(&input, &output, overwrite)?;
            return Ok(true);
        }
        Command::Import {
            inputs,
            output,
            replace,
        } => {
            output::check_destination(&output)?;
            (import::run(&inputs, &output, replace)?, output, true)
        }
        Command::Sources { read, page } => {
            output::check_destination(&read.output)?;
            (query::sources(&read, &page)?, read.output, true)
        }
        Command::Search {
            text,
            kind,
            read,
            page,
        } => {
            output::check_destination(&read.output)?;
            (
                query::search(&read, &page, text.as_deref(), kind)?,
                read.output,
                true,
            )
        }
        Command::Node {
            selector,
            read,
            raw,
        } => {
            output::check_destination(&read.output)?;
            (query::node(&read, &selector, raw)?, read.output, true)
        }
        Command::Neighbors {
            selector,
            read,
            page,
            depth,
            direction,
        } => {
            output::check_destination(&read.output)?;
            (
                query::neighbors(&read, &page, &selector, depth, direction)?,
                read.output,
                true,
            )
        }
        Command::References {
            variable,
            language,
            read,
            page,
        } => {
            output::check_destination(&read.output)?;
            (
                query::references(&read, &page, &variable, language)?,
                read.output,
                true,
            )
        }
        Command::Entity {
            name,
            kind,
            raw,
            read,
        } => {
            output::check_destination(&read.output)?;
            (
                query::entity(&read, &name, kind.as_deref(), raw)?,
                read.output,
                true,
            )
        }
        Command::Validate {
            read,
            page,
            check_files,
        } => {
            output::check_destination(&read.output)?;
            let (response, valid) = query::validate(&read, &page, check_files)?;
            (response, read.output, valid)
        }
    };
    output::emit(&response, &options)?;
    Ok(valid)
}
