//! Email sending for magic link authentication.
//!
//! Sends magic link emails to users via SMTP.

use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};

/// Email configuration.
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port.
    pub smtp_port: u16,
    /// SMTP username (optional for local testing).
    pub smtp_user: Option<String>,
    /// SMTP password (optional for local testing).
    pub smtp_pass: Option<String>,
    /// From email address.
    pub from_email: String,
    /// From display name.
    pub from_name: String,
}

/// Errors that can occur when sending email.
#[derive(Debug)]
pub enum EmailError {
    /// Error building the email message.
    MessageError(String),
    /// Error sending the email.
    TransportError(String),
    /// Email sending is not configured.
    NotConfigured,
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::MessageError(e) => write!(f, "Failed to build email: {}", e),
            EmailError::TransportError(e) => write!(f, "Failed to send email: {}", e),
            EmailError::NotConfigured => write!(f, "Email sending is not configured"),
        }
    }
}

impl std::error::Error for EmailError {}

/// Email sender for magic link authentication.
#[derive(Clone)]
pub struct EmailSender {
    config: EmailConfig,
}

impl EmailSender {
    /// Creates a new email sender with the given configuration.
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    /// Sends a magic link email to the specified address.
    ///
    /// # Arguments
    /// * `to` - Recipient email address
    /// * `name` - Optional recipient name for personalization
    /// * `link` - The magic link URL
    pub async fn send_magic_link(
        &self,
        to: &str,
        name: Option<&str>,
        link: &str,
    ) -> Result<(), EmailError> {
        let greeting = match name {
            Some(n) => format!("Hi {},", n),
            None => "Hi,".to_string(),
        };

        let body = format!(
            r#"{greeting}

Click the link below to sign in to ToduFit:

{link}

This link expires in 10 minutes.

If you didn't request this, you can ignore this email.

- ToduFit"#
        );

        let from = format!("{} <{}>", self.config.from_name, self.config.from_email);

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|e| EmailError::MessageError(format!("{}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| EmailError::MessageError(format!("{}", e)))?)
            .subject("Sign in to ToduFit")
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| EmailError::MessageError(e.to_string()))?;

        // Build transport based on config
        let transport = self.build_transport()?;

        transport
            .send(email)
            .await
            .map_err(|e| EmailError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Builds the SMTP transport.
    fn build_transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, EmailError> {
        let mut builder = if self.config.smtp_port == 465 {
            // SSL/TLS on port 465
            AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
                .map_err(|e| EmailError::TransportError(e.to_string()))?
                .port(465)
        } else {
            // STARTTLS on port 587 or plain for local testing
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.smtp_host)
                .map_err(|e| EmailError::TransportError(e.to_string()))?
                .port(self.config.smtp_port)
        };

        // Add credentials if provided
        if let (Some(user), Some(pass)) = (&self.config.smtp_user, &self.config.smtp_pass) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        Ok(builder.build())
    }
}

impl std::fmt::Debug for EmailSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailSender")
            .field("smtp_host", &self.config.smtp_host)
            .field("smtp_port", &self.config.smtp_port)
            .field("from_email", &self.config.from_email)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EmailConfig {
        EmailConfig {
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: None,
            smtp_pass: None,
            from_email: "noreply@example.com".to_string(),
            from_name: "ToduFit".to_string(),
        }
    }

    #[test]
    fn test_email_sender_new() {
        let config = test_config();
        let sender = EmailSender::new(config);

        assert_eq!(sender.config.smtp_host, "localhost");
        assert_eq!(sender.config.smtp_port, 1025);
    }

    #[test]
    fn test_email_config_with_credentials() {
        let config = EmailConfig {
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            smtp_user: Some("user@example.com".to_string()),
            smtp_pass: Some("secret".to_string()),
            from_email: "noreply@example.com".to_string(),
            from_name: "ToduFit".to_string(),
        };

        let sender = EmailSender::new(config);
        assert!(sender.config.smtp_user.is_some());
        assert!(sender.config.smtp_pass.is_some());
    }

    #[test]
    fn test_email_error_display() {
        let err = EmailError::MessageError("invalid address".to_string());
        assert!(err.to_string().contains("invalid address"));

        let err = EmailError::TransportError("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));

        let err = EmailError::NotConfigured;
        assert!(err.to_string().contains("not configured"));
    }
}
