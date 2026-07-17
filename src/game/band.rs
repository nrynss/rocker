use super::genre::MusicGenre;
use super::music::{Release, Song}; // Import new structs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub name: String,
    /// The sound the band plays. Saves from before genres existed load as Rock.
    #[serde(default)]
    pub genre: MusicGenre,
    pub fame: u8, // 0-100
    /// Highest fame the band has ever reached — the peak that its permanent
    /// floors are earned against (see fame gravity, design §C). Old saves
    /// default to 0; every read lifts it to current fame so a loaded career
    /// never forgets a peak it already stood on.
    #[serde(default)]
    pub peak_fame: u8,
    pub skill: u8, // 0-100
    pub unreleased_songs: Vec<Song>,
    pub singles_released: Vec<Release>,
    pub albums_released: Vec<Release>,
    pub members: Vec<BandMember>,
    pub record_deal: Option<RecordDeal>,
    pub reputation: BandReputation,
    /// Weeks left before a label will make the band a new offer — imposed
    /// after a breach (design §E-4). `0` means no cooldown, the default for
    /// every band that has never breached a deal. Same field name FUTURE §3
    /// plans a Manager around, so a later cycle inherits it.
    #[serde(default)]
    pub deal_cooldown: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandMember {
    pub name: String,
    pub instrument: Instrument,
    pub skill: u8,
    pub loyalty: u8, // 0-100, affects chance of leaving
    pub drug_problem: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instrument {
    Guitar,
    Bass,
    Drums,
    Keyboard,
    Vocals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordDeal {
    pub label_name: String,
    pub label_tier: String, // e.g., "Major", "Independent", "Boutique"
    pub advance: u32,
    pub royalty_rate: f32, // Percentage
    pub albums_required: u8,
    pub albums_delivered: u8,
    /// The label's distribution muscle (0-100), taken from the label data.
    #[serde(default = "default_market_reach")]
    pub market_reach: u8,
    /// Label recoupment ledger (design §E-2). The label's outlay the player
    /// must repay out of royalties before a cent reaches the band: the advance
    /// at signing, then pressing + promo at every release under the deal.
    /// Royalty income pays this down first; while it is positive the player
    /// earns nothing from the label's sales. Old saves default to 0 (nothing
    /// owed — a fully recouped or advance-free deal).
    ///
    /// NOTE (M9 boundary): recoupment is tracked for the *active* deal only.
    /// The design says the balance survives the deal, but today
    /// `fulfill_album_obligation` clears `record_deal` (and this ledger with
    /// it) the moment the album count is met. Making recoupment outlive the
    /// deal is M9's job (deal lifecycle); M5 deliberately does not touch it.
    ///
    /// RESOLVED (M9): the deal no longer clears the instant albums are met —
    /// see `fulfill_album_obligation` below. `unrecouped` now survives every
    /// early album delivery and is only ever zeroed at real deal-end (free
    /// agency pays nothing further; breach writes off whatever remains).
    #[serde(default)]
    pub unrecouped: i32,
    /// The week this deal was signed, stamped at signing (design §E-4).
    /// `0` alongside `term_weeks == 0` marks a deal that predates M9's term
    /// system — see `term_weeks` for the legacy policy.
    #[serde(default)]
    pub signed_week: u32,
    /// Contract length in weeks, generated at offer time by label tier
    /// (design §E-4: Boutique 52-78, Independent 78-104, Major 104-156).
    ///
    /// Legacy policy: `0` is the sentinel for "no term system yet" — every
    /// deal signed before M9 loads with `term_weeks: 0`. Such a deal is
    /// treated as **term already served** (so free agency still fires the
    /// instant the album count is met, exactly the pre-M9 behavior) and as
    /// **never breachable** (a clock that didn't exist can't run out) — see
    /// `term_served` / `term_expired`.
    #[serde(default)]
    pub term_weeks: u16,
}

fn default_market_reach() -> u8 {
    50
}

impl RecordDeal {
    /// The later half of the free-agency rule (design §E-4): whether the
    /// term has run its course as of `current_week`. A legacy deal
    /// (`term_weeks == 0`) is always considered served — see the field's
    /// doc comment for the policy this preserves.
    pub fn term_served(&self, current_week: u32) -> bool {
        self.term_weeks == 0 || current_week >= self.term_end_week()
    }

    /// Whether the term has expired in the *breach* sense: a real
    /// (non-legacy) term whose clock ran out. Distinct from `term_served` —
    /// a legacy deal is always "served" (so albums alone free it) but can
    /// never "expire" into a breach that didn't exist when it was signed.
    pub fn term_expired(&self, current_week: u32) -> bool {
        self.term_weeks > 0 && current_week >= self.term_end_week()
    }

    /// The week the term runs out. Meaningless (reads as `signed_week`) for
    /// a legacy deal with `term_weeks == 0` — callers must check
    /// `term_weeks > 0` (via `term_expired`) before treating this as a
    /// real deadline.
    pub fn term_end_week(&self) -> u32 {
        self.signed_week.saturating_add(u32::from(self.term_weeks))
    }

    /// Whether albums are still owed under the deal.
    pub fn albums_owed(&self) -> bool {
        self.albums_delivered < self.albums_required
    }

    /// The renewal window (design §E-4): open when all albums are
    /// delivered, the deal carries a real term, and `current_week` falls
    /// within `window_weeks` of the term's expiry (but hasn't passed it —
    /// once the term is actually up, free agency or breach has already
    /// resolved the deal).
    pub fn renewal_window_open(&self, current_week: u32, window_weeks: u32) -> bool {
        if self.term_weeks == 0 || self.albums_owed() {
            return false;
        }
        let term_end = self.term_end_week();
        current_week < term_end && current_week.saturating_add(window_weeks) >= term_end
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandReputation {
    pub critical_acclaim: u8,   // 0-100
    pub commercial_success: u8, // 0-100
    pub live_performance: u8,   // 0-100
    pub media_presence: u8,     // 0-100
}

impl Default for Band {
    fn default() -> Self {
        Self {
            name: String::new(),
            genre: MusicGenre::Rock,
            fame: 0,
            peak_fame: 0,
            skill: 20,
            unreleased_songs: Vec::new(),
            singles_released: Vec::new(),
            albums_released: Vec::new(),
            members: vec![
                BandMember {
                    name: "Dave".to_string(),
                    instrument: Instrument::Guitar,
                    skill: 25,
                    loyalty: 75,
                    drug_problem: false,
                },
                BandMember {
                    name: "Sarah".to_string(),
                    instrument: Instrument::Bass,
                    skill: 20,
                    loyalty: 80,
                    drug_problem: false,
                },
                BandMember {
                    name: "Mike".to_string(),
                    instrument: Instrument::Drums,
                    skill: 30,
                    loyalty: 70,
                    drug_problem: false,
                },
            ],
            record_deal: None,
            reputation: BandReputation::default(),
            deal_cooldown: 0,
        }
    }
}

impl Default for BandReputation {
    fn default() -> Self {
        Self {
            critical_acclaim: 10,
            commercial_success: 5,
            live_performance: 15,
            media_presence: 0,
        }
    }
}

impl Band {
    /// The peak fame the band has stood on, robust against pre-0.6 saves that
    /// default `peak_fame` to 0: it can never read lower than current fame.
    pub fn effective_peak_fame(&self) -> u8 {
        self.peak_fame.max(self.fame)
    }

    /// Add fame the one true way. While the band is climbing back toward a
    /// peak it has already reached, the gain is doubled (the comeback rule,
    /// design §C); the result is clamped to `MAX_FAME` and the peak updated.
    /// Fame *losses* (idle decay, bad events) must not route through here.
    pub fn gain_fame(&mut self, amount: u8) -> u8 {
        self.gain_fame_capped(amount, crate::game::constants::MAX_FAME)
    }

    /// Like [`Band::gain_fame`], but the result also respects a ceiling —
    /// the live-fame caps: comeback doubling never carries fame past `cap`.
    /// A cap already at or below current fame never reduces fame.
    ///
    /// Returns the fame actually applied — after comeback doubling and the
    /// caps — which is what any "fame +N" log line must report: the raw
    /// `amount` understates a comeback gain 2× and overstates a capped one.
    pub fn gain_fame_capped(&mut self, amount: u8, cap: u8) -> u8 {
        let before = self.fame;
        let ceiling = cap.max(self.fame);
        let peak = self.effective_peak_fame();
        let multiplier = if self.fame < peak {
            u16::from(crate::game::constants::FAME_COMEBACK_MULTIPLIER)
        } else {
            1
        };
        let gained = u16::from(amount) * multiplier;
        self.fame = (u16::from(self.fame) + gained)
            .min(u16::from(ceiling))
            .min(u16::from(crate::game::constants::MAX_FAME)) as u8;
        self.peak_fame = peak.max(self.fame);
        self.fame - before
    }

    pub fn get_fame_level(&self) -> &str {
        match self.fame {
            0..=10 => "Unknown",
            11..=25 => "Local scene",
            26..=40 => "Regional",
            41..=60 => "National",
            61..=80 => "International",
            81..=95 => "Superstar",
            _ => "Legend",
        }
    }

    pub fn get_skill_level(&self) -> &str {
        match self.skill {
            0..=20 => "Amateur",
            21..=40 => "Competent",
            41..=60 => "Good",
            61..=80 => "Professional",
            81..=95 => "Expert",
            _ => "Virtuoso",
        }
    }

    pub fn average_member_skill(&self) -> u8 {
        if self.members.is_empty() {
            return 0;
        }
        let total: u32 = self.members.iter().map(|m| m.skill as u32).sum();
        (total / self.members.len() as u32) as u8
    }

    pub fn band_morale(&self) -> u8 {
        if self.members.is_empty() {
            return 0;
        }
        let total: u32 = self.members.iter().map(|m| m.loyalty as u32).sum();
        (total / self.members.len() as u32) as u8
    }

    pub fn has_record_deal(&self) -> bool {
        self.record_deal.is_some()
    }

    pub fn can_record_album(&self) -> bool {
        self.unreleased_songs.len() >= crate::data::constants::MIN_ALBUM_SONGS as usize // Assuming MIN_ALBUM_SONGS is available
    }

    pub fn can_record_single(&self) -> bool {
        !self.unreleased_songs.is_empty()
    }

    pub fn total_releases(&self) -> usize {
        // Changed to usize to match Vec::len()
        self.singles_released.len() + self.albums_released.len()
    }

    /// Whether the band's genre answers to any of the given labels. Matching
    /// is tolerant of surface form: "Hair Metal" and "hair_metal" both match
    /// Metal (via [`MusicGenre::aliases`]), "Grunge" matches Alternative, and
    /// so on — so historical events can name sub-genres the coarse enum folds
    /// together. A label that maps to no genre (e.g. "Folk Rock") simply
    /// matches nothing.
    pub fn dominant_genres_match(&self, target_genres: &[&str]) -> bool {
        fn normalize(s: &str) -> String {
            s.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect()
        }
        let keys: Vec<String> = std::iter::once(self.genre.name())
            .chain(self.genre.aliases().iter().copied())
            .map(normalize)
            .collect();
        target_genres
            .iter()
            .any(|target| keys.contains(&normalize(target)))
    }

    pub fn current_deal(&self) -> Option<&RecordDeal> {
        self.record_deal.as_ref()
    }

    pub fn sign_deal(&mut self, deal: RecordDeal) {
        self.record_deal = Some(deal);
    }

    /// An album just released under the deal. Design §E-4: free agency
    /// comes at the LATER of all albums delivered and the term served — an
    /// act that finishes its albums early stays on the roster (releases
    /// still go through the label, single-cuts and recoupment continue)
    /// until the term is also served. The album count crossing the
    /// requirement is a one-shot event: further albums delivered while the
    /// term runs on don't re-announce it.
    pub fn fulfill_album_obligation(&mut self, current_week: u32) -> DealCompletionOutcome {
        let Some(deal) = &mut self.record_deal else {
            return DealCompletionOutcome::StillActive;
        };
        let already_delivered = !deal.albums_owed();
        deal.albums_delivered = deal.albums_delivered.saturating_add(1);
        if already_delivered || deal.albums_owed() {
            // Either this crossing was already announced (the band keeps
            // delivering albums while the term runs on), or the count still
            // falls short — nothing deal-ending happens on this release.
            return DealCompletionOutcome::StillActive;
        }
        // This release just crossed the album requirement for the first time.
        if deal.term_served(current_week) {
            let label_name = deal.label_name.clone();
            self.record_deal = None;
            DealCompletionOutcome::FreeAgent { label_name }
        } else {
            let label_name = deal.label_name.clone();
            let term_end_week = deal.term_end_week();
            DealCompletionOutcome::ObligationDelivered {
                label_name,
                term_end_week,
            }
        }
    }

    /// Weekly term-clock decrement (design §E-4): ticks down independent of
    /// any release, so a cooldown imposed by a breach actually expires.
    pub fn tick_deal_cooldown(&mut self) {
        self.deal_cooldown = self.deal_cooldown.saturating_sub(1);
    }

    /// Weekly breach check (design §E-4): the term's clock, checked
    /// regardless of whether anything was released this week. Fires only
    /// when a *real* term (`term_weeks > 0`) has expired with albums still
    /// owed — a legacy deal (`term_weeks == 0`) can never breach. On
    /// breach: the deal ends, `commercial_success` takes the hit, any
    /// remaining `unrecouped` balance is written off, and a cooldown blocks
    /// new offers. Returns `None` on every other week.
    pub fn check_term_breach(&mut self, current_week: u32) -> Option<BreachOutcome> {
        let owed_at_breach = {
            let deal = self.record_deal.as_ref()?;
            deal.term_expired(current_week) && deal.albums_owed()
        };
        if !owed_at_breach {
            return None;
        }
        let deal = self.record_deal.take().expect("checked Some above");
        self.reputation.commercial_success = self
            .reputation
            .commercial_success
            .saturating_sub(crate::game::constants::DEAL_BREACH_REPUTATION_HIT);
        self.deal_cooldown = crate::game::constants::DEAL_BREACH_COOLDOWN_WEEKS;
        Some(BreachOutcome {
            label_name: deal.label_name,
            written_off: deal.unrecouped,
        })
    }

    /// Weekly free-agency check for a deal that already delivered its
    /// albums early (`ObligationDelivered`, from `fulfill_album_obligation`)
    /// and has just been waiting out the calendar (design §E-4). Without
    /// this, a deal that finished its albums early would only ever clear on
    /// *another* release — but there's no reason to release more once
    /// nothing is owed, so the term running out must free the band on its
    /// own, checked every week alongside the breach clock. Returns the
    /// label name when the deal ends this way; `None` on every other week
    /// (including every legacy deal already cleared instantly by
    /// `fulfill_album_obligation`, since `term_served` is trivially true for
    /// them the moment albums are met).
    pub fn check_term_served_free_agency(&mut self, current_week: u32) -> Option<String> {
        let ends = {
            let deal = self.record_deal.as_ref()?;
            !deal.albums_owed() && deal.term_served(current_week)
        };
        if !ends {
            return None;
        }
        let deal = self.record_deal.take().expect("checked Some above");
        Some(deal.label_name)
    }
}

/// What happened when an album released under a deal (design §E-4).
#[derive(Debug, Clone, PartialEq)]
pub enum DealCompletionOutcome {
    /// Nothing deal-ending this release: albums are still owed, or the
    /// album-count crossing already fired on an earlier release.
    StillActive,
    /// Albums delivered, but the term runs on — the band stays signed.
    ObligationDelivered {
        label_name: String,
        term_end_week: u32,
    },
    /// Free agency: both albums delivered and the term served. The deal is
    /// cleared.
    FreeAgent { label_name: String },
}

/// The result of a term expiring with albums still owed (design §E-4).
#[derive(Debug, Clone, PartialEq)]
pub struct BreachOutcome {
    pub label_name: String,
    /// Whatever `unrecouped` balance remained, written off (never charged
    /// to the player — the label simply eats the loss and remembers).
    pub written_off: i32,
}

impl std::fmt::Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instrument::Guitar => write!(f, "Guitar"),
            Instrument::Bass => write!(f, "Bass"),
            Instrument::Drums => write!(f, "Drums"),
            Instrument::Keyboard => write!(f, "Keyboard"),
            Instrument::Vocals => write!(f, "Vocals"),
        }
    }
}
