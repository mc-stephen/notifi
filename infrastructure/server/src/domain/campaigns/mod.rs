//! Campaigns: message origins (one channel × one audience × one message
//! × one timing). They own no flow — doctrine flows at the module docs
//! of the service; delivery goes straight through connected providers.
//!
//! Routes: `/app/campaigns`.

pub mod entities;
pub mod services;

pub use entities::{
    AudienceMode, Campaign, CampaignInput, CampaignPatch, CampaignStats, CampaignStatus,
};
pub use services::CampaignService;
