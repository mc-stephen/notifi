//! Channel delivery modules — all channels are modules of the `server` crate.
//! Channels group their providers: `chat_channel/providers/slack.rs` is a
//! chat provider, not its own channel. Only `email_channel` is wired into
//! the API today; the rest are available for future channel plugins.

pub mod android_channel;
pub mod chat_channel;
pub mod email_channel;
pub mod ios_channel;
pub mod linux_channel;
pub mod macos_channel;
pub mod push_channel;
pub mod sms_channel;
pub mod web_channel;
pub mod webhook_channel;
pub mod window_channel;
