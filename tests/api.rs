#[tokio::test]
async fn test_health_check() {
    let client = reqwest::Client::new();
    let res = client
        .get("http://localhost:31415/players")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}
