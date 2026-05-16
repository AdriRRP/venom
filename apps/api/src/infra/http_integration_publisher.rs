use reqwest::Client;
use std::time::Duration;
use venom_domain::{
    IntegrationEventPublishError, IntegrationEventPublisher, PendingIntegrationEvent,
};

pub const HTTP_INTEGRATION_PUBLISHER_KEY: &str = "http-publisher";

#[derive(Debug, Clone)]
pub struct HttpIntegrationPublisher {
    client: Client,
    endpoint_url: Box<str>,
}

impl HttpIntegrationPublisher {
    /// Build one bounded HTTP integration publisher.
    ///
    /// # Errors
    ///
    /// Returns an error string when the timeout is invalid or the HTTP client
    /// cannot be constructed safely.
    pub fn new(endpoint_url: impl Into<Box<str>>, timeout_ms: u32) -> Result<Self, String> {
        if timeout_ms == 0 {
            return Err("http publisher timeout must be greater than zero".to_owned());
        }

        let endpoint_url = endpoint_url.into();
        let client = Client::builder()
            .timeout(Duration::from_millis(u64::from(timeout_ms)))
            .build()
            .map_err(|error| format!("http publisher client build failed: {error}"))?;

        Ok(Self {
            client,
            endpoint_url,
        })
    }
}

impl IntegrationEventPublisher for HttpIntegrationPublisher {
    fn publisher_key(&self) -> &'static str {
        HTTP_INTEGRATION_PUBLISHER_KEY
    }

    async fn publish<'a>(
        &'a self,
        event: &'a PendingIntegrationEvent,
    ) -> Result<(), IntegrationEventPublishError> {
        let response = self
            .client
            .post(self.endpoint_url.as_ref())
            .json(event)
            .send()
            .await
            .map_err(|error| as_publish_error(&error))?;

        if response.status().is_success() {
            return Ok(());
        }

        Err(status_publish_error(response.status().as_u16()))
    }
}

fn as_publish_error(error: &reqwest::Error) -> IntegrationEventPublishError {
    if error.is_timeout() {
        return IntegrationEventPublishError::new(true, "http publisher timeout");
    }

    let message = bound_message(error.to_string());
    IntegrationEventPublishError::new(true, message)
}

const fn is_retryable_status(status: u16) -> bool {
    status == 408 || status == 425 || status == 429 || status >= 500
}

fn status_publish_error(status: u16) -> IntegrationEventPublishError {
    IntegrationEventPublishError::new(
        is_retryable_status(status),
        format!("http publisher returned status {status}"),
    )
}

fn bound_message(message: String) -> Box<str> {
    const MAX_LEN: usize = 192;
    if message.len() <= MAX_LEN {
        return message.into_boxed_str();
    }

    let mut bounded = message;
    bounded.truncate(MAX_LEN);
    bounded.into_boxed_str()
}

#[cfg(test)]
mod tests {
    use super::{bound_message, status_publish_error};

    #[test]
    fn server_errors_are_retryable() {
        let error = status_publish_error(503);
        assert!(error.retryable);
        assert_eq!(error.message.as_ref(), "http publisher returned status 503");
    }

    #[test]
    fn client_errors_are_not_retryable() {
        let error = status_publish_error(400);
        assert!(!error.retryable);
        assert_eq!(error.message.as_ref(), "http publisher returned status 400");
    }

    #[test]
    fn bound_messages_stay_compact() {
        let message = "x".repeat(300);
        let bounded = bound_message(message);
        assert_eq!(bounded.len(), 192);
    }
}
