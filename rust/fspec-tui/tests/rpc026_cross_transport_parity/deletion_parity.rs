/// Scenario: persistence_delete_session round-trips identically across both transports
#[tokio::test]
async fn embedded_and_websocket_delete_session_byte_identical() {
    use deletion_fixture::{DeletionFixture, SESSION_IDS};

    // @step Given a SharedFspecService with sessions ["s-1", "s-2", "s-3"]
    let (_guard, temp) = setup_parity_temp_dir();
    let workspace = temp.path().join("workspace");
    workspace_with_seed(&workspace);
    let embedded_fixture = DeletionFixture::new(&temp.path().join("embedded"), &workspace);
    let embedded = EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        Arc::clone(&embedded_fixture.service),
    );
    let embedded_before = embedded
        .list_sessions(String::new())
        .await
        .expect("embedded initial list");
    assert_eq!(embedded_before, embedded_fixture.expected);
    assert!(embedded_fixture
        .sessions_dir
        .join(format!("{}.json", SESSION_IDS[1]))
        .exists());

    // @step When the test calls EmbeddedFspecBackend.persistence_delete_session("s-2")
    embedded
        .persistence_delete_session(SessionId::new(SESSION_IDS[1]))
        .await
        .expect("embedded delete");
    // @step And then calls EmbeddedFspecBackend.list_sessions()
    let embedded_after = embedded
        .list_sessions(String::new())
        .await
        .expect("embedded remaining list");
    // @step Then the result equals ["s-1", "s-3"]
    assert_eq!(
        embedded_after
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        [SESSION_IDS[0], SESSION_IDS[2]],
    );
    assert_eq!(
        embedded_after,
        [embedded_before[0].clone(), embedded_before[2].clone()]
    );
    embedded_fixture.assert_middle_deleted();
    let embedded_manifest_bytes = std::fs::read(
        embedded_fixture
            .sessions_dir
            .join(format!("{}.json", SESSION_IDS[0])),
    )
    .expect("read embedded survivor");
    drop(embedded);
    drop(embedded_fixture);

    // @step Given another SharedFspecService with sessions ["s-1", "s-2", "s-3"]
    let websocket_fixture = DeletionFixture::new(&temp.path().join("websocket"), &workspace);
    let (addr, _stats, server) =
        bind_and_serve("127.0.0.1:0", Arc::clone(&websocket_fixture.service))
            .await
            .expect("independent websocket service");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket = WebSocketFspecBackend::connect(url).await.expect("connect");
    let websocket_before = websocket
        .list_sessions(String::new())
        .await
        .expect("websocket initial list");
    assert_eq!(websocket_before, websocket_fixture.expected);
    assert_eq!(
        serde_json::to_vec(&embedded_before).expect("serialize embedded seed"),
        serde_json::to_vec(&websocket_before).expect("serialize websocket seed")
    );
    assert_eq!(
        embedded_manifest_bytes,
        std::fs::read(
            websocket_fixture
                .sessions_dir
                .join(format!("{}.json", SESSION_IDS[0])),
        )
        .expect("read independently seeded survivor")
    );
    assert!(websocket_fixture
        .sessions_dir
        .join(format!("{}.json", SESSION_IDS[1]))
        .exists());

    // @step When the test calls WebSocketFspecBackend.persistence_delete_session("s-2")
    websocket
        .persistence_delete_session(SessionId::new(SESSION_IDS[1]))
        .await
        .expect("websocket delete");
    // @step And then calls WebSocketFspecBackend.list_sessions()
    let websocket_after = websocket
        .list_sessions(String::new())
        .await
        .expect("websocket remaining list");
    // @step Then the result equals ["s-1", "s-3"]
    assert_eq!(
        websocket_after
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        [SESSION_IDS[0], SESSION_IDS[2]],
    );
    assert_eq!(
        websocket_after,
        [websocket_before[0].clone(), websocket_before[2].clone()]
    );
    websocket_fixture.assert_middle_deleted();
    // @step And both transports produced byte-identical SessionInfo lists
    assert_eq!(
        serde_json::to_vec(&embedded_after).expect("serialize embedded remaining list"),
        serde_json::to_vec(&websocket_after).expect("serialize websocket remaining list")
    );
    assert!(!temp
        .path()
        .join("embedded/sessions")
        .join(format!("{}.json", SESSION_IDS[1]))
        .exists());
    server.abort();
}
