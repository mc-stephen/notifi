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

pub mod providers;
