//! Channel delivery modules — all channels are modules of the `server` crate.
//! Channels group their vendor controllers: `chat_channel/slack.rs` is a
//! chat vendor, not its own channel, and the device family
//! (`device_channel`) holds android, apple, window, and web as
//! destination flavors of one channel. Every family delivers —
//! `infra::delivery_adapters` wires each registry card to its
//! controller, and the catalogs (`device_channel::catalog()`,
//! `other_channel::catalog()`) feed the provider registry
//! (`/app/providers`).

pub mod chat_channel;
pub mod device_channel;
pub mod email_channel;
pub mod other_channel;
pub mod sms_channel;
pub mod webhook_channel;
