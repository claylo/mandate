use mandate::{convert_markdown_to_roff, ManpageOptions};

fn options() -> ManpageOptions {
    ManpageOptions::new("mandate", "1", "Test", None, None)
}

#[test]
fn renders_headings_and_inline_styles() {
    let markdown = r#"
# mandate(1) -- Example Tool

## Overview

Paragraph with *em* and **strong** and `code` and <arg>.
"#;
    let roff = convert_markdown_to_roff(markdown, &options()).expect("render roff");
    assert!(roff.contains(".TH \"mandate\" \"1\" \"Test\" \"\" \"\""));
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
}
