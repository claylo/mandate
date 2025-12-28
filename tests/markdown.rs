use mandate::{ManpageOptions, convert_markdown_to_roff};

fn options() -> ManpageOptions {
    ManpageOptions::new("mandate", "1", "Test", None, None)
}

fn th_fields(roff: &str) -> Vec<String> {
    roff.lines()
        .find(|line| line.starts_with(".TH "))
        .map(|line| {
            line.split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[test]
fn renders_headings_and_inline_styles() {
    let markdown = r#"
# mandate(1) -- Example Tool

## Overview

Paragraph with *em* and **strong** and `code` and <arg>.
"#;
    let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
    let fields = th_fields(&roff);
    assert!(fields.len() >= 5);
    assert_eq!(fields[0], "mandate");
    assert_eq!(fields[1], "1");
    assert!(!fields[2].is_empty());
    assert_eq!(fields[4], "Test");
    assert!(roff.contains(".SH \"NAME\""));
    assert!(roff.contains("\\fBmandate\\fR \\- Example Tool"));
    assert!(roff.contains("\\fIem\\fR"));
    assert!(roff.contains("\\fBstrong\\fR"));
    assert!(roff.contains("\\fBcode\\fR"));
    assert!(roff.contains("\\fIarg\\fR"));
}

#[test]
fn special_list_consumes_following_paragraphs() {
    let markdown = r#"
## OPTIONS

- Foo:

Paragraph after foo.
"#;
    let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
    assert!(roff.contains(".TP"));
    assert!(roff.contains("Foo:"));
    assert!(roff.contains(".IP"));
    assert!(roff.contains("Paragraph after foo\\."));
    assert!(roff.contains("Foo:\n.IP"));
}
