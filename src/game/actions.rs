//! The player's weekly actions: what each choice costs, what it does,
//! and the `execute_action` dispatch that routes a `GameAction` here.

use super::*;

impl Game {
    fn calculate_songwriting_quality(&self, rng: &mut impl Rng) -> u8 {
        let mut quality = QUALITY_BASE_SONGWRITING as f32;
        let mut player_bonus = 0.0;

        // Player energy bonus
        if self.player.energy > 70 {
            player_bonus += 5.0;
        } else if self.player.energy > 40 {
            player_bonus += 2.0;
        }

        // Player stress bonus (low stress is good)
        if self.player.stress < 30 {
            player_bonus += 5.0;
        } else if self.player.stress < 60 {
            player_bonus += 2.0;
        }

        // Band member skill bonus
        player_bonus += (self.band.average_member_skill() / 15) as f32;

        quality += player_bonus.min(QUALITY_SONGWRITING_MAX_BONUS_PLAYER_STATS as f32);

        // Random variation
        let random_offset = rng.gen_range(0..=QUALITY_SONGWRITING_RANDOM_VARIATION) as i8
            - (QUALITY_SONGWRITING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

        quality.clamp(1.0, 100.0) as u8
    }

    fn get_selected_songs_for_release(
        &mut self,
        count: usize,
    ) -> Result<(Vec<music::Song>, u8), String> {
        if self.band.unreleased_songs.len() < count {
            return Err(format!(
                "Not enough unreleased songs. Need {}, have {}.",
                count,
                self.band.unreleased_songs.len()
            ));
        }

        let selected_songs: Vec<music::Song> = self
            .band
            .unreleased_songs
            .drain((self.band.unreleased_songs.len() - count)..)
            .collect();

        if selected_songs.is_empty() && count > 0 {
            return Err("No songs were selected, though count was > 0.".to_string());
        }
        if count == 0 {
            return Ok((Vec::new(), 0));
        }

        let total_quality: u32 = selected_songs
            .iter()
            .map(|s| s.songwriting_quality as u32)
            .sum();
        let avg_quality = (total_quality / selected_songs.len() as u32) as u8;

        Ok((selected_songs, avg_quality))
    }

    fn calculate_release_quality(&self, avg_song_quality: u8, rng: &mut impl Rng) -> u8 {
        let mut quality = (QUALITY_BASE_RECORDING as f32 + avg_song_quality as f32) / 2.0;

        quality += (self.band.skill / 10) as f32;

        let mut player_bonus: f32 = 0.0;
        if self.player.energy > 70 {
            player_bonus += 3.0;
        } else if self.player.energy > 40 {
            player_bonus += 1.0;
        }
        if self.player.stress < 30 {
            player_bonus += 3.0;
        } else if self.player.stress < 60 {
            player_bonus += 1.0;
        }
        quality += player_bonus.min(QUALITY_RECORDING_MAX_BONUS_PLAYER_STATS as f32);

        let random_offset = rng.gen_range(0..=QUALITY_RECORDING_RANDOM_VARIATION) as i8
            - (QUALITY_RECORDING_RANDOM_VARIATION / 2) as i8;
        quality += random_offset as f32;

        quality.clamp((avg_song_quality as f32 / 2.0).max(1.0), 100.0) as u8
    }

    fn action_laze_around(&mut self) -> Result<(), String> {
        self.player.energy = (self.player.energy + 20).min(constants::MAX_ENERGY);
        self.player.stress = self.player.stress.saturating_sub(10);
        self.log("😴 You took it easy this week — energy up, stress down.");
        Ok(())
    }

    fn action_write_songs(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        if self.player.energy < 20 {
            return Err("You're too tired to write songs!".to_string());
        }
        self.player.energy -= 20;

        let num_songs_to_write = rng.gen_range(1..=3);
        let mut titles = Vec::new();
        for _ in 0..num_songs_to_write {
            let quality = self.calculate_songwriting_quality(rng);
            let song_name = self.data_files.generate_song_title(rng);
            titles.push(format!("\"{}\"", song_name));
            self.band.unreleased_songs.push(music::Song {
                id: self.next_song_id,
                name: song_name,
                songwriting_quality: quality,
            });
            self.next_song_id += 1;
        }
        self.log(format!(
            "🎼 Wrote {} new song{}: {}",
            num_songs_to_write,
            if num_songs_to_write == 1 { "" } else { "s" },
            titles.join(", ")
        ));
        Ok(())
    }

    fn action_practice(&mut self) -> Result<(), String> {
        if self.player.energy < 15 {
            return Err("You're too tired to practice!".to_string());
        }
        self.player.energy -= 15;
        self.band.skill = (self.band.skill + 2).min(constants::MAX_SKILL);
        let skill = self.band.skill;
        self.log(format!(
            "🥁 A week in the rehearsal room — band skill is now {}%.",
            skill
        ));
        Ok(())
    }

    pub(super) fn action_record_single(
        &mut self,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if !self.band.can_record_single() {
            return Err("You need to write at least one song first!".to_string());
        }

        let recording_cost = self.recording_cost(&music::ReleaseType::Single);
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Single, pressing)?;
        let cost = recording_cost + pressing_cost;
        if !self.player.can_afford(cost) {
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent single costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record a single!", cost));
        }

        let (selected_songs, avg_song_quality) = self.get_selected_songs_for_release(1)?;
        if selected_songs.is_empty() {
            return Err("Failed to select a song for the single.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, rng);
        let release_name = format!("Single: {}", selected_songs[0].name);

        let new_release = music::Release {
            id: self.next_release_id,
            name: release_name,
            release_type: music::ReleaseType::Single,
            release_quality,
            week_released: self.week,
            songs_involved_quality_avg: avg_song_quality,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: Some(self.band.genre.clone()),
            copies_pressed: copies,
            copies_sold: 0,
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} and pressed {} copies for ${} — out in {} weeks.",
                name, recording_cost, copies, pressing_cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded '{}' for ${} — the label presses {} copies, out in {} weeks.",
                name, recording_cost, copies, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        self.apply_label_promo();
        Ok(())
    }

    fn action_record_album(
        &mut self,
        pressing: Option<usize>,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        if !self.band.can_record_album() {
            return Err(format!(
                "You need at least {} unreleased songs to record an album!",
                constants::MIN_ALBUM_SONGS
            ));
        }

        let recording_cost = self.recording_cost(&music::ReleaseType::Album);
        let (copies, pressing_cost) = self.plan_pressing(&music::ReleaseType::Album, pressing)?;
        let cost = recording_cost + pressing_cost;
        if !self.player.can_afford(cost) {
            if pressing_cost > 0 {
                return Err(format!(
                    "An independent album costs ${} — ${} studio time plus ${} to press {} copies!",
                    cost, recording_cost, pressing_cost, copies
                ));
            }
            return Err(format!("You need at least ${} to record an album!", cost));
        }

        let (selected_songs, avg_song_quality) =
            self.get_selected_songs_for_release(constants::MIN_ALBUM_SONGS as usize)?;
        if selected_songs.len() < constants::MIN_ALBUM_SONGS as usize {
            return Err("Not enough songs selected for an album.".to_string());
        }
        self.player.spend_money(cost);

        let release_quality = self.calculate_release_quality(avg_song_quality, rng);
        let release_name = self.data_files.random_album_title(rng);

        let new_release = music::Release {
            id: self.next_release_id,
            name: release_name,
            release_type: music::ReleaseType::Album,
            release_quality,
            week_released: self.week,
            songs_involved_quality_avg: avg_song_quality,
            active_marketing: Vec::new(),
            marketing_level_achieved: 0,
            initial_sales_score: 0,
            total_income_generated: 0,
            genre: Some(self.band.genre.clone()),
            copies_pressed: copies,
            copies_sold: 0,
        };
        let name = new_release.name.clone();
        self.just_released_music.push(new_release);
        self.next_release_id += 1;
        if pressing_cost > 0 {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${} and pressed {} copies for ${} — out in {} weeks.",
                name, recording_cost, copies, pressing_cost, INITIAL_SALES_WINDOW_WEEKS
            ));
        } else {
            self.log(format!(
                "🎙️ Recorded the album '{}' for ${} — the label presses {} copies, out in {} weeks.",
                name, recording_cost, copies, INITIAL_SALES_WINDOW_WEEKS
            ));
        }
        self.apply_label_promo();

        if self.timeline.is_album_era() {
            self.band.fame = (self.band.fame + 3).min(constants::MAX_FAME);
            self.log(
                "📈 It's an album-oriented era — the announcement alone earns you buzz (+3 fame).",
            );
        }
        Ok(())
    }

