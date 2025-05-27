use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io;

use crate::data::{calculate_weeks_to_years_months, constants, format_money};
use crate::game::{Game, GameAction};

pub struct TerminalUI {
    stdout: io::Stdout,
}

impl TerminalUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stdout: io::stdout(),
        })
    }

    pub fn clear_screen(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        execute!(self.stdout, Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        Ok(())
    }

    pub fn show_welcome(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;

        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("╔══════════════════════════════════════════╗\n"),
            Print("║              🎸 ROCKER 🎸               ║\n"),
            Print("║                                          ║\n"),
            Print("║      Rock Star Management Simulator     ║\n"),
            Print("║                                          ║\n"),
            Print("║  Build your band, write songs, tour     ║\n"),
            Print("║  the world, and become a ROCKSTAR!      ║\n"),
            Print("║                                          ║\n"),
            Print("╚══════════════════════════════════════════╝\n"),
            ResetColor,
            Print("\n"),
            Print("Press any key to continue..."),
        )?;

        self.wait_for_key()?;
        Ok(())
    }

    pub fn get_input(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        execute!(self.stdout, Print(prompt))?;

        let mut input = String::new();
        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => {
                        input.push(c);
                        execute!(self.stdout, Print(c))?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            execute!(
                                self.stdout,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }

        execute!(self.stdout, Print("\n"))?;
        Ok(input.trim().to_string())
    }

    pub fn display_game_state(&mut self, game: &Game) -> Result<(), Box<dyn std::error::Error>> {
        let current_year = 1970 + (game.week / 52);
        let week_in_year = game.week % 52;

        execute!(
            self.stdout,
            SetForegroundColor(Color::Cyan),
            Print("═══════════════════════════════════════════════════════════════\n"),
            Print(&format!(
                "  Week {} of {} | {} ({})\n",
                week_in_year + 1,
                current_year,
                game.player.name,
                game.band.name
            )),
            Print(&format!(
                "  Era: {} | {}\n",
                game.timeline.get_current_era().era_name,
                calculate_weeks_to_years_months(game.week)
            )),
            Print("═══════════════════════════════════════════════════════════════\n"),
            ResetColor,
        )?;

        // Player stats
        execute!(
            self.stdout,
            SetForegroundColor(Color::Yellow),
            Print("💰 FINANCES & HEALTH\n"),
            ResetColor,
            Print(&format!("  Money: {}\n", format_money(game.player.money))),
            Print(&format!(
                "  Health: {}% ({})\n",
                game.player.health,
                game.player.get_health_status()
            )),
            Print(&format!(
                "  Energy: {}% ({})\n",
                game.player.energy,
                game.player.get_energy_status()
            )),
            Print(&format!(
                "  Stress: {}% ({})\n",
                game.player.stress,
                game.player.get_stress_status()
            )),
        )?;

        // Add health warning if critical
        if game.player.health <= constants::CRITICAL_HEALTH_THRESHOLD {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Red),
                Print("  ⚠️  CRITICAL HEALTH - Visit a doctor immediately!\n"),
                ResetColor,
            )?;
        }

        // Add addiction warning
        if game.player.is_addicted() {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Yellow),
                Print("  ⚠️  ADDICTION PROBLEM - Consider rehabilitation!\n"),
                ResetColor,
            )?;
        }

        execute!(self.stdout, Print("\n"))?;

        // Band stats
        execute!(
            self.stdout,
            SetForegroundColor(Color::Magenta),
            Print("🎸 BAND STATUS\n"),
            ResetColor,
            Print(&format!(
                "  Fame: {}% ({})\n",
                game.band.fame,
                game.band.get_fame_level()
            )),
            Print(&format!(
                "  Skill: {}% ({}) | Avg Member: {}%\n",
                game.band.skill,
                game.band.get_skill_level(),
                game.band.average_member_skill()
            )),
            Print(&format!("  Band Morale: {}%\n", game.band.band_morale())),
            Print(&format!(
                "  Unreleased Songs: {}\n",
                game.band.unreleased_songs
            )),
            Print(&format!("  Singles Released: {}\n", game.band.singles)),
            Print(&format!("  Albums Released: {}\n", game.band.albums)),
        )?;

        // Band members
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Green),
            Print("👥 BAND MEMBERS\n"),
            ResetColor,
        )?;

        for member in &game.band.members {
            let status = if member.drug_problem { "⚠️" } else { "✅" };
            execute!(
                self.stdout,
                Print(&format!(
                    "  {} {} - {} (Skill: {}, Loyalty: {}) {}\n",
                    status,
                    member.name,
                    member.instrument,
                    member.skill,
                    member.loyalty,
                    if member.drug_problem {
                        "[Drug Problem]"
                    } else {
                        ""
                    }
                ))
            )?;
        }

        // Show current music trends and any recent historical events
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Blue),
            Print("🎵 MUSIC SCENE\n"),
            ResetColor,
            Print(&format!(
                "  Trending Genres: {}\n",
                game.timeline.get_trending_genres().join(", ")
            )),
            Print(&format!(
                "  Market Demand: {}%\n",
                game.timeline
                    .get_current_era()
                    .market_conditions
                    .overall_demand
            )),
            Print(&format!(
                "  Innovation Climate: {}%\n",
                game.timeline.get_innovation_bonus()
            )),
        )?;

        // Show historical event if there was one recently
        if let Some(historical_event) = game.get_last_historical_event() {
            execute!(
                self.stdout,
                SetForegroundColor(Color::Magenta),
                Print("📰 MUSIC NEWS: "),
                ResetColor,
                Print(&format!("{}\n", historical_event)),
            )?;
        }

        execute!(self.stdout, Print("\n"))?;
        Ok(())
    }

    pub fn show_main_menu(
        &mut self,
        game: &Game,
    ) -> Result<GameAction, Box<dyn std::error::Error>> {
        let single_cost = ((constants::SINGLE_RECORDING_COST as f32)
            * game.timeline.get_recording_cost_modifier()) as i32;
        let album_cost = ((constants::ALBUM_RECORDING_BASE_COST as f32)
            * game.timeline.get_recording_cost_modifier()) as i32;

        let single_available = game.band.can_record_single() && game.player.can_afford(single_cost);
        let album_available = game.band.can_record_album() && game.player.can_afford(album_cost);
        let doctor_available = game.player.can_afford(constants::DOCTOR_VISIT_COST);

        execute!(
            self.stdout,
            SetForegroundColor(Color::White),
            Print("🎵 WHAT DO YOU WANT TO DO THIS WEEK?\n"),
            ResetColor,
            Print("\n"),
            Print("1. Laze around (recover energy and reduce stress)\n"),
            Print("2. Write songs\n"),
            Print("3. Practice with the band\n"),
            Print(&format!(
                "4. Record a single ({}) {}\n",
                format_money(single_cost),
                if single_available { "✅" } else { "❌" }
            )),
            Print(&format!(
                "5. Record an album ({}) {}\n",
                format_money(album_cost),
                if album_available { "✅" } else { "❌" }
            )),
            Print("6. Play a gig\n"),
            Print("7. Go on tour\n"),
            Print("8. Take a break (full recovery)\n"),
            Print(&format!(
                "9. Visit the doctor ({}) {}\n",
                format_money(constants::DOCTOR_VISIT_COST),
                if doctor_available { "✅" } else { "❌" }
            )),
            Print("Q. Quit game\n"),
            Print("\n"),
            Print("Enter your choice: "),
        )?;

        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                let action = match code {
                    KeyCode::Char('1') => Some(GameAction::LazeAround),
                    KeyCode::Char('2') => Some(GameAction::WriteSongs),
                    KeyCode::Char('3') => Some(GameAction::Practice),
                    KeyCode::Char('4') => Some(GameAction::RecordSingle),
                    KeyCode::Char('5') => Some(GameAction::RecordAlbum),
                    KeyCode::Char('6') => Some(GameAction::Gig),
                    KeyCode::Char('7') => Some(GameAction::GoOnTour),
                    KeyCode::Char('8') => Some(GameAction::TakeBreak),
                    KeyCode::Char('9') => Some(GameAction::VisitDoctor),
                    KeyCode::Char('q') | KeyCode::Char('Q') => Some(GameAction::Quit),
                    _ => None,
                };

                if let Some(action) = action {
                    execute!(self.stdout, Print("\n\n"))?;
                    return Ok(action);
                }
            }
        }
    }

    pub fn show_error(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("❌ ERROR: "),
            Print(message),
            Print("\n\n"),
            ResetColor,
            Print("Press any key to continue..."),
        )?;

        self.wait_for_key()?;
        Ok(())
    }

    pub fn show_game_over(&mut self, game: &Game) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen()?;

        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            Print("╔══════════════════════════════════════════╗\n"),
            Print("║                GAME OVER                 ║\n"),
            Print("╚══════════════════════════════════════════╝\n"),
            ResetColor,
            Print("\n"),
            Print(&format!("Final Status: {}\n", game.get_status_message())),
            Print(&format!(
                "Career Length: {}\n",
                calculate_weeks_to_years_months(game.week)
            )),
            Print(&format!(
                "Final Fame Level: {}% ({})\n",
                game.band.fame,
                game.band.get_fame_level()
            )),
            Print(&format!("Money: {}\n", format_money(game.player.money))),
            Print(&format!("Singles Released: {}\n", game.band.singles)),
            Print(&format!("Albums Released: {}\n", game.band.albums)),
            Print("\n"),
            Print("Thanks for playing ROCKER!\n"),
            Print("Press any key to exit..."),
        )?;

        self.wait_for_key()?;
        Ok(())
    }

    fn wait_for_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            if let Event::Key(_) = event::read()? {
                break;
            }
        }
        Ok(())
    }
}
