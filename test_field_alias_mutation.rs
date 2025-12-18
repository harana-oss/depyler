#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'enum'()"]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Team {
    #[default]
    Home = 0,
    Away = 1,
}
impl Team {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Home),
            1 => Some(Self::Away),
            _ => None,
        }
    }
    pub fn from_i32_or_default(value: i32) -> Self {
        Self::from_i32(value).unwrap_or(Self::Home)
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SinBinStatus {
    #[default]
    NotSet = 0,
    Active = 1,
}
impl SinBinStatus {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::NotSet),
            1 => Some(Self::Active),
            _ => None,
        }
    }
    pub fn from_i32_or_default(value: i32) -> Self {
        Self::from_i32(value).unwrap_or(Self::NotSet)
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Player {
    pub sin_bin_status: SinBinStatus,
}
impl Player {
    pub fn new(sin_bin_status: SinBinStatus) -> Self {
        Self { sin_bin_status }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "sin_bin_status" => {
                Some(Box::new(self.sin_bin_status.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "sin_bin_status" => {
                if let Some(v) = value.downcast_ref::<SinBinStatus>() {
                    self.sin_bin_status = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamStatistics {
    pub sin_bin_players: Vec<Player>,
}
impl TeamStatistics {
    pub fn new(sin_bin_players: Vec<Player>) -> Self {
        Self { sin_bin_players }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "sin_bin_players" => {
                Some(Box::new(self.sin_bin_players.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "sin_bin_players" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.sin_bin_players = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub home_sin_bin: Vec<Player>,
    pub away_sin_bin: Vec<Player>,
    pub home_statistics: TeamStatistics,
    pub away_statistics: TeamStatistics,
}
impl State {
    pub fn new(
        home_sin_bin: Vec<Player>,
        away_sin_bin: Vec<Player>,
        home_statistics: TeamStatistics,
        away_statistics: TeamStatistics,
    ) -> Self {
        Self {
            home_sin_bin,
            away_sin_bin,
            home_statistics,
            away_statistics,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "home_sin_bin" => Some(Box::new(self.home_sin_bin.clone()) as Box<dyn std::any::Any>),
            "away_sin_bin" => Some(Box::new(self.away_sin_bin.clone()) as Box<dyn std::any::Any>),
            "home_statistics" => {
                Some(Box::new(self.home_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "away_statistics" => {
                Some(Box::new(self.away_statistics.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "home_sin_bin" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.home_sin_bin = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_sin_bin" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.away_sin_bin = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_statistics" => {
                if let Some(v) = value.downcast_ref::<TeamStatistics>() {
                    self.home_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_statistics" => {
                if let Some(v) = value.downcast_ref::<TeamStatistics>() {
                    self.away_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
pub fn rebuild_sin_bin_players(state: &mut State, team: &Team) {
    for team in vec![Team::Home, Team::Away] {
        let sin_bin_collection = if team == Team::Home {
            &state.home_sin_bin
        } else {
            &state.away_sin_bin
        };
        let sin_bin_players = sin_bin_collection
            .iter()
            .cloned()
            .filter(|player| player.sin_bin_status != SinBinStatus::NotSet)
            .collect::<Vec<_>>();
        let mut team_stats = if team == Team::Home {
            &mut state.home_statistics
        } else {
            &mut state.away_statistics
        };
        team_stats.sin_bin_players = sin_bin_players;
    }
}
