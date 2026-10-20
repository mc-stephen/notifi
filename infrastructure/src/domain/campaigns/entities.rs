//! Campaign entities: lifecycle, composition, audience scope.
//! Framework-free.

/// Who a campaign targets. `All` = every recipient with a contact for
/// the channel, resolved live at fire time; `Custom` = only the
/// recipients selected at compose time (snapshot of keys, contacts
/// still resolved live so edits show through).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudienceMode {
    #[default]
    All,
    Custom,
}

impl AudienceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Campaign lifecycle. `scheduled` waits for its `send_at`; `running`
/// drains its expanded send rows in batches; `paused` owes to the worker
/// (in-flight batches finish, pending rows wait); the final states are
/// terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignStatus {
    Draft,
    Scheduled,
    Running,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

impl CampaignStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Scheduled => "scheduled",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "scheduled" => Some(Self::Scheduled),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "cancelled" => Some(Self::Cancelled),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Drafts and scheduled campaigns are the only editable ones —
    /// everything else already carries fired sends.
    pub fn editable(self) -> bool {
        matches!(self, Self::Draft | Self::Scheduled)
    }
}

/// One campaign: one channel × one audience × one message × one timing.
///
/// Campaigns are message origins, not automation origins — they own no
/// flow and deliver directly through the project's connected providers
/// for their channel (nothing connects to flows at all). Template
/// campaigns snapshot their rendered content at compose time; later
/// template edits never change a stored campaign.
#[derive(Debug, Clone)]
pub struct Campaign {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub channel: String,
    pub template_id: Option<String>,
    /// The composed message snapshot (rendered from the template at
    /// compose time — campaigns compose from templates by design).
    pub title: String,
    pub body: String,
    /// Who gets it.
    pub audience_mode: AudienceMode,
    /// Custom-mode targeting keys (recipient `user_id`s), snapshotted at
    /// compose time.
    pub selected_user_ids: Vec<String>,
    /// Explicit primary provider pick (empty = no explicit pick all
    /// connected providers for the channel try in store order). Tried
    /// first; other connected providers stay as fallbacks.
    pub provider_id: String,
    pub status: CampaignStatus,
    /// For scheduled campaigns: when the worker claims this row.
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Why a campaign failed (empty audience, storage failure, ...).
    /// Row-level delivery problems live on each send row instead.
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for composing a campaign. Campaigns compose from templates by
/// design — `template_id` is mandatory and `channel` names the template
/// version to send (the exact channel the campaign delivers through).
#[derive(Debug, Clone, Default)]
pub struct CampaignInput {
    pub name: String,
    pub description: Option<String>,
    pub template_id: String,
    /// Which template version (channel) to send: email, sms, push, …
    pub channel: String,
    /// Explicit primary provider (the connected config to try first);
    /// None lets the store order decide with the rest as fallbacks.
    pub provider_id: Option<String>,
    pub audience_mode: AudienceMode,
    pub selected_user_ids: Vec<String>,
    /// Some = scheduled for this time (must be in the future); None
    /// creates a plain draft.
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Patch a campaign (draft/scheduled only). A different `template_id`
/// re-renders; a `channel` re-picks the template channel to snapshot
/// (must be covered by the template); a `provider_id` set re-picks the
/// primary provider (empty string clears it); removing `send_at` turns a
/// scheduled campaign back into a draft. Copy is never hand-edited —
/// only re-rendered from a template.
#[derive(Debug, Clone, Default)]
pub struct CampaignPatch {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub template_id: Option<String>,
    pub channel: Option<String>,
    /// Set the primary provider pick (empty string clears it); absent
    /// leaves it untouched.
    pub provider_id: Option<String>,
    pub audience_mode: Option<AudienceMode>,
    pub selected_user_ids: Option<Vec<String>>,
    pub send_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

/// Derived from the campaign's expanded send rows — never stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CampaignStats {
    /// Expanded rows (recipients contacted by this campaign).
    pub total: usize,
    pub delivered: usize,
    pub failed: usize,
    pub pending: usize,
}
