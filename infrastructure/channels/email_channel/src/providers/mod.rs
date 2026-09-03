pub mod smtp;
pub mod sendgrid;
pub mod resend;
pub mod aws_ses;
pub mod postmark;
pub mod mailgun;
pub mod brevo;

/// All supported email providers. THIS IS THE SOURCE OF TRUTH.
/// To add a new provider: create providers/new_provider.rs, add variant here.
#[derive(Debug, Clone)]
pub enum EmailProvider {
    Smtp(smtp::SmtpConfig),
    SendGrid(sendgrid::SendGridConfig),
    Resend(resend::ResendConfig),
    AwsSes(aws_ses::AwsSesConfig),
    Postmark(postmark::PostmarkConfig),
    Mailgun(mailgun::MailgunConfig),
    Brevo(brevo::BrevoConfig),
}

impl EmailProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Smtp(_) => "smtp",
            Self::SendGrid(_) => "sendgrid",
            Self::Resend(_) => "resend",
            Self::AwsSes(_) => "aws_ses",
            Self::Postmark(_) => "postmark",
            Self::Mailgun(_) => "mailgun",
            Self::Brevo(_) => "brevo",
        }
    }
}
