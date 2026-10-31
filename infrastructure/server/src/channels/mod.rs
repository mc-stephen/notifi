//! Channel delivery modules — all channels are modules of the `server` crate.
//! Channels group their vendor controllers: `chat_channel/slack.rs` is a
//! chat vendor, not its own channel; the device family (`device_channel`)
//! holds android, apple, window, and web as destination flavors; and the
//! other family (`other_channel`) owns the cross-platform sorts — the
//! push gateways' single cards, the retired-webhook relay controller
//! (the function placeholder was removed), and the hosted gateway's
//! card logic (`gateway.rs`, door at `api/routes/gateway.rs`). Every
//! family delivers —
//! `infra::delivery_adapters` wires each registry card to its
//! controller, and the catalogs (`device_channel::catalog()`,
//! `other_channel::catalog()`) feed the provider registry
//! (`/app/providers`).

pub mod chat_channel;
pub mod device_channel;
pub mod email_channel;
pub mod other_channel;
pub mod sms_channel;
