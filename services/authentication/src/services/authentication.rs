use crate::services::Authenticator;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_sqs::types::QueueAttributeName;
use chrono::Utc;
use common::types::{
    Bundle, DateTime, Error, Id, Phone, Refresh, Session, SessionStatus, Setup, Status, Token,
    User, Verification,
};
use rand::Rng;
use serde_json::json;
use std::collections::HashMap;

impl Authenticator {
    pub async fn login(&self, phone: Phone) -> Result<Verification, Error> {
        let db = self.config.db();
        let sqs = self.config.sqs();
        let queue_url = self.config.messenger_queue_url();

        // 1. SQS Queue Backpressure Check (HTTP 429 if pending >= 20,000)
        let attr_out = sqs
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
            .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
            .send()
            .await;

        if let Ok(out) = attr_out {
            if let Some(attrs) = out.attributes {
                let visible: u64 = attrs
                    .get(&QueueAttributeName::ApproximateNumberOfMessages)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let not_visible: u64 = attrs
                    .get(&QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);

                if visible + not_visible >= 20_000 {
                    return Err(Error::SlowDown);
                }
            }
        }

        // 2. DynamoDB Rate Limit Check (RLTS 60s)
        let existing_item = db
            .get_item()
            .table_name("eduxal-verifications")
            .key("phone", AttributeValue::S(phone.as_ref().to_string()))
            .send()
            .await;

        if let Ok(out) = existing_item {
            if let Some(item) = out.item {
                if let Ok(verification) = Verification::try_from(&item) {
                    if Utc::now().timestamp() < (verification.created.timestamp() + 60) {
                        return Err(Error::SlowDown);
                    }
                }
            }
        }

        // 3. Create Verification record
        let code: u32 = rand::thread_rng().gen_range(100000..999999);
        let created = DateTime::now();
        let ttl = DateTime::from_timestamp(created.timestamp() + 300, 0).unwrap_or(created);
        let verification = Verification::new(phone.clone(), code.to_string(), created, ttl);

        let item: HashMap<String, AttributeValue> = (&verification).into();
        db.put_item()
            .table_name("eduxal-verifications")
            .set_item(Some(item))
            .send()
            .await
            .map_err(|_| Error::InvalidToken)?;

        // 4. Enqueue ONLY { "phone": "+254..." } to SQS MessengerQueue
        let body = json!({ "phone": phone.as_ref() }).to_string();
        let _ = sqs
            .send_message()
            .queue_url(queue_url)
            .message_body(body)
            .send()
            .await;

        // 5. Return Verification (Serde automatically skips `code`)
        Ok(verification)
    }

    pub async fn verify(&self, phone: Phone, code: &str) -> Result<(Token<Setup>, String), Error> {
        let db = self.config.db();

        // 1. Query Verification from DynamoDB
        let get_out = db
            .get_item()
            .table_name("eduxal-verifications")
            .key("phone", AttributeValue::S(phone.as_ref().to_string()))
            .send()
            .await
            .map_err(|_| Error::InvalidToken)?;

        let item = get_out.item.ok_or(Error::InvalidToken)?;
        let verification = Verification::try_from(&item)?;

        if verification.code != code || verification.ttl.timestamp() <= Utc::now().timestamp() {
            return Err(Error::InvalidToken);
        }

        // 2. Delete Verification from DynamoDB
        let _ = db
            .delete_item()
            .table_name("eduxal-verifications")
            .key("phone", AttributeValue::S(phone.as_ref().to_string()))
            .send()
            .await;

        // 3. Generate Token<Setup> & R2 upload_url
        let setup_token = Token::setup(phone.clone());
        let upload_url = format!(
            "https://r2.eduxal.com/upload/profile_{}.jpg?signature=presigned_upload_url",
            phone.as_ref()
        );

        Ok((setup_token, upload_url))
    }

    pub async fn setup(
        &self,
        token_setup_str: &str,
        name: String,
        device: String,
    ) -> Result<Bundle, Error> {
        let db = self.config.db();

        // 1. Decrypt & validate Token<Setup>
        let setup_token = Token::<Setup>::decode_paseto(token_setup_str)?;
        let phone = setup_token.phone.clone();

        // 2. Create User in DynamoDB eduxal-users
        let user_id = Id::new();
        let user = User::new(
            user_id.clone(),
            phone.clone(),
            name,
            Status::Active,
            DateTime::now(),
        );

        let user_item: HashMap<String, AttributeValue> = (&user).into();
        let _ = db
            .put_item()
            .table_name("eduxal-users")
            .set_item(Some(user_item))
            .send()
            .await;

        // 3. Create Session in DynamoDB eduxal-sessions
        let session = Session::new(user_id.clone(), device);
        let session_item: HashMap<String, AttributeValue> = (&session).into();
        let _ = db
            .put_item()
            .table_name("eduxal-sessions")
            .set_item(Some(session_item))
            .send()
            .await;

        // 4. Generate Tokens and profile URL
        let access = Token::access(session.id.clone(), user_id.clone());
        let refresh = Token::refresh(session.id.clone(), user_id.clone());
        let profile_url = format!("https://r2.eduxal.com/profiles/{}.jpg", user_id.to_hex());

        Ok(Bundle::new(access, refresh, profile_url))
    }

    pub async fn refresh(&self, refresh_token_str: &str) -> Result<Bundle, Error> {
        let db = self.config.db();

        // 1. Decrypt & validate Token<Refresh>
        let refresh_token = Token::<Refresh>::decode_paseto(refresh_token_str)?;
        let session_id = refresh_token.session.clone();
        let user_id = refresh_token.user.clone();

        // 2. Fetch Session from eduxal-sessions DynamoDB
        let session_out = db
            .get_item()
            .table_name("eduxal-sessions")
            .key("user", AttributeValue::S(user_id.to_hex()))
            .key("id", AttributeValue::S(session_id.to_hex()))
            .send()
            .await
            .map_err(|_| Error::InvalidToken)?;

        let item = session_out.item.ok_or(Error::InvalidToken)?;
        let session = Session::try_from(&item)?;

        if session.status != SessionStatus::Active || session.is_expired() {
            return Err(Error::InvalidToken);
        }

        // 3. Generate fresh tokens & profile URL
        let new_access = Token::access(session.id.clone(), user_id.clone());
        let new_refresh = Token::refresh(session.id.clone(), user_id.clone());
        let profile_url = format!("https://r2.eduxal.com/profiles/{}.jpg", user_id.to_hex());

        Ok(Bundle::new(new_access, new_refresh, profile_url))
    }
}
