use aetheris_daemon::adapters::create_adapter;

#[tokio::test]
async fn create_adapter_instantiates_supported_platforms() {
    let agy = create_adapter("antigravity");
    assert_eq!(agy.platform(), "antigravity");

    let claude = create_adapter("claude");
    assert_eq!(claude.platform(), "claude");

    let direct = create_adapter("direct_api");
    assert_eq!(direct.platform(), "direct_api");
}

#[tokio::test]
async fn adapter_rejects_invalid_mode() {
    let agy = create_adapter("antigravity");
    let res = agy.set_mode("super_danger_mode").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn adapter_accepts_valid_modes() {
    let agy = create_adapter("antigravity");
    assert!(agy.set_mode("safe").await.is_ok());
    assert!(agy.set_mode("yolo").await.is_ok());
    assert!(agy.set_mode("plan").await.is_ok());
}
