//! Record-deal scouting, buzz, scene poaching of rejected offers, and (M9)
//! the term a deal carries at signing plus the recoupment-dependent
//! renewal window that opens before a healthy term's expiry (design §E-4).
//!
//! **L-BID (design §E).** The new-signing offer stream is a *market priced from
//! live label state*: an offer's advance and royalty come from the offering
//! [`Label`]'s current `capital` / `market_reach` / `financial_power` / `tier`
//! and the act's fame (`priced_advance` / `priced_royalty`), not the static
//! `record_labels.json` row — and a label cannot advance money it does not have
//! (the advance is bounded by `capital`). More than one label can table an offer
//! in the same window, and when two-plus want the act their terms escalate
//! toward the richer/hungrier label's ceiling (`escalate_offers`), each bounded
//! by its own means. The offer *paperwork* (lifetime/expiry, the renewal window)
//! is unchanged — only how offers are priced and sourced. A pre-0.8 save carries
//! no seeded labels, so `generate_deal_offers` falls back to the legacy
//! JSON-priced path there (`generate_deal_offers_from_json`) — an old save can
//! still get signed. All rolls derive from the injected world seed; labels
//! iterate in `BTreeMap` id order, so a seed replays the market identically.

use crate::data_loader::{GameDataFiles, InterestThreshold, RecordLabel, RecordLabelsData};
use crate::game::band::Band;
use crate::game::constants::{
    ADVANCE_CAPITAL_FRACTION, ADVANCE_FAME_FLOOR, ADVANCE_MIN_VIABLE_AFFORDABLE,
    ADVANCE_STATURE_PER_FP_CUBED, BID_ESCALATION_FRACTION, DEAL_EXTENSION_ADVANCE_FRACTION,
    DEAL_EXTENSION_ALBUMS, DEAL_EXTENSION_TERM_WEEKS, DEAL_NEW_CONTRACT_ROYALTY_BUMP_MAX,
    DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN, DEAL_RENEWAL_DECENT_SALES_MIN_COMMERCIAL_SUCCESS,
    DEAL_RENEWAL_DEEP_RED_UNRECOUPED, DEAL_RENEWAL_WEAK_SALES_MAX_COMMERCIAL_SUCCESS,
    DEAL_RENEWAL_WINDOW_WEEKS, DEAL_TERM_BOUTIQUE_WEEKS, DEAL_TERM_INDEPENDENT_WEEKS,
    DEAL_TERM_MAJOR_WEEKS, GENRE_FIT_CHANCE_MULTIPLIER, OFFER_CHANCE_MAX, ROYALTY_BID_PREMIUM,
    ROYALTY_FAME_BONUS_MAX, ROYALTY_HARD_CAP, ROYALTY_REACH_MAX, ROYALTY_REACH_MIN,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::GameWorld;
use super::labels::{Label, LabelStatus, LabelTier};

// Deal-offer buzz: the scale the tier thresholds in record_labels.json
// measure (independent 10, boutique 20, major 30). Fame carries most of it;
// records prove you can deliver; a charting record adds short-lived heat.
pub(crate) const BUZZ_PER_SINGLE: u32 = 3;
pub(crate) const BUZZ_PER_ALBUM: u32 = 4;
pub(crate) const BUZZ_CATALOG_CAP: u32 = 10;
pub(crate) const BUZZ_CHART_BONUS: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotentialDealOffer {
    pub label_name: String,
    pub label_tier: String,
    pub advance: u32,
    pub royalty_rate: f32,
    pub albums_required: u8,
    pub original_label_data: RecordLabel,
    /// The week the label withdraws the offer if it's ignored. `None`
    /// means it never expires — deliberately, because offers saved before
    /// expiry existed deserialize to `None`, and a bare numeric default
    /// (0) would kill every live offer the moment an old save loaded.
    #[serde(default)]
    pub expires_week: Option<u32>,
    /// Contract length in weeks (design §E-4), generated here at offer
    /// time. `0` on a pre-M9 save's still-pending offer — accepting it
    /// then signs a legacy-policy deal (see `RecordDeal::term_weeks`).
    #[serde(default)]
    pub term_weeks: u16,
    /// Set only on a renewal-window EXTENSION offer (design §E-4): the old
    /// deal's `unrecouped` balance, carried into the new deal's ledger on
    /// top of this offer's own (small) advance instead of starting fresh.
    /// `0` for every ordinary signing and every NEW CONTRACT renewal.
    #[serde(default)]
    pub carry_forward_unrecouped: i32,
}

impl GameWorld {
    pub fn poach_rejected_deal(&mut self, label_name: &str, rng: &mut impl Rng) -> Option<String> {
        if !rng.gen_bool(0.6) {
            return None;
        }
        let idx = self
            .bands
            .iter()
            .enumerate()
            .filter(|(_, b)| b.label.is_none() && b.fame >= 20)
            .max_by_key(|(_, b)| b.fame)
            .map(|(i, _)| i)?;
        let band = &mut self.bands[idx];
        band.label = Some(label_name.to_string());
        band.momentum = (band.momentum + 1).min(3);
        Some(band.name.clone())
    }
    pub(super) fn band_buzz(&self, band: &Band) -> u8 {
        let fame_heat = u32::from(band.fame) * 3 / 10;
        let catalog_heat = (band.singles_released.len() as u32 * BUZZ_PER_SINGLE
            + band.albums_released.len() as u32 * BUZZ_PER_ALBUM)
            .min(BUZZ_CATALOG_CAP);
        // (M3 note for M9: this used to read the legacy flat `self.charts`
        // field; that field is now vestigial (design §C — regional Top
        // 100s), so the check moved to the regional boards via
        // `player_is_charting`.)
        let chart_heat = if self.player_is_charting() {
            BUZZ_CHART_BONUS
        } else {
            0
        };
        (fame_heat + catalog_heat + chart_heat).min(100) as u8
    }

    /// The new-signing offer stream (design §E). L-BID makes this a *market
    /// priced from live label state*: each seeded [`Label`] is evaluated against
    /// the act and, if it bites, its offer's advance and royalty are priced from
    /// that label's current `capital` / `market_reach` / `financial_power` and
    /// the act's fame — never the static JSON row. When two-plus labels want the
    /// act their terms escalate (`escalate_offers`). A breach cooldown blocks the
    /// whole stream; a pre-0.8 save with no seeded labels falls back to the
    /// legacy JSON pricing so an old career can still get signed.
    pub fn generate_deal_offers(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<PotentialDealOffer> {
        // M9 (design §E-4): a breach's cooldown blocks every new offer.
        // This is the single choke point `check_and_generate_deal_offers`
        // (turn.rs) calls through for the player, so gating here — rather
        // than at that call site — is enough.
        if band.deal_cooldown > 0 {
            return Vec::new();
        }
        // A pre-0.8 save carries no seeded label entities to price from; fall
        // back to the legacy JSON pricing so an old career can still get signed
        // (the same graceful-degradation L-ENTITY gives the ledger when the
        // label map is empty). Every 0.8 world seeds the full roster, so live
        // play always takes the priced market below.
        if self.labels.is_empty() {
            return self.generate_deal_offers_from_json(band, game_data, rng);
        }
        self.generate_priced_offers(band, game_data, rng)
    }

    /// The live-state market (design §E). Iterate the seeded `labels` in id order
    /// (majors, then independents, then boutiques — the JSON order `seed_labels`
    /// preserves), gate each on the act's fit + the label's appetite, and price a
    /// biting label's offer from its current state. Multiple labels may bite in
    /// the same window; `escalate_offers` then runs the bidding war. Every roll
    /// is the injected world `rng`, drawn in `BTreeMap` id order, so a seed
    /// replays the market identically (the determinism trio is unaffected).
    fn generate_priced_offers(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<PotentialDealOffer> {
        let labels_data = game_data.get_record_labels_data();
        let buzz = self.band_buzz(band);
        let albums_out = band.albums_released.len();
        let releases_out = band.total_releases();

        let mut offers = Vec::new();
        let mut bounds: Vec<OfferBidBound> = Vec::new();

        for label in self.labels.values() {
            // A distressed/folding/acquired label is out of the signing market
            // (L-MNA §F drives those states; L-BID simply respects them).
            if label.status != LabelStatus::Active {
                continue;
            }
            let (tier_name, threshold) = tier_interest(label.tier, labels_data);
            // Interest gate — the act must clear this tier's fame/catalog/buzz
            // bar. Unchanged from the JSON stream: the `singles` column reads as
            // "records out, of any kind", so an album counts as a release too.
            if band.fame < threshold.fame
                || albums_out < usize::from(threshold.albums)
                || releases_out < usize::from(threshold.singles)
                || buzz < threshold.buzz
            {
                continue;
            }
            // Already signed with this label — no self-poaching.
            if band
                .current_deal()
                .is_some_and(|deal| deal.label_name == label.name)
            {
                continue;
            }
            // Appetite / affordability (design §E): a label deep in the red signs
            // cautiously or not at all — it stays quiet unless it can front a
            // viable advance out of its own `capital`.
            let affordable = advance_affordable_ceiling(label);
            if affordable < ADVANCE_MIN_VIABLE_AFFORDABLE {
                continue;
            }
            // The coin flip: tiered by fame, sharpened when the act fits the
            // label's specialty (a fitting act draws more suitors → more bidding).
            let mut chance = tier_offer_chance(tier_name, band.fame);
            if label.genres.contains(&band.genre) {
                chance = (chance * GENRE_FIT_CHANCE_MULTIPLIER).min(OFFER_CHANCE_MAX);
            }
            if !rng.gen_bool(chance) {
                continue;
            }
            // Priced from live state — advance from capital/financial-power/fame,
            // royalty from reach/fame.
            let advance = priced_advance(label, band.fame);
            let royalty_rate = priced_royalty(label, band.fame);
            let albums_required = albums_required_for_tier(tier_name, rng);
            let term_weeks = term_weeks_for_tier(tier_name, rng);

            offers.push(PotentialDealOffer {
                label_name: label.name.clone(),
                label_tier: tier_name.to_string(),
                advance,
                royalty_rate,
                albums_required,
                original_label_data: label_snapshot(label, labels_data),
                // The world has no clock; the game stamps the deadline when the
                // offer lands on the table (`check_and_generate_deal_offers`).
                expires_week: None,
                term_weeks,
                carry_forward_unrecouped: 0,
            });
            bounds.push(OfferBidBound {
                affordable,
                royalty_ceiling: royalty_ceiling(label),
            });
        }

        // Bidding (design §E): with two-plus offers on the table, terms escalate
        // — each toward the best rival's, bounded by that label's own means.
        escalate_offers(&mut offers, &bounds);
        offers
    }

    /// The legacy JSON-priced offer stream, kept as the fallback for a pre-0.8
    /// save whose `labels` map is empty (see `generate_deal_offers`). Byte-for-
    /// byte the pre-L-BID logic, so an old career's offer stream — and thus its
    /// determinism — is unchanged. New 0.8 worlds never reach this path.
    fn generate_deal_offers_from_json(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
    ) -> Vec<PotentialDealOffer> {
        let mut offers = Vec::new();
        let labels_data = game_data.get_record_labels_data();
        let buzz = self.band_buzz(band);

        let label_tiers = [
            (
                "Major",
                &labels_data.major_labels,
                &labels_data
                    .label_requirements
                    .major_label_interest_threshold,
            ),
            (
                "Independent",
                &labels_data.independent_labels,
                &labels_data
                    .label_requirements
                    .independent_label_interest_threshold,
            ),
            (
                "Boutique",
                &labels_data.boutique_labels,
                &labels_data
                    .label_requirements
                    .boutique_label_interest_threshold,
            ),
        ];

        for (tier_name, labels_in_tier, threshold) in &label_tiers {
            for label in *labels_in_tier {
                // The `singles` column reads as "records out, of any kind":
                // an album on the shelf opens doors at least as well as a
                // 45, so an act that went straight to albums isn't
                // invisible to every A&R desk in town.
                if band.fame >= threshold.fame
                    && band.albums_released.len() >= threshold.albums as usize
                    && band.total_releases() >= threshold.singles as usize
                    && buzz >= threshold.buzz
                {
                    // Check if already signed with this label
                    if let Some(current_deal) = band.current_deal()
                        && current_deal.label_name == label.name
                    {
                        continue; // Already signed with this label
                    }

                    // Random chance to make an offer
                    let offer_chance = match *tier_name {
                        "Major" => {
                            if band.fame > 70 {
                                0.30
                            } else if band.fame > 50 {
                                0.20
                            } else {
                                0.10
                            }
                        }
                        "Independent" => {
                            if band.fame > 40 {
                                0.40
                            } else if band.fame > 20 {
                                0.25
                            } else {
                                0.15
                            }
                        }
                        "Boutique" => {
                            if band.fame > 10 {
                                0.50
                            } else {
                                0.20
                            }
                        }
                        _ => 0.10,
                    };

                    if rng.gen_bool(offer_chance) {
                        let advance_percentage = match band.fame {
                            0..=20 => rng.gen_range(0.0..0.4), // Lower end for low fame
                            21..=50 => rng.gen_range(0.3..0.7),
                            51..=100 => rng.gen_range(0.6..1.0), // Higher end for high fame
                            _ => 0.5,
                        };
                        let advance_range_span = label.advance_range[1] - label.advance_range[0];
                        let calculated_advance = label.advance_range[0]
                            + (advance_range_span as f32 * advance_percentage) as u32;

                        let advance = calculated_advance
                            .clamp(label.advance_range[0], label.advance_range[1]);

                        let royalty_rate = label.royalty_rate as f32 / 100.0;

                        let albums_required = match *tier_name {
                            "Major" => rng.gen_range(2..=4),
                            "Independent" => rng.gen_range(1..=3),
                            "Boutique" => rng.gen_range(1..=2),
                            _ => 2,
                        };
                        let term_weeks = term_weeks_for_tier(tier_name, rng);

                        offers.push(PotentialDealOffer {
                            label_name: label.name.clone(),
                            label_tier: tier_name.to_string(),
                            advance,
                            royalty_rate,
                            albums_required,
                            original_label_data: label.clone(),
                            // The world has no clock; the game stamps the
                            // deadline when the offer lands on the table.
                            expires_week: None,
                            term_weeks,
                            carry_forward_unrecouped: 0,
                        });
                    }
                }
            }
        }
        offers
    }

    /// The renewal window (design §E-4): 26 weeks [tune] before a healthy
    /// term's expiry, with all albums delivered, the label looks at its
    /// ledger and makes a move through the same offer stream a fresh
    /// signing uses. `None` when the window isn't open, or when the ledger
    /// says "deep in the red with weak sales" (the label just lets the
    /// clock run out).
    pub fn generate_renewal_offer(
        &self,
        band: &Band,
        game_data: &GameDataFiles,
        rng: &mut impl Rng,
        current_week: u32,
    ) -> Option<PotentialDealOffer> {
        let deal = band.current_deal()?;
        if !deal.renewal_window_open(current_week, DEAL_RENEWAL_WINDOW_WEEKS) {
            return None;
        }
        let labels_data = game_data.get_record_labels_data();
        let label_data = find_label_by_name(labels_data, &deal.label_tier, &deal.label_name)?;

        match renewal_decision(deal.unrecouped, band.reputation.commercial_success) {
            RenewalDecision::Silence => None,
            RenewalDecision::Extension => {
                let advance = (deal.advance as f32 * DEAL_EXTENSION_ADVANCE_FRACTION) as u32;
                Some(PotentialDealOffer {
                    label_name: deal.label_name.clone(),
                    label_tier: deal.label_tier.clone(),
                    advance,
                    royalty_rate: deal.royalty_rate,
                    albums_required: DEAL_EXTENSION_ALBUMS,
                    original_label_data: label_data.clone(),
                    expires_week: None,
                    term_weeks: DEAL_EXTENSION_TERM_WEEKS,
                    // The label protects its investment: the balance it's
                    // still owed carries into the new deal's ledger.
                    carry_forward_unrecouped: deal.unrecouped,
                })
            }
            RenewalDecision::NewContract => {
                let bump = rng.gen_range(
                    DEAL_NEW_CONTRACT_ROYALTY_BUMP_MIN..=DEAL_NEW_CONTRACT_ROYALTY_BUMP_MAX,
                );
                let royalty_rate = (deal.royalty_rate + bump).min(1.0);
                let advance_percentage = match band.fame {
                    0..=20 => rng.gen_range(0.0..0.4),
                    21..=50 => rng.gen_range(0.3..0.7),
                    51..=100 => rng.gen_range(0.6..1.0),
                    _ => 0.5,
                };
                let advance_range_span = label_data.advance_range[1] - label_data.advance_range[0];
                let advance = (label_data.advance_range[0]
                    + (advance_range_span as f32 * advance_percentage) as u32)
                    .clamp(label_data.advance_range[0], label_data.advance_range[1]);
                let term_weeks = term_weeks_for_tier(&deal.label_tier, rng);
                let albums_required = match deal.label_tier.as_str() {
                    "Major" => rng.gen_range(2..=4),
                    "Independent" => rng.gen_range(1..=3),
                    "Boutique" => rng.gen_range(1..=2),
                    _ => 2,
                };
                Some(PotentialDealOffer {
                    label_name: deal.label_name.clone(),
                    label_tier: deal.label_tier.clone(),
                    advance,
                    royalty_rate,
                    albums_required,
                    original_label_data: label_data.clone(),
                    expires_week: None,
                    term_weeks,
                    // Recouped — a fresh ledger starts clean off the new
                    // advance alone.
                    carry_forward_unrecouped: 0,
                })
            }
        }
    }
}

/// Contract term at signing, by tier (design §E-4 table) — shared between a
/// fresh signing and a renewal-window NEW CONTRACT.
fn term_weeks_for_tier(tier_name: &str, rng: &mut impl Rng) -> u16 {
    let (min, max) = match tier_name {
        "Major" => DEAL_TERM_MAJOR_WEEKS,
        "Independent" => DEAL_TERM_INDEPENDENT_WEEKS,
        "Boutique" => DEAL_TERM_BOUTIQUE_WEEKS,
        _ => DEAL_TERM_INDEPENDENT_WEEKS,
    };
    rng.gen_range(min..=max)
}

/// Look up a label's live data by name within its own tier — the renewal
/// window comes from the same label the band is already signed to.
fn find_label_by_name<'a>(
    labels_data: &'a RecordLabelsData,
    label_tier: &str,
    label_name: &str,
) -> Option<&'a RecordLabel> {
    let list = match label_tier {
        "Major" => &labels_data.major_labels,
        "Independent" => &labels_data.independent_labels,
        "Boutique" => &labels_data.boutique_labels,
        _ => return None,
    };
    list.iter().find(|l| l.name == label_name)
}

/// The three-way renewal-window ledger read (design §E-4). Recouped with
/// decent sales earns a new contract; deep in the red with weak sales gets
/// silence; everything else — chiefly "not yet recouped" — gets an
/// extension, the label protecting its investment rather than rewarding
/// the band.
enum RenewalDecision {
    NewContract,
    Extension,
    Silence,
}

fn renewal_decision(unrecouped: i32, commercial_success: u8) -> RenewalDecision {
    if unrecouped <= 0 && commercial_success >= DEAL_RENEWAL_DECENT_SALES_MIN_COMMERCIAL_SUCCESS {
        RenewalDecision::NewContract
    } else if unrecouped >= DEAL_RENEWAL_DEEP_RED_UNRECOUPED
        && commercial_success <= DEAL_RENEWAL_WEAK_SALES_MAX_COMMERCIAL_SUCCESS
    {
        RenewalDecision::Silence
    } else {
        RenewalDecision::Extension
    }
}

// ============================================================================
// L-BID (design §E): pricing an offer from a live label's state, and the
// bidding war when two-plus labels want the act. Everything here is a pure
// function of the `Label`'s public state (+ the act's fame) and the [tune]
// constants — no RNG — so the market's variety comes from the labels differing,
// not a die roll, and the pricing is directly unit-testable. The offer stream's
// only randomness stays the per-label coin flip and the paperwork rolls
// (`albums_required`, `term_weeks`) in `generate_priced_offers`.
// ============================================================================

/// Per-offer bidding bounds, parallel to the generated offers, carried into
/// `escalate_offers`: how far *this* label can raise its advance (its
/// affordability ceiling) and its royalty (its ceiling) under competition.
struct OfferBidBound {
    affordable: i64,
    royalty_ceiling: f32,
}

/// The most a label will front as a single advance out of its `capital` — the
/// hard "cannot advance money it does not have" ceiling (design §E). A label in
/// the red (`capital <= 0`) can afford nothing.
fn advance_affordable_ceiling(label: &Label) -> i64 {
    (label.capital.max(0) as f64 * f64::from(ADVANCE_CAPITAL_FRACTION)) as i64
}

/// The marquee advance a label's *stature* justifies (`financial_power³ × k`) —
/// what it would offer a fame-100 act before the affordability bound bites.
fn advance_stature(label: &Label) -> i64 {
    let fp = i64::from(label.financial_power);
    ((fp * fp * fp) as f64 * f64::from(ADVANCE_STATURE_PER_FP_CUBED)) as i64
}

/// Fame's linear pull on the advance: `ADVANCE_FAME_FLOOR` at fame 0 rising to
/// 1.0 at fame 100 — an unknown draws a fraction of the marquee figure, a
/// superstar the whole of it (design §E).
fn fame_advance_frac(fame: u8) -> f32 {
    ADVANCE_FAME_FLOOR + (1.0 - ADVANCE_FAME_FLOOR) * (f32::from(fame) / 100.0)
}

/// An offer's advance, priced from live state (design §E): the stature figure
/// trimmed by the act's heat, then bounded by what the label can actually afford
/// — so a hot act draws a bigger advance than an unknown, and a broke/lean label
/// cannot advance money it does not have. Pure and deterministic.
pub(super) fn priced_advance(label: &Label, fame: u8) -> u32 {
    let desired = advance_stature(label) as f64 * f64::from(fame_advance_frac(fame));
    let affordable = advance_affordable_ceiling(label) as f64;
    desired.min(affordable).max(0.0).round() as u32
}

/// The reach-set share of the royalty: a low-reach label competes on royalty, a
/// high-reach major pays the least (design §E). Runs from `ROYALTY_REACH_MAX` at
/// reach 0 down to `ROYALTY_REACH_MIN` at reach 100.
fn royalty_from_reach(market_reach: u8) -> f32 {
    let reach = f32::from(market_reach) / 100.0;
    ROYALTY_REACH_MAX - reach * (ROYALTY_REACH_MAX - ROYALTY_REACH_MIN)
}

/// An offer's royalty, priced from the label's live reach plus a hot act's small
/// premium (design §E), clamped to the sane band. Pure and deterministic.
pub(super) fn priced_royalty(label: &Label, fame: u8) -> f32 {
    let base = royalty_from_reach(label.market_reach);
    let fame_bonus = ROYALTY_FAME_BONUS_MAX * (f32::from(fame) / 100.0);
    (base + fame_bonus).clamp(ROYALTY_REACH_MIN, ROYALTY_HARD_CAP)
}

/// The highest royalty a label will go to under a bidding war: its reach-set
/// base plus the competitive premium, hard-capped. Royalty is the cheap lever a
/// poorer label can still pull when it cannot match a rival's advance (§E).
fn royalty_ceiling(label: &Label) -> f32 {
    (royalty_from_reach(label.market_reach) + ROYALTY_BID_PREMIUM).min(ROYALTY_HARD_CAP)
}

/// The bidding war (design §E): with two-plus offers on the table each label
/// escalates toward the best competing terms — advance toward the top advance
/// (bounded by its own affordability, so a broke label cannot outbid a flush
/// one) and royalty toward the top royalty (bounded by its royalty ceiling). A
/// single offer, or none, is left untouched (no competition, no war). Pure and
/// deterministic; `bounds` is parallel to `offers`.
fn escalate_offers(offers: &mut [PotentialDealOffer], bounds: &[OfferBidBound]) {
    if offers.len() < 2 {
        return;
    }
    let top_advance = offers.iter().map(|o| o.advance).max().unwrap_or(0);
    let top_royalty = offers.iter().map(|o| o.royalty_rate).fold(0.0f32, f32::max);
    for (offer, bound) in offers.iter_mut().zip(bounds) {
        // Advance closes a fraction of the gap to the top bid, capped by means.
        let gap = f64::from(top_advance.saturating_sub(offer.advance));
        let target = f64::from(offer.advance) + gap * f64::from(BID_ESCALATION_FRACTION);
        let capped = target.min(bound.affordable.max(0) as f64).max(0.0);
        offer.advance = capped.round() as u32;
        // Royalty closes a fraction of the gap to the top rate, capped by the
        // label's ceiling.
        let rgap = (top_royalty - offer.royalty_rate).max(0.0);
        let rtarget = offer.royalty_rate + rgap * BID_ESCALATION_FRACTION;
        offer.royalty_rate = rtarget.min(bound.royalty_ceiling);
    }
}

/// The tier label + interest threshold for a live label's tier (design §E). The
/// thresholds still come from `record_labels.json`'s `label_requirements` (they
/// are a per-tier bar on the *act*, not a per-label price), keyed off the
/// label's current `tier`.
fn tier_interest(
    tier: LabelTier,
    labels_data: &RecordLabelsData,
) -> (&'static str, &InterestThreshold) {
    match tier {
        LabelTier::Major => (
            "Major",
            &labels_data
                .label_requirements
                .major_label_interest_threshold,
        ),
        LabelTier::Independent => (
            "Independent",
            &labels_data
                .label_requirements
                .independent_label_interest_threshold,
        ),
        LabelTier::Boutique => (
            "Boutique",
            &labels_data
                .label_requirements
                .boutique_label_interest_threshold,
        ),
    }
}

/// The tiered base chance a label of the given tier tables an offer, by the
/// act's fame — the same curve the legacy JSON stream used, so a fresh 0.8
/// world's scouting cadence matches the pre-L-BID one before genre fit and
/// bidding layer on top.
fn tier_offer_chance(tier_name: &str, fame: u8) -> f64 {
    match tier_name {
        "Major" => {
            if fame > 70 {
                0.30
            } else if fame > 50 {
                0.20
            } else {
                0.10
            }
        }
        "Independent" => {
            if fame > 40 {
                0.40
            } else if fame > 20 {
                0.25
            } else {
                0.15
            }
        }
        "Boutique" => {
            if fame > 10 {
                0.50
            } else {
                0.20
            }
        }
        _ => 0.10,
    }
}

/// Albums a deal of the given tier asks for (the legacy per-tier ranges).
fn albums_required_for_tier(tier_name: &str, rng: &mut impl Rng) -> u8 {
    match tier_name {
        "Major" => rng.gen_range(2..=4),
        "Independent" => rng.gen_range(1..=3),
        "Boutique" => rng.gen_range(1..=2),
        _ => 2,
    }
}

/// The `RecordLabel` snapshot an offer carries for the deal paperwork and the
/// deal-picker UI. The numeric standing fields are read from the *live* label
/// (so a signed deal snapshots the label's current reach — the sales-fix
/// invariant that a release freezes its own reach still holds downstream), while
/// the flavor fields (grade, reputation, specialty strings, and the now-unused
/// advance_range/royalty_rate) come from the matching JSON row when present. A
/// label with no JSON row (e.g. one L-MNA founds at runtime) still yields a
/// coherent snapshot from live state alone.
fn label_snapshot(label: &Label, labels_data: &RecordLabelsData) -> RecordLabel {
    let json = find_json_row(labels_data, &label.name);
    RecordLabel {
        name: label.name.clone(),
        grade: json.map(|j| j.grade.clone()).unwrap_or_default(),
        market_reach: label.market_reach,
        financial_power: label.financial_power,
        artist_development: label.artist_development,
        creative_freedom: label.creative_freedom,
        royalty_rate: json.map_or(0, |j| j.royalty_rate),
        advance_range: json.map_or([0, 0], |j| j.advance_range),
        specialty_genres: json.map(|j| j.specialty_genres.clone()).unwrap_or_default(),
        founded: u32::from(label.founded_year),
        reputation: json.map(|j| j.reputation.clone()).unwrap_or_default(),
    }
}

/// The `record_labels.json` row for a label by name, across all three buckets —
/// the flavor source for `label_snapshot`. `None` for a label with no seed row.
fn find_json_row<'a>(labels_data: &'a RecordLabelsData, name: &str) -> Option<&'a RecordLabel> {
    labels_data
        .major_labels
        .iter()
        .chain(labels_data.independent_labels.iter())
        .chain(labels_data.boutique_labels.iter())
        .find(|l| l.name == name)
}

