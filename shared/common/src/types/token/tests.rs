use crate::types::{
    token::{
        access::Access,
        bundle::Bundle,
        refresh::Refresh,
        token::Token,
    },
    DateTime, Error, Id, Phone,
};

#[test]
fn test_token_types_and_deref() {
    let session_id = Id::new();
    let user_id = Id::new();

    // Access Token
    let access_token = Token::access(session_id.clone(), user_id.clone());
    assert_eq!(access_token.session, session_id);
    assert_eq!(access_token.user, user_id);
    assert!(!access_token.is_expired());

    // Refresh Token
    let refresh_token = Token::refresh(session_id.clone(), user_id.clone());
    assert_eq!(refresh_token.session, session_id);
    assert_eq!(refresh_token.user, user_id);
    assert!(!refresh_token.is_expired());

    // Setup Token
    let phone = Phone::new("+254712345678").unwrap();
    let setup_token = Token::setup(phone.clone());
    assert_eq!(setup_token.phone, phone);
    assert!(!setup_token.is_expired());
}

#[test]
fn test_serde_serialization_includes_type_field() {
    let session_id = Id::new();
    let user_id = Id::new();

    let access_token = Token::access(session_id.clone(), user_id.clone());
    let json_str = serde_json::to_string(&access_token).unwrap();

    // Verify type field is injected into JSON
    assert!(json_str.contains("\"type\":\"Access\""));
    assert!(json_str.contains("\"session\""));
    assert!(json_str.contains("\"user\""));
    assert!(json_str.contains("\"expires\""));

    // Verify deserialization back to Token<Access>
    let deserialized: Token<Access> = serde_json::from_str(&json_str).unwrap();
    assert_eq!(access_token, deserialized);
}

#[test]
fn test_serde_deserialization_type_mismatch() {
    let session_id = Id::new();
    let user_id = Id::new();

    let refresh_token = Token::refresh(session_id, user_id);
    let json_str = serde_json::to_string(&refresh_token).unwrap();
    assert!(json_str.contains("\"type\":\"Refresh\""));

    // Attempting to deserialize a Refresh token as Token<Access> must fail
    let result: Result<Token<Access>, _> = serde_json::from_str(&json_str);
    assert!(result.is_err());
}

#[test]
fn test_paseto_encode_decode_and_type_mismatch_error() {
    let session_id = Id::new();
    let user_id = Id::new();

    let refresh_token = Token::refresh(session_id, user_id);
    let paseto_str = refresh_token.encode_paseto().unwrap();

    // Decode correctly as Token<Refresh> without passing key
    let decoded_refresh: Token<Refresh> = Token::decode_paseto(&paseto_str).unwrap();
    assert_eq!(refresh_token, decoded_refresh);

    // Decoding Token<Refresh> PASETO as Token<Access> must return Error::InvalidToken
    let access_result: Result<Token<Access>, Error> = Token::decode_paseto(&paseto_str);
    assert_eq!(access_result, Err(Error::InvalidToken));
}

#[test]
fn test_tokenize_and_direct_string_deserialization() {
    let session_id = Id::new();
    let user_id = Id::new();

    let access_token = Token::access(session_id.clone(), user_id.clone());
    let token_str = access_token.tokenize().unwrap();

    // Deserializing a JSON string containing the PASETO string directly into Token<Access>
    let json_quoted = format!("\"{}\"", token_str);
    let deserialized: Token<Access> = serde_json::from_str(&json_quoted).unwrap();
    assert_eq!(access_token, deserialized);
}

#[test]
fn test_bundle_serialization_and_deserialization() {
    let session_id = Id::new();
    let user_id = Id::new();

    let access_token = Token::access(session_id.clone(), user_id.clone());
    let refresh_token = Token::refresh(session_id.clone(), user_id.clone());
    let bundle = Bundle::new(
        access_token.clone(),
        refresh_token.clone(),
        "https://r2.eduxal.com/profiles/user123.jpg",
    );

    // Serialize bundle to JSON
    let bundle_json = serde_json::to_string(&bundle).unwrap();
    assert!(bundle_json.contains("\"access\":\"v4.local."));
    assert!(bundle_json.contains("\"refresh\":\"v4.local."));
    assert!(bundle_json.contains("\"profile\":\"https://r2.eduxal.com/profiles/user123.jpg\""));

    // Deserialize bundle back from JSON
    let deserialized_bundle: Bundle = serde_json::from_str(&bundle_json).unwrap();
    assert_eq!(bundle, deserialized_bundle);
}

#[test]
fn test_serde_deserialization_expired_token() {
    let session_id = Id::new();
    let user_id = Id::new();
    let past_expires = DateTime::from_timestamp(1000000, 0).unwrap();

    let expired_token = Token {
        claims: Access {
            session: session_id,
            user: user_id,
        },
        expires: past_expires,
    };
    let json_str = serde_json::to_string(&expired_token).unwrap();

    // Deserializing an expired token must return an error
    let result: Result<Token<Access>, _> = serde_json::from_str(&json_str);
    assert!(result.is_err());
}

#[test]
fn test_paseto_expired_token_returns_invalid_token_error() {
    let session_id = Id::new();
    let user_id = Id::new();
    let past_expires = DateTime::from_timestamp(1000000, 0).unwrap();

    let expired_token = Token {
        claims: Access {
            session: session_id,
            user: user_id,
        },
        expires: past_expires,
    };
    let paseto_str = expired_token.encode_paseto().unwrap();

    // Decoding an expired PASETO token must return Err(Error::InvalidToken)
    let decode_result: Result<Token<Access>, Error> = Token::decode_paseto(&paseto_str);
    assert_eq!(decode_result, Err(Error::InvalidToken));
}
