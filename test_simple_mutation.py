#!/usr/bin/env python3
"""Simple test case for the attribute assignment bug"""

class Scores:
    def __init__(self):
        self.tries = 0
        self.total = 0

class Statistics:
    def __init__(self):
        self.scores = Scores()

class TeamStatistics:
    def __init__(self):
        self.period_statistics = [Statistics()]

class State:
    def __init__(self):
        self.home_statistics = TeamStatistics()

def add_try(state: State) -> None:
    """Test function that should generate correct mutation code"""
    period_idx = 0
    
    # This should generate direct indexing, not .get().cloned().unwrap()
    state.home_statistics.period_statistics[period_idx].scores.tries = 5
    state.home_statistics.period_statistics[period_idx].scores.total = 25
