use mandate::convert_yaml_to_markdown;

#[test]
fn converts_yaml_manual_to_markdown() {
    let yaml = include_str!("fixtures/manual.yml");
    let markdown = convert_yaml_to_markdown(yaml).expect("convert yaml");
    assert!(markdown.contains("## SYNOPSIS"));
    assert!(markdown.contains("## FILTERS"));
    assert!(markdown.contains("### "));
    assert!(markdown.contains("~~~~"));
    assert!(markdown.contains("=> "));
    assert!(markdown.contains("jq '"));
}