    pub(super) fn action_start_marketing_campaign(
        &mut self,
        release_id: u32,
        campaign_type: MarketingCampaignType,
    ) -> Result<(), String> {
        if let Some(deal) = self.band.current_deal() {
            return Err(format!(
                "Promotion is {}'s job — their people are already on it.",
                deal.label_name
            ));
        }
        let spec = campaign_type.spec();
        if !self.player.can_afford(spec.cost) {
            return Err(format!(
                "Not enough money for a {} campaign. Need ${}.",
                spec.name, spec.cost
            ));
        }

        let current_week = self.week;
        // Find in just_released_music first, then in already released music
        let release = self
            .just_released_music
            .iter_mut()
            .find(|r| r.id == release_id)
            .or_else(|| {
                self.band
                    .singles_released
                    .iter_mut()
                    .find(|r| r.id == release_id)
            })
            .or_else(|| {
                self.band
                    .albums_released
                    .iter_mut()
                    .find(|r| r.id == release_id)
            })
            .ok_or_else(|| {
                format!(
                    "Release with ID {} not found to start marketing campaign.",
                    release_id
                )
            })?;

        release.active_marketing.push(ActiveMarketingCampaign {
            campaign_type,
            start_week: current_week,
            end_week: current_week + spec.duration_weeks,
            effectiveness_bonus: spec.effectiveness_bonus,
        });

        release.marketing_level_achieved = release
            .active_marketing
            .iter()
            .map(|c| c.effectiveness_bonus as u32)
            .sum::<u32>()
            .min(100) as u8;
        let release_name = release.name.clone();

        self.player.spend_money(spec.cost);
        self.log(format!(
            "📣 {} campaign launched for '{}' — ${}, runs {} weeks, +{} buzz.",
            spec.name, release_name, spec.cost, spec.duration_weeks, spec.effectiveness_bonus
        ));
        Ok(())
    }

