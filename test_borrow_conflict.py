from dataclasses import dataclass

@dataclass
class Team:
    name: str

@dataclass
class State:
    score: int
    team_in_possession: Team

def assign_try(state: State, player_index: int, team: Team) -> None:
    state.score = state.score + 5

def process_play(state: State, player_index: int) -> None:
    # This should trigger borrow conflict detection:
    # state is passed as &mut (first arg)
    # state.team_in_possession accesses a field on state (third arg)
    assign_try(state, player_index, state.team_in_possession)
