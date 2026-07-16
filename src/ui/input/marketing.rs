use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::game::GameAction;
use crate::game::music::MarketingCampaignType;
use crate::ui::app::{App, Screen};

impl App {
    pub(crate) fn handle_marketing_release_key(&mut self, key: KeyEvent) {
        let Screen::MarketingRelease { selected } = self.screen else {
            return;
        };
        let targets = self.marketing_targets();
        if targets.is_empty() {
            self.screen = Screen::Main;
            return;
        }

        match key.code {
            KeyCode::Esc => self.screen = Screen::Main,
            KeyCode::Up | KeyCode::Char('k') => {
                let selected = selected.checked_sub(1).unwrap_or(targets.len() - 1);
                self.screen = Screen::MarketingRelease { selected };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.screen = Screen::MarketingRelease {
                    selected: (selected + 1) % targets.len(),
                };
            }
            KeyCode::Enter => {
                let target = &targets[selected.min(targets.len() - 1)];
                self.screen = Screen::MarketingCampaign {
                    release_id: target.id,
                    release_name: target.name.clone(),
                    selected: 0,
                };
            }
            _ => {}
        }
    }

    pub(crate) fn handle_marketing_campaign_key(&mut self, key: KeyEvent) {
        let Screen::MarketingCampaign {
            release_id,
            selected,
            ..
        } = self.screen
        else {
            return;
        };
        let count = MarketingCampaignType::ALL.len();

        match key.code {
            KeyCode::Esc => self.screen = Screen::MarketingRelease { selected: 0 },
            KeyCode::Up | KeyCode::Char('k') => {
                if let Screen::MarketingCampaign { selected, .. } = &mut self.screen {
                    *selected = super::cycle_index(*selected, count, false);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Screen::MarketingCampaign { selected, .. } = &mut self.screen {
                    *selected = super::cycle_index(*selected, count, true);
                }
            }
            KeyCode::Enter => {
                let campaign = MarketingCampaignType::ALL[selected.min(count - 1)];
                self.screen = Screen::Main;
                self.dispatch(GameAction::StartMarketingCampaign(release_id, campaign));
            }
            _ => {}
        }
    }
}