    pub fn get_sorted_regions(&self) -> Vec<(String, String, String, u32, u8, u8)> {
        let mut result = Vec::new();
        let markets_data = &self.data_files.markets_data;

        let mut countries: Vec<String> = markets_data.markets.keys().cloned().collect();
        countries.sort();

        for country in countries {
            if let Some(c_market) = markets_data.markets.get(&country) {
                let mut regions: Vec<String> = c_market.regions.keys().cloned().collect();
                regions.sort();
                for r_key in regions {
                    if let Some(r_market) = c_market.regions.get(&r_key) {
                        let fame_req = if r_market.population < 3_000_000 {
                            25
                        } else if r_market.population < 7_000_000 {
                            35
                        } else if r_market.population < 10_000_000 {
                            45
                        } else if r_market.population < 15_000_000 {
                            55
                        } else {
                            70
                        };
                        result.push((
                            country.clone(),
                            r_key.clone(),
                            r_market.name.clone(),
                            r_market.population,
                            r_market.economic_strength,
                            fame_req,
                        ));
                    }
                }
            }
        }
        result
    }

    /// How famous live performance alone can make you. Every record in the
    /// catalog — including one still in its sales window — lifts the ceiling.
    fn live_fame_cap(&self) -> u8 {
        let mut singles = self.band.singles_released.len();
        let mut albums = self.band.albums_released.len();
        for release in &self.just_released_music {
            match release.release_type {
                ReleaseType::Single => singles += 1,
                ReleaseType::Album => albums += 1,
            }
        }
        (LIVE_FAME_BASE_CAP as usize
            + singles * LIVE_FAME_PER_SINGLE as usize
            + albums * LIVE_FAME_PER_ALBUM as usize)
            .min(constants::MAX_FAME as usize) as u8
    }

