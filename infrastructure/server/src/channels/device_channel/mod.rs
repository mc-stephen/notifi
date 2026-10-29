//! Device channel delivery — one channel family for everything that
//! lands on a user's device: native app push per OS flavor (android,
//! ios, macos, linux) and Windows (webhook or WNS), plus browser push
//! (web).
//!
//! The OS is the provider, not a channel of its own: IOS and macOS share
//! one APNs controller, Windows picks a flavor by config, and the
//! destination shapes genuinely differ per OS (registration token /
//! self-hosted endpoint / WNS channel URI / web-push subscription), so
//! the family dispatches by variant rather than a uniform message type.
//! Onboarding docs for vendors live in `docs/`.
//!
//! The dashboard's provider registry (`/app/providers`) derives its
//! device rows from `providers::catalog()` — adding a vendor is its
//! provider file plus the destination's `Provider` enum and `vendors()`
//! listing (golden test `catalog_pins_every_family_variant` pins both
//! lists to each other). Device tokens are recipient data — the
//! recipient's contacts blob carries a `devices` array (§25); delivery
//! is adapter-wired in `infra::delivery_adapters` — every registry
//! vendor sends (fcm, pushy, onesignal; apns + onesignal;
//! endpoint; webhook + WNS; web_push), and the blob walker that fans a
//! recipient's `devices` array out per install is still the send
//! layer's pending job.

pub mod providers;
