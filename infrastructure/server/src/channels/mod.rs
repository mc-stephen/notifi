//! Channel delivery modules — all channels are modules of the `server` crate.
//! Channels group their providers: `chat_channel/providers/slack.rs` is a
//! chat provider, not its own channel, and the device family
//! (`device_channel/providers/`) holds android, ios, macos, linux,
//! window, and web as provider flavors of one destination class. Only
//! `email_channel` is wired into the API today; the rest are available
//! for future channel plugins.

pub mod chat_channel;
pub mod device_channel;
pub mod email_channel;
pub mod push_channel;
pub mod sms_channel;
pub mod webhook_channel;
