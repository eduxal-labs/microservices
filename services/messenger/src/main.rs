use aws_lambda_events::event::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoClient;
use common::types::{Error, Verification};
use lambda_runtime::{run, service_fn, LambdaEvent};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

const META_URL: &str = "https://graph.facebook.com/v21.0/960426547146856/messages";

#[derive(Debug, Deserialize)]
struct PhonePayload {
    phone: String,
}

pub struct Messenger {
    token: &'static str,
    client: Client,
    db: DynamoClient,
}

impl Messenger {
    pub async fn new() -> Self {
        let token = env!("WHATSAPP_TOKEN");
        let client = Client::builder()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .pool_max_idle_per_host(80)
            .build()
            .unwrap_or_else(|_| Client::new());

        let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let db = DynamoClient::new(&sdk_config);

        Self { token, client, db }
    }

    fn body(recipient: &str, code: &str) -> serde_json::Value {
        json!({
            "messaging_product": "whatsapp",
            "to": recipient,
            "type": "template",
            "template": {
                "name": "auth_code",
                "language": {"code": "en"},
                "components": [
                    {
                        "type": "body",
                        "parameters": [{"type": "text", "text": code}]
                    },
                    {
                        "type": "button",
                        "sub_type": "url",
                        "index": 0,
                        "parameters": [{"type": "text", "text": code}]
                    }
                ]
            }
        })
    }

    pub async fn send_whatsapp(&self, recipient: &str, code: &str) -> Result<(), Error> {
        let json = Self::body(recipient, code);
        self.client
            .post(META_URL)
            .bearer_auth(self.token)
            .json(&json)
            .send()
            .await
            .map_err(|_| Error::InvalidToken)?
            .error_for_status()
            .map_err(|_| Error::InvalidToken)?;
        Ok(())
    }
}

async fn function_handler(
    event: LambdaEvent<SqsEvent>,
    messenger: &Arc<Messenger>,
) -> Result<SqsBatchResponse, lambda_runtime::Error> {
    let records = event.payload.records;
    if records.is_empty() {
        return Ok(SqsBatchResponse {
            batch_item_failures: vec![],
        });
    }

    // Map phone -> message_id
    let mut phone_to_msg_id: HashMap<String, String> = HashMap::new();
    for record in &records {
        if let Some(body) = &record.body {
            if let Ok(payload) = serde_json::from_str::<PhonePayload>(body) {
                phone_to_msg_id.insert(payload.phone, record.message_id.clone().unwrap_or_default());
            }
        }
    }

    if phone_to_msg_id.is_empty() {
        return Ok(SqsBatchResponse {
            batch_item_failures: vec![],
        });
    }

    // 1. Bulk read from DynamoDB eduxal-verifications table via BatchGetItem
    let keys: Vec<HashMap<String, AttributeValue>> = phone_to_msg_id
        .keys()
        .map(|phone| {
            let mut map = HashMap::new();
            map.insert("phone".to_string(), AttributeValue::S(phone.clone()));
            map
        })
        .collect();

    let keys_and_attrs = aws_sdk_dynamodb::types::KeysAndAttributes::builder()
        .set_keys(Some(keys))
        .build()
        .map_err(|_| Error::InvalidToken)?;

    let request_items = HashMap::from([("eduxal-verifications".to_string(), keys_and_attrs)]);

    let mut verification_map: HashMap<String, String> = HashMap::new();
    if let Ok(out) = messenger
        .db
        .batch_get_item()
        .set_request_items(Some(request_items))
        .send()
        .await
    {
        if let Some(responses) = out.responses {
            if let Some(items) = responses.get("eduxal-verifications") {
                for item in items {
                    if let Ok(verification) = Verification::try_from(item) {
                        verification_map.insert(verification.phone.as_ref().to_string(), verification.code);
                    }
                }
            }
        }
    }

    // 2. Dispatch all WhatsApp requests concurrently
    let mut tasks = Vec::new();
    let mut batch_item_failures = Vec::new();

    for (phone, msg_id) in &phone_to_msg_id {
        if let Some(code) = verification_map.get(phone) {
            let messenger_ref = Arc::clone(messenger);
            let phone = phone.clone();
            let code = code.clone();
            let msg_id = msg_id.clone();

            tasks.push(async move {
                match messenger_ref.send_whatsapp(&phone, &code).await {
                    Ok(_) => None,
                    Err(_) => Some(BatchItemFailure {
                        item_identifier: msg_id,
                    }),
                }
            });
        } else {
            // If verification record missing in DynamoDB, mark message as failed for retry
            batch_item_failures.push(BatchItemFailure {
                item_identifier: msg_id.clone(),
            });
        }
    }

    let results = futures::future::join_all(tasks).await;
    for failure in results.into_iter().flatten() {
        batch_item_failures.push(failure);
    }

    Ok(SqsBatchResponse { batch_item_failures })
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let messenger = Arc::new(Messenger::new().await);
    run(service_fn(|event| function_handler(event, &messenger))).await
}
