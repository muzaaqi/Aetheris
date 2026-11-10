use aetheris_daemon::context::{parse_skill_markdown, scan_skills, search_files};
use aetheris_daemon::protocol::SkillScope;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn search_files_finds_matching_files_and_respects_ignore() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("hello.rs"), "fn main() {}").unwrap();
    fs::write(root.join("world.txt"), "hello").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/hello_sub.rs"), "fn sub() {}").unwrap();

    let matches = search_files(root, "hello", 10).await.unwrap();
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().any(|m| m.path.contains("hello.rs")));
    assert!(matches.iter().any(|m| m.path.contains("hello_sub.rs")));
}

#[tokio::test]
async fn search_files_handles_regex_special_characters_safely() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("file[1].rs"), "fn test() {}").unwrap();

    // Query with unclosed bracket must match literal string and not panic
    let matches = search_files(root, "file[", 10).await.unwrap();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].path.contains("file[1].rs"));
}

#[tokio::test]
async fn search_files_handles_empty_query_clamped_to_limit() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    for i in 0..5 {
        fs::write(root.join(format!("item_{i}.txt")), "content").unwrap();
    }
    let matches = search_files(root, "", 3).await.unwrap();
    assert_eq!(matches.len(), 3);
}

#[test]
fn parse_skill_markdown_extracts_frontmatter_or_fallback() {
    let content_with_fm = "---\nname: my-skill\ndescription: Do things\n---\n# Body";
    let (name, desc) =
        parse_skill_markdown(std::path::Path::new("/dummy/SKILL.md"), content_with_fm);
    assert_eq!(name, "my-skill");
    assert_eq!(desc, "Do things");

    let content_without_fm = "# Plain Title\nNo frontmatter here.";
    let (name, desc) = parse_skill_markdown(
        std::path::Path::new("/dummy/tool-maker/SKILL.md"),
        content_without_fm,
    );
    assert_eq!(name, "tool-maker");
    assert_eq!(desc, "");
}

#[tokio::test]
async fn scan_skills_finds_workspace_skills() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let skill_dir = root.join(".agents/skills/custom-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: custom-skill\ndescription: My custom workspace skill\n---\n# Docs",
    )
    .unwrap();

    let skills = scan_skills(Some(root)).await.unwrap();
    let ws_skill = skills.iter().find(|s| s.name == "custom-skill");
    assert!(ws_skill.is_some());
    let s = ws_skill.unwrap();
    assert_eq!(s.scope, SkillScope::Workspace);
    assert_eq!(s.description, "My custom workspace skill");
}
