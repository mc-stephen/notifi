//! Request/response shapes for campaigns (camelCase).
//!
//! Stats (`totals`) are derived from the campaign's expanded send rows;
//! campaigns that never fired carry `estimatedAudience` instead.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::campaigns::entities::Campaign;

/// Serde helper for patch fields that distinguish "absent" (no change)
/// from an explicit `null` (clear the field) on `Option<Option<T>>`.
/// Plain serde treats both as `None`; `null` must arrive as `Some(None)`.
///
/// Use as `#[serde(default, deserialize_with = "double_option")]`.
pub(crate) fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// Derived, never stored: one might read these as
/// `sent = delivered + failed`, `remaining = pending`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignStatsDto {
    pub total: usize,
    pub delivered: usize,
    pub failed: usize,
    pub pending: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub channel: String,
    pub template_id: Option<String>,
    /// The frozen copy snapshot (rendered from the template at compose
    /// time — read-only by design).
    pub title: String,
    pub body: String,
    /// Who gets it: "all" (channel-capable recipients) or "custom".
    pub audience_mode: String,
    /// Custom-mode keys (recipient `user_id`s), snapshotted at compose.
    pub selected_user_ids: Vec<String>,
    /// Explicit primary provider pick when named (empty = auto order).
    pub provider_id: String,
    pub status: String,
    pub send_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Campaign> for CampaignDto {
    fn from(c: Campaign) -> Self {
        Self {
            id: c.id,
            project_id: c.project_id,
            name: c.name,
            description: c.description,
            created_by: c.created_by,
            channel: c.channel,
            template_id: c.template_id,
            title: c.title,
            body: c.body,
            audience_mode: c.audience_mode.as_str().to_string(),
            selected_user_ids: c.selected_user_ids,
            provider_id: c.provider_id,
            status: c.status.as_str().to_string(),
            send_at: c.send_at,
            started_at: c.started_at,
            completed_at: c.completed_at,
            last_error: c.last_error,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// A campaign paired with its derived stats (list + detail responses).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignWithStatsDto {
    #[serde(flatten)]
    pub campaign: CampaignDto,
    pub totals: CampaignStatsDto,
    /// Fire-time audience estimate for campaigns that haven't expanded
    /// yet — all-mode counts channel-capable recipients, custom counts
    /// the selection.
    pub estimated_audience: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCampaignRequest {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Campaigns compose from templates — missing id is a 400.
    pub template_id: Option<String>,
    /// Which template version (channel) to send — required; a validated
    /// missing/blank value is a 400.
    pub channel: Option<String>,
    /// The connected provider to try first (optional; the other
    /// connected providers for the channel stay as fallbacks).
    pub provider_id: Option<String>,
    pub audience_mode: Option<String>,
    pub selected_user_ids: Option<Vec<String>>,
    /// Some when scheduling the fire; omitted for a plain draft.
    pub send_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCampaignRequest {
    pub project_id: String,
    pub name: Option<String>,
    /// Some(text) sets (trimmed; empty string clears); absent keeps.
    pub description: Option<String>,
    /// A (different) template id re-renders the snapshot.
    pub template_id: Option<String>,
    /// Re-picks the template channel to send through (must be covered).
    pub channel: Option<String>,
    /// Sets the primary provider pick (empty string clears → auto order;
    /// absent leaves untouched).
    pub provider_id: Option<String>,
    pub audience_mode: Option<String>,
    pub selected_user_ids: Option<Vec<String>>,
    /// `Some(None)` (explicit null) turns a scheduled campaign back into
    /// a draft.
    #[serde(default, deserialize_with = "double_option")]
    pub send_at: Option<Option<DateTime<Utc>>>,
}

/// Empty body — the path carries the action.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignActionRequest {
    pub project_id: String,
}