// ============================================================================
// L-BID (design §E) tests. The pricing and bidding helpers are pure functions
// of a label's public state, so they are exercised directly here (co-located so
// they can reach the module-private helpers); the market itself is exercised
// through `generate_deal_offers` on a real seeded world.
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::music::{Release, ReleaseType};
    use crate::game::world::LabelStrategy;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A live `Label` with the given standing, everything else neutral. Genres
    /// are left empty so genre fit does not perturb the pricing under test.
    fn label_fixture(
        capital: i64,
        financial_power: u8,
        market_reach: u8,
        tier: LabelTier,
    ) -> Label {
        Label {
            id: 0,
            name: "Fixture Records".to_string(),
            tier,
            genres: Vec::new(),
            founded_year: 1975,
            capital,
            market_reach,
            financial_power,
            artist_development: 50,
            creative_freedom: 50,
            strategy: LabelStrategy::Balanced,
            status: LabelStatus::Active,
            roster: Vec::new(),
            hit_window: Vec::new(),
            ledger_ytd: 0,
            weeks_below_floor: 0,
            acquired_by: None,
        }
    }

    fn record(release_type: ReleaseType) -> Release {
        Release {
            id: 0,
            name: "Test Record".to_string(),
            release_type,
            release_quality: 50,
            week_released: 0,
            songs_involved_quality_avg: 50,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: None,
            copies_pressed: 0,
            copies_sold: 0,
            peak_chart_position: None,
            singles_cut: 0,
            certified: 0,
            distribution_channel: None,
            label_market_reach: None,
        }
    }

    fn act(fame: u8, singles: usize, albums: usize) -> Band {
        Band {
            fame,
            singles_released: (0..singles).map(|_| record(ReleaseType::Single)).collect(),
            albums_released: (0..albums).map(|_| record(ReleaseType::Album)).collect(),
            ..Band::default()
        }
    }

    fn offer_with(advance: u32, royalty_rate: f32) -> PotentialDealOffer {
        PotentialDealOffer {
            label_name: "x".to_string(),
            label_tier: "Independent".to_string(),
            advance,
            royalty_rate,
            albums_required: 1,
            original_label_data: RecordLabel {
                name: "x".to_string(),
                grade: String::new(),
                market_reach: 50,
                financial_power: 50,
                artist_development: 50,
                creative_freedom: 50,
                royalty_rate: 0,
                advance_range: [0, 0],
                specialty_genres: Vec::new(),
                founded: 1975,
                reputation: String::new(),
            },
            expires_week: None,
            term_weeks: 90,
            carry_forward_unrecouped: 0,
        }
    }

    // --- pricing from state -------------------------------------------------

    #[test]
    fn a_hotter_act_commands_a_bigger_advance_and_better_royalty() {
        // Same label; the only thing that changes is the act's fame.
        let label = label_fixture(5_000_000, 60, 55, LabelTier::Independent);
        assert!(
            priced_advance(&label, 100) > priced_advance(&label, 20),
            "a fame-100 act must draw a bigger advance than a fame-20 unknown"
        );
        assert!(
            priced_royalty(&label, 100) >= priced_royalty(&label, 20),
            "a hot act commands at least as good a royalty"
        );
    }

    #[test]
    fn an_advance_can_never_exceed_what_the_label_can_afford() {
        // A label cannot advance money it does not have (design §E): however
        // hot the act, the advance stays inside the affordability ceiling.
        for capital in [0i64, 10_000, 250_000, 5_000_000, 30_000_000] {
            let label = label_fixture(capital, 90, 80, LabelTier::Major);
            let advance = i64::from(priced_advance(&label, 100));
            let ceiling = advance_affordable_ceiling(&label);
            assert!(
                advance <= ceiling,
                "advance {advance} exceeded affordability ceiling {ceiling} at capital {capital}"
            );
            assert!(
                advance <= capital.max(0),
                "a label advanced money it did not have"
            );
        }
    }

    #[test]
    fn a_broke_label_cannot_outbid_a_flush_one() {
        // Identical stature and reach; only the balance sheet differs. The
        // broke label's advance is squeezed by affordability and loses.
        let flush = label_fixture(20_000_000, 70, 65, LabelTier::Major);
        let broke = label_fixture(60_000, 70, 65, LabelTier::Major);
        let flush_adv = priced_advance(&flush, 95);
        let broke_adv = priced_advance(&broke, 95);
        assert!(
            flush_adv > broke_adv,
            "the flush label ({flush_adv}) must outbid the broke one ({broke_adv})"
        );
        // The broke label's advance is exactly its affordability bound (its
        // stature would justify far more, but it cannot pay it).
        assert_eq!(broke_adv as i64, advance_affordable_ceiling(&broke));
    }

    #[test]
    fn royalty_tracks_reach_a_major_pays_less_than_a_boutique() {
        let major = label_fixture(20_000_000, 90, 95, LabelTier::Major);
        let boutique = label_fixture(200_000, 25, 30, LabelTier::Boutique);
        let major_r = priced_royalty(&major, 50);
        let boutique_r = priced_royalty(&boutique, 50);
        assert!(
            boutique_r > major_r,
            "a low-reach boutique ({boutique_r:.3}) competes on royalty vs a major ({major_r:.3})"
        );
        for r in [major_r, boutique_r] {
            assert!(
                (ROYALTY_REACH_MIN..=ROYALTY_HARD_CAP).contains(&r),
                "royalty {r:.3} out of the sane band"
            );
        }
    }

    // --- bidding escalation -------------------------------------------------

    #[test]
    fn a_single_offer_is_left_untouched_by_bidding() {
        let mut offers = vec![offer_with(50_000, 0.20)];
        let bounds = vec![OfferBidBound {
            affordable: 1_000_000,
            royalty_ceiling: 0.30,
        }];
        escalate_offers(&mut offers, &bounds);
        assert_eq!(offers[0].advance, 50_000, "no rival, no escalation");
        assert_eq!(offers[0].royalty_rate, 0.20);
    }

    #[test]
    fn bidding_raises_terms_toward_the_top_bounded_by_each_labels_means() {
        // A flush leader at $500k / 12% and a lean challenger at $20k / 26%.
        let mut offers = vec![offer_with(500_000, 0.12), offer_with(20_000, 0.26)];
        let bounds = vec![
            // Leader: deep pockets, low royalty ceiling.
            OfferBidBound {
                affordable: 5_000_000,
                royalty_ceiling: 0.15,
            },
            // Challenger: shallow pockets ($40k), high royalty ceiling.
            OfferBidBound {
                affordable: 40_000,
                royalty_ceiling: 0.31,
            },
        ];
        escalate_offers(&mut offers, &bounds);

        // The leader's advance is already the top, so it does not rise; its
        // royalty ticks up halfway toward the challenger's 26%, capped at 15%.
        assert_eq!(offers[0].advance, 500_000);
        assert!(
            (offers[0].royalty_rate - 0.15).abs() < 1e-6,
            "leader royalty capped at its 15% ceiling"
        );

        // The challenger reaches toward the $500k top but is capped at its own
        // $40k affordability — it cannot outbid the flush leader on advance.
        assert_eq!(
            offers[1].advance, 40_000,
            "challenger advance capped by affordability"
        );
        assert!(
            offers[1].advance < offers[0].advance,
            "the broke challenger loses the advance war"
        );
        // Its royalty was already the top, so it holds (a touch under its ceiling).
        assert!((offers[1].royalty_rate - 0.26).abs() < 1e-6);
    }

    // --- the market on a real seeded world ---------------------------------

    /// Total offers drawn and the richest advance seen for an act, aggregated
    /// over many independent offer windows so the coin-flips average out.
    fn market_sample(world: &GameWorld, data: &GameDataFiles, band: &Band) -> (usize, u32) {
        let mut total = 0usize;
        let mut top_advance = 0u32;
        for seed in 0..64u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let offers = world.generate_deal_offers(band, data, &mut rng);
            total += offers.len();
            top_advance = top_advance.max(offers.iter().map(|o| o.advance).max().unwrap_or(0));
        }
        (total, top_advance)
    }

    #[test]
    fn a_hot_act_draws_more_and_richer_offers_than_an_unknown() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(42));

        let hot = act(100, 3, 3); // a superstar with a deep catalog
        let unknown = act(20, 2, 0); // a working act, barely on the radar

        let (hot_total, hot_top) = market_sample(&world, &data, &hot);
        let (cold_total, cold_top) = market_sample(&world, &data, &unknown);

        assert!(
            hot_total > cold_total,
            "a hot act should draw more offers ({hot_total}) than an unknown ({cold_total})"
        );
        assert!(
            hot_top > cold_top,
            "and a richer top advance ({hot_top}) than the unknown's ({cold_top})"
        );
    }

    #[test]
    fn the_priced_market_replays_identically_for_a_seed() {
        let data = GameDataFiles::load().expect("data files present");
        let world = GameWorld::new(&data, &mut StdRng::seed_from_u64(7));
        let band = act(80, 4, 3);

        let run = |seed: u64| {
            let mut rng = StdRng::seed_from_u64(seed);
            world
                .generate_deal_offers(&band, &data, &mut rng)
                .into_iter()
                .map(|o| (o.label_name, o.advance, o.royalty_rate, o.term_weeks))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(2025), run(2025), "same seed replays the same market");
    }

    #[test]
    fn an_industry_with_no_cash_makes_no_offers() {
        // Every label bled dry (capital 0) → the affordability gate silences
        // the whole market, however hot the act (design §E).
        let data = GameDataFiles::load().expect("data files present");
        let mut world = GameWorld::new(&data, &mut StdRng::seed_from_u64(3));
        for label in world.labels.values_mut() {
            label.capital = 0;
        }
        let band = act(100, 4, 4);
        for seed in 0..40u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            assert!(
                world
                    .generate_deal_offers(&band, &data, &mut rng)
                    .is_empty(),
                "a cashless industry cannot front advances"
            );
        }
    }
}