    pub(super) fn action_play_gig(&mut self, venue_index: usize) -> Result<(), String> {
        if self.player.energy < 30 {
            return Err("You're too tired to perform!".to_string());
        }
        if venue_index >= self.world.venues.len() {
            return Err("Invalid venue selected.".to_string());
        }
        let venue = &self.world.venues[venue_index];
        if venue.prestige > self.band.fame.saturating_add(20) {
            return Err(format!(
                "'{}' is out of your league! Get more famous first.",
                venue.name
            ));
        }

        self.player.energy -= 30;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();

        let attendance_ratio =
            ((self.band.fame as f32 + 10.0) / (venue.prestige as f32 + 10.0)).min(1.0);
        let attendance = (venue.capacity as f32 * attendance_ratio) as u32;

        let earnings =
            (venue.base_payment as f32 * attendance_ratio * market_modifier * era_modifier) as u32;

        let base_fame_gain = if venue.capacity <= 200 {
            1
        } else if venue.capacity <= 2000 {
            2
        } else {
            3
        };
        let fame_gain = if attendance_ratio < 0.5 {
            (base_fame_gain / 2).max(1)
        } else {
            base_fame_gain
        };

        let venue_ceiling = venue.prestige.saturating_add(VENUE_FAME_HEADROOM);
        let live_cap = self.live_fame_cap();
        let headroom = venue_ceiling.min(live_cap).saturating_sub(self.band.fame);
        let fame_gain = fame_gain.min(headroom);

        self.player.earn_money(earnings);
        self.band.fame = (self.band.fame + fame_gain).min(constants::MAX_FAME);
        if fame_gain > 0 {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}, fame +{}.",
                venue.name, attendance, venue.capacity, earnings, fame_gain
            ));
        } else if self.band.fame >= live_cap {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}. The buzz has peaked — without new records, word of mouth carries no further.",
                venue.name, attendance, venue.capacity, earnings
            ));
        } else {
            self.log(format!(
                "🎤 Played at '{}' — sold {}/{} tickets, earned ${}. The regulars know every word — you've outgrown this stage.",
                venue.name, attendance, venue.capacity, earnings
            ));
        }
        Ok(())
    }

    fn action_go_on_tour(&mut self, region_index: usize, rng: &mut impl Rng) -> Result<(), String> {
        if self.player.energy < 40 {
            return Err("You're too tired to go on tour!".to_string());
        }

        let sorted_regions = self.get_sorted_regions();
        if region_index >= sorted_regions.len() {
            return Err("Invalid region selected.".to_string());
        }

        let (country_key, region_key, region_name, population, economic_strength, fame_req) =
            &sorted_regions[region_index];

        if self.band.fame < *fame_req {
            return Err(format!(
                "Your band needs at least {} fame to tour '{}'.",
                fame_req, region_name
            ));
        }

        let tier_name = if self.band.fame < 35 {
            "local"
        } else if self.band.fame < 60 {
            "regional"
        } else if self.band.fame < 80 {
            "national"
        } else {
            "international"
        };

        let touring_costs = self
            .data_files
            .markets_data
            .market_modifiers
            .touring_costs
            .get(tier_name)
            .ok_or_else(|| "Touring cost tier not found.".to_string())?;

        let country_travel_mult = match country_key.as_str() {
            "united_states" => 1.5,
            "united_kingdom" => 0.8,
            "europe" => 1.2,
            "japan" => 1.0,
            "australia" => 1.4,
            _ => 1.0,
        };

        let tour_cost = (touring_costs.base_cost_per_show as f32 * country_travel_mult) as i32;

        if !self.player.can_afford(tour_cost) {
            return Err(format!(
                "You need at least ${} to finance this tour!",
                tour_cost
            ));
        }

        let (tour_weeks, fame_gain) = if self.band.fame >= 80 {
            (4, 10)
        } else if self.band.fame >= 60 {
            (3, 6)
        } else if self.band.fame >= 35 {
            (2, 4)
        } else {
            (2, 3)
        };

        let regional_fame_key = format!("{}:{}", country_key, region_key);
        let regional_fame = *self.regional_fame.get(&regional_fame_key).unwrap_or(&0);

        let audience = (self.band.fame as f32 / 3.0) + (regional_fame as f32);
        let base_gross =
            (*population as f32).sqrt() * (*economic_strength as f32 / 100.0) * audience * 0.06;

        let era_modifier = self.timeline.get_gig_pay_modifier();
        let market_modifier = self.world.get_market_modifier();
        let final_earnings = (base_gross * era_modifier * market_modifier) as i32;

        self.player.spend_money(tour_cost);
        self.player.earn_money(final_earnings as u32);
        self.player.energy -= 40;
        self.player.stress = (self.player.stress + 30).min(constants::MAX_STRESS);
        // Tours are live shows too: fame stalls at the catalog cap.
        let fame_gain = fame_gain.min(self.live_fame_cap().saturating_sub(self.band.fame));
        self.band.fame += fame_gain;

        let regional_fame_gain = 10 + rng.gen_range(0..=5);
        let new_regional_fame = (regional_fame as u16 + regional_fame_gain as u16).min(100) as u8;
        self.regional_fame
            .insert(regional_fame_key.clone(), new_regional_fame);

        self.week += tour_weeks;
        self.log(format!(
            "🚌 Tour of {} ({}): grossed ${} against ${} in costs, fame +{}, regional fame {}% (+{}).",
            region_name, country_key.replace("_", " "), final_earnings, tour_cost, fame_gain, new_regional_fame, regional_fame_gain
        ));

        if rng.gen_bool(0.3) {
            let bonus = 2u8.min(self.live_fame_cap().saturating_sub(self.band.fame));
            if bonus > 0 {
                self.band.fame += bonus;
                self.log("🗣️ Word of your live show spreads — extra fame on the way home.");
            }
        } else if rng.gen_bool(0.15) {
            self.player.health = self.player.health.saturating_sub(10);
            self.log("🤒 The road took its toll — you came home run down.");
        }
        Ok(())
    }

    pub(super) fn action_take_break(&mut self) -> Result<(), String> {
        self.player.energy = constants::MAX_ENERGY;
        self.player.stress = 0;
        self.player.health = (self.player.health + 30).min(constants::MAX_HEALTH);
        self.week += BREAK_WEEKS - 1;
        self.log(format!(
            "🏖️ You disappeared for {} weeks — fully recharged and healthier for it.",
            BREAK_WEEKS
        ));
        Ok(())
    }

    fn action_visit_doctor(&mut self) -> Result<(), String> {
        if !self.player.can_afford(constants::DOCTOR_VISIT_COST) {
            return Err(format!(
                "You need ${} to visit the doctor!",
                constants::DOCTOR_VISIT_COST
            ));
        }
        self.player.spend_money(constants::DOCTOR_VISIT_COST);
        self.player.health = (self.player.health + 20).min(constants::MAX_HEALTH);
        self.log(format!(
            "🩺 The doctor patched you up (+20 health, -${}).",
            constants::DOCTOR_VISIT_COST
        ));
        Ok(())
    }

    fn action_accept_deal(&mut self, offer_index: usize) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        let label_name = offer.label_name.clone();
        let advance = offer.advance;
        let albums_required = offer.albums_required;
        let new_deal = band::RecordDeal {
            label_name: offer.label_name,
            label_tier: offer.label_tier,
            advance: offer.advance,
            royalty_rate: offer.royalty_rate,
            albums_required: offer.albums_required,
            albums_delivered: 0,
            market_reach: offer.original_label_data.market_reach,
        };
        self.band.sign_deal(new_deal);
        self.player.earn_money(advance);
        self.pending_deal_offers.clear();
        self.log(format!(
            "✍️ Signed with {}! ${} advance in the bank — you owe them {} album{}.",
            label_name,
            advance,
            albums_required,
            if albums_required == 1 { "" } else { "s" }
        ));
        Ok(())
    }

    fn action_reject_deal(&mut self, offer_index: usize, rng: &mut impl Rng) -> Result<(), String> {
        if offer_index >= self.pending_deal_offers.len() {
            return Err("Invalid deal offer selected.".to_string());
        }
        let offer = self.pending_deal_offers.remove(offer_index);
        self.log(format!("🚫 Turned down {}'s offer.", offer.label_name));

        if let Some(poaching_band) = self.world.poach_rejected_deal(&offer.label_name, rng) {
            self.log(format!(
                "📰 NEWS: {} signed with {} after you turned them down!",
                poaching_band, offer.label_name
            ));
        }
        Ok(())
    }

    pub(super) fn action_accept_support_tour(&mut self, rng: &mut impl Rng) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.clone() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        if self.player.energy < 30 {
            return Err("You're too exhausted to head out on the road!".to_string());
        }
        self.pending_support_offer = None;

        self.player.earn_money(offer.pay);
        self.player.energy = self.player.energy.saturating_sub(35);
        self.player.stress = (self.player.stress + 20).min(constants::MAX_STRESS);
        self.band.fame = (self.band.fame + offer.fame_gain).min(constants::MAX_FAME);
        self.week += offer.weeks;
        self.log(format!(
            "🎟️ Opened for {} for {} weeks — ${} and a taste of the big stage (fame +{}).",
            offer.host_band, offer.weeks, offer.pay, offer.fame_gain
        ));

        if rng.gen_bool(0.25) {
            self.band.fame = (self.band.fame + 2).min(constants::MAX_FAME);
            self.log("🔥 Their crowd adopted you — encores every night (+2 fame).");
        }
        Ok(())
    }

    pub(super) fn action_decline_support_tour(&mut self) -> Result<(), String> {
        let Some(offer) = self.pending_support_offer.take() else {
            return Err("Nobody has offered you a support slot.".to_string());
        };
        self.log(format!("🚫 Passed on {}'s support slot.", offer.host_band));
        Ok(())
    }

    pub(super) fn execute_action(
        &mut self,
        action: GameAction,
        rng: &mut impl Rng,
    ) -> Result<(), String> {
        match action {
            GameAction::LazeAround => self.action_laze_around(),
            GameAction::WriteSongs => self.action_write_songs(rng),
            GameAction::Practice => self.action_practice(),
            GameAction::RecordSingle { pressing } => self.action_record_single(pressing, rng),
            GameAction::RecordAlbum { pressing } => self.action_record_album(pressing, rng),
            GameAction::Gig(venue_index) => self.action_play_gig(venue_index),
            GameAction::GoOnTour(region_index) => self.action_go_on_tour(region_index, rng),
            GameAction::TakeBreak => self.action_take_break(),
            GameAction::VisitDoctor => self.action_visit_doctor(),
            GameAction::AcceptDeal(index) => self.action_accept_deal(index),
            GameAction::RejectDeal(index) => self.action_reject_deal(index, rng),
            GameAction::AcceptSupportTour => self.action_accept_support_tour(rng),
            GameAction::DeclineSupportTour => self.action_decline_support_tour(),
            GameAction::StartMarketingCampaign(release_id, campaign_type) => {
                self.action_start_marketing_campaign(release_id, campaign_type)
            }
            GameAction::Quit => {
                self.game_over = true;
                Ok(())
            }
        }
    }
}
