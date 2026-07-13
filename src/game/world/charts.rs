//! Weekly top-10 chart surface shared by the player and the scene.

use serde::{Deserialize, Serialize};

use super::GameWorld;

pub const CHART_SIZE: usize = 10;
const CHART_DECAY: f32 = 0.85;
const CHART_FLOOR_SCORE: u32 = 25;

/// One record on the weekly top-10.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartEntry {
    pub title: String,
    pub band_name: String,
    pub is_player: bool,
    pub score: u32,
    pub weeks_on_chart: u32,
}

impl GameWorld {
    pub(super) fn decay_charts(&mut self, news: &mut Vec<String>) {
        for entry in &mut self.charts {
            entry.score = (entry.score as f32 * CHART_DECAY) as u32;
            entry.weeks_on_chart += 1;
        }
        let dropped: Vec<&ChartEntry> = self
            .charts
            .iter()
            .filter(|e| e.is_player && e.score < CHART_FLOOR_SCORE)
            .collect();
        for entry in dropped {
            news.push(format!(
                "📉 '{}' slips off the charts after {} week{}.",
                entry.title,
                entry.weeks_on_chart,
                if entry.weeks_on_chart == 1 { "" } else { "s" }
            ));
        }
        self.charts.retain(|e| e.score >= CHART_FLOOR_SCORE);
    }

    /// Submit a release to the charts. Returns its position (1-based) if it
    /// makes the top 10.
    pub fn submit_chart_entry(
        &mut self,
        title: String,
        band_name: String,
        is_player: bool,
        score: u32,
    ) -> Option<usize> {
        self.charts.push(ChartEntry {
            title: title.clone(),
            band_name,
            is_player,
            score,
            weeks_on_chart: 0,
        });
        self.charts.sort_by_key(|e| std::cmp::Reverse(e.score));
        let position = self
            .charts
            .iter()
            .position(|e| e.weeks_on_chart == 0 && e.title == title)
            .map(|i| i + 1);
        self.charts.truncate(CHART_SIZE);
        position.filter(|&p| p <= CHART_SIZE)
    }
}
