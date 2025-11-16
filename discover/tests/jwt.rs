use discover::jwt::*;
use regex_lite::Regex;

const ECDSA_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg5lTeXefIw7PeO809
pxg9THzXGN5PToqXGXzhCcTpGbKhRANCAAQt5X5osNzNIxexaywne36MaSFO2Lok
Vk6DwMW41i7/Hr0DjGlvBRmCCf0DcsyDyK14OAXltdwX5rYWSkGq8wev
-----END PRIVATE KEY-----"#;

const ECDSA_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAELeV+aLDczSMXsWssJ3t+jGkhTti6
JFZOg8DFuNYu/x69A4xpbwUZggn9A3LMg8iteDgF5bXcF+a2FkpBqvMHrw==
-----END PUBLIC KEY-----"#;

#[test]
fn assert_create_token() {
    let manager = TokenManager::new(
        Some(Key::Text::<String>(ECDSA_PRIVATE_KEY.to_string())),
        Key::Text::<String>(ECDSA_PUBLIC_KEY.to_string()),
    )
    .unwrap();

    let claims = Claims::new("user1".into());

    let token = manager.create_token(&claims).unwrap();

    assert!(token.starts_with("eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9."));
    assert!(
        Regex::new(r"^[A-Za-z0-9_-]{2,}(?:\.[A-Za-z0-9_-]{2,}){2}$")
            .unwrap()
            .captures(&token)
            .is_some()
    );
}
