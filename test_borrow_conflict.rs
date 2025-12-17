#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Team {
    pub name: String,
}
impl Team {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "name" => Some(Box::new(self.name.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "name" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.name = v.clone();
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
    pub score: i32,
    pub team_in_possession: Team,
}
impl State {
    pub fn new(score: i32, team_in_possession: Team) -> Self {
        Self {
            score,
            team_in_possession,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "score" => Some(Box::new(self.score.clone()) as Box<dyn std::any::Any>),
            "team_in_possession" => Some(Box::new(self.team_in_possession.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "score" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.score = v.clone();
                    true
                } else {
                    false
                }
            }
            "team_in_possession" => {
                if let Some(v) = value.downcast_ref::<Team>() {
                    self.team_in_possession = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
pub fn assign_try(state: &mut State, player_index: i32, team: &Team) {
    state.score = state.score + 5;
}
pub fn process_play(state: &mut State, player_index: i32) {
    assign_try(state, player_index, &state.team_in_possession.clone());
}
