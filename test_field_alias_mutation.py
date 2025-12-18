from dataclasses import dataclass
from enum import IntEnum
from typing import List

class Team(IntEnum):
    Home = 0
    Away = 1

class SinBinStatus(IntEnum):
    NotSet = 0
    Active = 1

@dataclass
class Player:
    sin_bin_status: SinBinStatus

@dataclass
class TeamStatistics:
    sin_bin_players: List[Player]

@dataclass
class State:
    home_sin_bin: List[Player]
    away_sin_bin: List[Player]
    home_statistics: TeamStatistics
    away_statistics: TeamStatistics

def rebuild_sin_bin_players(state: State, team: Team) -> None:
    for team in [Team.Home, Team.Away]:
        sin_bin_collection = state.home_sin_bin if team == Team.Home else state.away_sin_bin
        sin_bin_players = [player for player in sin_bin_collection if player.sin_bin_status != SinBinStatus.NotSet]

        team_stats = state.home_statistics if team == Team.Home else state.away_statistics
        team_stats.sin_bin_players = sin_bin_players
