#![forbid(unsafe_code)]

use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "mandate", version)]
struct Cli {
    #[arg(short = 'i', long = "input", value_name = "PATH", default_value = "-")]
    input: String,

    #[arg(short = 'p', long = "program", value_name = "NAME")]
    program: String,

    #[arg(
        short = 's',
        long = "section",
        value_name = "SECTION",
        default_value = "1"
    )]
    section: String,

    #[arg(short = 't', long = "title", value_name = "TITLE")]
    title: String,

    #[arg(short = 'm', long = "manual-section", value_name = "MANUAL")]
    manual_section: Option<String>,

    #[arg(long = "source", value_name = "SOURCE")]
    source: Option<String>,

    #[arg(short = 'o', long = "output", value_name = "PATH")]
    output: Option<PathBuf>,

    #[arg(long = "validate")]
    validate: bool,

    #[arg(long = "schema", value_name = "PATH")]
    schema: Option<PathBuf>,
}

fn read_input(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

fn write_output(path: Option<PathBuf>, contents: &str) -> io::Result<()> {
    match path {
        Some(path) => fs::write(path, contents),
        None => {
            print!("{contents}");
            Ok(())
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let input = read_input(&cli.input)?;

    let options = mandate::ManpageOptions::new(
        cli.program,
        cli.section,
        cli.title,
        cli.manual_section,
        cli.source,
    );

    let output = match input_kind(&cli.input) {
        InputKind::Yaml => {
            if cli.validate {
                validate_yaml(&input, cli.schema.as_ref())?;
            }
            mandate::convert_yaml_to_roff(&input, &options)?
        }
        InputKind::Markdown => mandate::convert_markdown_to_roff(&input, &options)?,
        InputKind::Auto => {
            if cli.validate {
                match validate_yaml(&input, cli.schema.as_ref()) {
                    Ok(()) => mandate::convert_yaml_to_roff(&input, &options)?,
                    Err(mandate::MandateError::Yaml(_)) => {
                        mandate::convert_markdown_to_roff(&input, &options)?
                    }
                    Err(err) => return Err(Box::new(err)),
                }
            } else {
                mandate::convert_yaml_to_roff(&input, &options)
                    .or_else(|_| mandate::convert_markdown_to_roff(&input, &options))?
            }
        }
    };
    write_output(cli.output, &output)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum InputKind {
    Yaml,
    Markdown,
    Auto,
}

fn input_kind(path: &str) -> InputKind {
    let lower = path.to_ascii_lowercase();
    if lower == "-" {
        return InputKind::Auto;
    }
    if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        return InputKind::Yaml;
    }
    if lower.ends_with(".md") || lower.ends_with(".markdown") {
        return InputKind::Markdown;
    }
    InputKind::Markdown
}

fn validate_yaml(input: &str, schema: Option<&PathBuf>) -> Result<(), mandate::MandateError> {
    match schema {
        Some(path) => mandate::validate_yaml_with_schema(input, path),
        None => mandate::validate_yaml_with_schema_str(input, mandate::BUILTIN_SCHEMA),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_kind_detects_extensions() {
        assert!(matches!(input_kind("-"), InputKind::Auto));
        assert!(matches!(input_kind("manual.yaml"), InputKind::Yaml));
        assert!(matches!(input_kind("manual.yml"), InputKind::Yaml));
        assert!(matches!(input_kind("manual.md"), InputKind::Markdown));
        assert!(matches!(input_kind("manual.markdown"), InputKind::Markdown));
        assert!(matches!(input_kind("manual.txt"), InputKind::Markdown));
    }
}
