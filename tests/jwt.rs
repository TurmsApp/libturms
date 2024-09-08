use libturms::jwt::*;
use regex_lite::Regex;

#[test]
fn assert_create_token() {
    let manager = TokenManager::new(
        Some(Key::Path("./tests/private.key")),
        Key::Path("./tests/key.pub"),
    )
    .unwrap();

    let claims = Claims::new("user1".into());

    let token = manager.create_token(&claims).unwrap();

    assert!(token.starts_with("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9."));
    assert!(Regex::new(r"^[A-Za-z0-9_-]{2,}(?:\.[A-Za-z0-9_-]{2,}){2}$")
        .unwrap()
        .captures(&token)
        .is_some());
}
