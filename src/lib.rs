//! Deterministic, client-safe policy for a feedless Happy Wakey briefing.
//!
//! This crate deliberately has no network, database, connector, model, or
//! credential access. It decides whether already-classified content may expose
//! a provider deep link and validates public vector/regression descriptors.

use happy_wakey_interfaces::{
    BriefingCard, BriefingCardPriority, ConnectorKind, CorrelationFinding, EmbeddingDescriptor,
    SafeDeepLink, UsefulnessDisposition,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;

pub const MIN_USEFULNESS_SCORE: f32 = 0.8;
pub const MAX_EMBEDDING_DIMENSIONS: u32 = 4_100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeepLinkDenial {
    MissingDecision,
    NotUseful,
    BelowThreshold,
    InconsistentDecision,
    FeedFallbackRequested,
    InvalidExpiry,
    Expired,
    InvalidUrl,
    UnsupportedTarget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescriptorError {
    InvalidDimensions,
    InvalidContentHash,
    InvalidCoefficient,
    InvalidProbability,
    InvalidConfidenceInterval,
    InsufficientSample,
    CausalClaim,
}

/// Authorize a specific provider item, never a generic social landing page.
///
/// The caller supplies trusted current time. The function remains deterministic
/// and does not inspect or retain source content.
///
/// # Errors
///
/// Returns a fail-closed denial when classification, expiry, URL, connector,
/// or item-specific routing requirements are not satisfied.
pub fn authorize_deep_link(
    card: &BriefingCard,
    now: OffsetDateTime,
) -> Result<&SafeDeepLink, DeepLinkDenial> {
    let link = card
        .deep_link
        .as_ref()
        .ok_or(DeepLinkDenial::MissingDecision)?;
    let decision = card
        .usefulness
        .as_ref()
        .ok_or(DeepLinkDenial::MissingDecision)?;

    if decision.disposition != UsefulnessDisposition::Useful {
        return Err(DeepLinkDenial::NotUseful);
    }
    if !decision.score.is_finite() || decision.score < MIN_USEFULNESS_SCORE {
        return Err(DeepLinkDenial::BelowThreshold);
    }
    if decision.decision_id != link.decision_id || decision.source_item_ref != link.source_item_ref
    {
        return Err(DeepLinkDenial::InconsistentDecision);
    }
    if link.feed_fallback_allowed {
        return Err(DeepLinkDenial::FeedFallbackRequested);
    }
    let expiry = OffsetDateTime::parse(&link.expires_at, &Rfc3339)
        .map_err(|_| DeepLinkDenial::InvalidExpiry)?;
    if expiry <= now {
        return Err(DeepLinkDenial::Expired);
    }
    let target = Url::parse(&link.target_url).map_err(|_| DeepLinkDenial::InvalidUrl)?;
    if target.scheme() != "https" || !provider_item_matches(link.connector, &target) {
        return Err(DeepLinkDenial::UnsupportedTarget);
    }
    Ok(link)
}

fn provider_item_matches(connector: ConnectorKind, target: &Url) -> bool {
    let Some(host) = target.host_str().map(str::to_ascii_lowercase) else {
        return false;
    };
    if !target.username().is_empty() || target.password().is_some() {
        return false;
    }
    let path = target.path();
    match connector {
        ConnectorKind::Linkedin => {
            matches!(host.as_str(), "linkedin.com" | "www.linkedin.com")
                && path.starts_with("/messaging/thread/")
                && path.len() > "/messaging/thread/".len()
        }
        ConnectorKind::XDm => {
            matches!(
                host.as_str(),
                "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com"
            ) && path.starts_with("/messages/")
                && path.len() > "/messages/".len()
        }
        ConnectorKind::Whatsapp => {
            matches!(
                host.as_str(),
                "wa.me" | "api.whatsapp.com" | "web.whatsapp.com"
            ) && path != "/"
                && !path.is_empty()
        }
        ConnectorKind::Slack => host.ends_with(".slack.com") && path.starts_with("/archives/"),
        ConnectorKind::Teams => {
            matches!(host.as_str(), "teams.microsoft.com" | "teams.live.com")
                && path.starts_with("/l/message/")
        }
        ConnectorKind::Email => {
            matches!(
                host.as_str(),
                "mail.google.com" | "outlook.office.com" | "outlook.live.com"
            ) && path != "/"
        }
        ConnectorKind::Calendar
        | ConnectorKind::Weather
        | ConnectorKind::Flights
        | ConnectorKind::Markets
        | ConnectorKind::Crm => false,
    }
}

/// Remove unsafe links while retaining the informational card itself.
#[must_use]
pub fn strip_unauthorized_deep_links(
    cards: impl IntoIterator<Item = BriefingCard>,
    now: OffsetDateTime,
) -> Vec<BriefingCard> {
    cards
        .into_iter()
        .map(|mut card| {
            if card.deep_link.is_some() && authorize_deep_link(&card, now).is_err() {
                card.deep_link = None;
            }
            card
        })
        .collect()
}

/// Rank a bounded HUD by urgency without turning it into an infinite feed.
#[must_use]
pub fn rank_and_bound_cards(mut cards: Vec<BriefingCard>, limit: usize) -> Vec<BriefingCard> {
    let safe_limit = limit.min(64);
    cards.sort_by_key(|card| priority_rank(card.priority));
    cards.truncate(safe_limit);
    cards
}

const fn priority_rank(priority: BriefingCardPriority) -> u8 {
    match priority {
        BriefingCardPriority::Critical => 0,
        BriefingCardPriority::High => 1,
        BriefingCardPriority::Normal => 2,
        BriefingCardPriority::Low => 3,
    }
}

/// Validate public embedding metadata without accepting vector values.
///
/// # Errors
///
/// Returns an error for an out-of-range dimension or a non-canonical SHA-256.
pub fn validate_embedding_descriptor(
    descriptor: &EmbeddingDescriptor,
) -> Result<(), DescriptorError> {
    if !(1..=MAX_EMBEDDING_DIMENSIONS).contains(&descriptor.dimensions) {
        return Err(DescriptorError::InvalidDimensions);
    }
    if !is_sha256(&descriptor.content_sha256) {
        return Err(DescriptorError::InvalidContentHash);
    }
    Ok(())
}

/// Validate that a discovery finding is bounded and explicitly non-causal.
///
/// # Errors
///
/// Returns an error for invalid statistics, an undersized sample, or any
/// attempt to mark the correlation as a causal claim.
pub fn validate_correlation_finding(finding: &CorrelationFinding) -> Result<(), DescriptorError> {
    if !finding.coefficient.is_finite() || !(-1.0..=1.0).contains(&finding.coefficient) {
        return Err(DescriptorError::InvalidCoefficient);
    }
    if !finding.p_value.is_finite() || !(0.0..=1.0).contains(&finding.p_value) {
        return Err(DescriptorError::InvalidProbability);
    }
    if !finding.confidence_low.is_finite()
        || !finding.confidence_high.is_finite()
        || finding.confidence_low > finding.confidence_high
        || finding.coefficient < finding.confidence_low
        || finding.coefficient > finding.confidence_high
    {
        return Err(DescriptorError::InvalidConfidenceInterval);
    }
    if finding.sample_size < 3 {
        return Err(DescriptorError::InsufficientSample);
    }
    if finding.causal_claim_allowed {
        return Err(DescriptorError::CausalClaim);
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use happy_wakey_interfaces::{BriefingCardKind, UsefulnessDecision, UsefulnessReason};

    fn now() -> OffsetDateTime {
        OffsetDateTime::parse("2026-09-05T12:00:00Z", &Rfc3339).expect("valid time")
    }

    fn useful_card() -> BriefingCard {
        BriefingCard {
            card_id: "card-1".into(),
            kind: BriefingCardKind::UsefulMessage,
            priority: BriefingCardPriority::High,
            title: "A reply is blocking delivery".into(),
            summary: "A short redacted summary".into(),
            source_label: "LinkedIn message".into(),
            observed_at: "2026-09-05T11:00:00Z".into(),
            action_by: None,
            deep_link: Some(SafeDeepLink {
                link_id: "link-1".into(),
                connector: ConnectorKind::Linkedin,
                target_url: "https://www.linkedin.com/messaging/thread/example".into(),
                decision_id: "decision-1".into(),
                source_item_ref: "source-1".into(),
                expires_at: "2026-09-05T14:00:00Z".into(),
                requires_reauthentication: true,
                feed_fallback_allowed: false,
            }),
            usefulness: Some(UsefulnessDecision {
                decision_id: "decision-1".into(),
                source_item_ref: "source-1".into(),
                disposition: UsefulnessDisposition::Useful,
                score: 0.92,
                reasons: vec![UsefulnessReason::BlockingOthers],
                model_ref: "model-1".into(),
                policy_version: "policy-1".into(),
                evaluated_at: "2026-09-05T11:01:00Z".into(),
                content_sha256: "a".repeat(64),
            }),
            uncertainty_notice: None,
        }
    }

    #[test]
    fn authorizes_specific_useful_message() {
        let card = useful_card();
        assert_eq!(
            authorize_deep_link(&card, now())
                .expect("authorized")
                .link_id,
            "link-1"
        );
    }

    #[test]
    fn denies_generic_feed_and_expired_links() {
        let mut card = useful_card();
        card.deep_link.as_mut().expect("link").target_url = "https://www.linkedin.com/feed/".into();
        assert_eq!(
            authorize_deep_link(&card, now()),
            Err(DeepLinkDenial::UnsupportedTarget)
        );

        card.deep_link.as_mut().expect("link").target_url =
            "https://www.linkedin.com/messaging/thread/example".into();
        card.deep_link.as_mut().expect("link").expires_at = "2026-09-05T11:00:00Z".into();
        assert_eq!(
            authorize_deep_link(&card, now()),
            Err(DeepLinkDenial::Expired)
        );
    }

    #[test]
    fn denies_below_threshold_and_mismatched_decisions() {
        let mut card = useful_card();
        card.usefulness.as_mut().expect("decision").score = 0.79;
        assert_eq!(
            authorize_deep_link(&card, now()),
            Err(DeepLinkDenial::BelowThreshold)
        );
        card.usefulness.as_mut().expect("decision").score = 0.9;
        card.deep_link.as_mut().expect("link").source_item_ref = "other".into();
        assert_eq!(
            authorize_deep_link(&card, now()),
            Err(DeepLinkDenial::InconsistentDecision)
        );
    }

    #[test]
    fn strips_unsafe_links_and_bounds_hud() {
        let mut unsafe_card = useful_card();
        unsafe_card
            .deep_link
            .as_mut()
            .expect("link")
            .feed_fallback_allowed = true;
        let cards = strip_unauthorized_deep_links(vec![unsafe_card], now());
        assert!(cards[0].deep_link.is_none());

        let mut cards = vec![useful_card(); 80];
        cards[0].priority = BriefingCardPriority::Critical;
        let cards = rank_and_bound_cards(cards, 1000);
        assert_eq!(cards.len(), 64);
        assert_eq!(cards[0].priority, BriefingCardPriority::Critical);
    }
}
