use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_sqs::Client as SqsClient;

#[derive(Clone)]
pub struct Config {
    db: DynamoClient,
    sqs: SqsClient,
    messenger_queue_url: String,
}

impl Config {
    pub async fn new() -> Self {
        let sdk_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let db = DynamoClient::new(&sdk_config);
        let sqs = SqsClient::new(&sdk_config);
        let messenger_queue_url = std::env::var("MESSENGER_QUEUE_URL")
            .unwrap_or_else(|_| "https://sqs.us-east-1.amazonaws.com/123456789012/eduxal-messenger-queue".to_string());

        Self {
            db,
            sqs,
            messenger_queue_url,
        }
    }

    pub fn db(&self) -> &DynamoClient {
        &self.db
    }

    pub fn sqs(&self) -> &SqsClient {
        &self.sqs
    }

    pub fn messenger_queue_url(&self) -> &str {
        &self.messenger_queue_url
    }
}
