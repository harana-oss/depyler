# Auto-generated concatenated Python file
# This file combines all generated Python modules

from state import State
from functions import *

# From: constants.py
CENTRE_OF_THE_FIELD_X = 500
CENTRE_OF_THE_FIELD_Y = 350
CONVERSION_POINTS = 2
CONVERSION_X_VALUES = [769, 773, 779, 788, 801, 823, 845, 866, 878, 890, 890, 882, 868, 847, 825, 801, 786, 779, 771, 769]
DROPOUT_X = 0
DROPOUT_Y = 350
EXTRA_TIME_INDEX = 2
EXTRA_TIME_LENGTH_IN_SECONDS = 600
FIELD_GOAL_ATTEMPT_EXTRA_TIME_SECONDS = 180
FIELD_GOAL_MAX_ABSOLUTE = 900
FIELD_GOAL_MAX_TIME_FIRST = 2400
FIELD_GOAL_MIN_ABSOLUTE = 100
FIELD_GOAL_MIN_AWAY = 400
FIELD_GOAL_MIN_HOME = 600
FIELD_GOAL_MIN_TIME_FIRST = 2340
FIELD_GOAL_MIN_TIME_SECOND = 4200
FIELD_GOAL_PARAM_EXTRA_TIME_VALUE = 25
FIELD_GOAL_POINTS = 1
FIRST_EIGHT_MINUTES_SECONDS = 480
FIRST_HALF_INDEX = 0
FULL_TIME_SECONDS = 4800
GAME_LENGTH_IN_SECONDS = 4800
GOAL_LINE_GRID_START_INDEX = 60
GOAL_LINE_X_WIDTH = 20
GOAL_LINE_Y_HEIGHT_INCREMENT = 113
GOAL_LINE_Y_VALUES = [(0, 113), (113, 226), (226, 339), (339, 452), (452, 565), (565, 678), (678, 700)]
GOAL_LINE_Y_WIDTH = 23.34
GOAL_LINE_ZONE_WEIGHTS = [[0.338255506, 0.161521868, 0.028635414, 0.011633486, 0.008945708, 0.063982141, 0.059955556, 0.028635414, 0.008948757, 0.003132014, 0.040716185, 0.043848199, 0.022818671, 0.016107357, 0.0035793, 0.038031456, 0.038478742, 0.024161543, 0.013869913, 0.005369458, 0.015660071, 0.012324, 0.005722, 0.003967, 0.0017], [0.006393969, 0.065970006, 0.021313621, 0.033491996, 0.005074616, 0.07510408, 0.08931277, 0.02841738, 0.039581769, 0.008118916, 0.082209011, 0.099462002, 0.042627242, 0.038566612, 0.008118916, 0.061910548, 0.079163538, 0.031462853, 0.024357922, 0.004059458, 0.050746158, 0.049731001, 0.035522311, 0.016239005, 0.003044301], [0.04926153, 0.045038504, 0.028853258, 0.021815592, 0.00492562, 0.065446776, 0.068261416, 0.027445938, 0.020408272, 0.00281464, 0.071077122, 0.064743116, 0.023222912, 0.016186312, 0.00281464, 0.038705564, 0.033075218, 0.031667898, 0.021111932, 0.00211098, 0.10696698, 0.098521994, 0.084447728, 0.067557756, 0.0035183], [0.10696698, 0.098521994, 0.084447728, 0.067557756, 0.0035183, 0.038705564, 0.033075218, 0.031667898, 0.021111932, 0.00211098, 0.071077122, 0.064743116, 0.023222912, 0.016186312, 0.00281464, 0.065446776, 0.068261416, 0.027445938, 0.020408272, 0.00281464, 0.04926153, 0.045038504, 0.028853258, 0.021815592, 0.00492562], [0.050746158, 0.049731001, 0.035522311, 0.016239005, 0.003044301, 0.061910548, 0.079163538, 0.031462853, 0.024357922, 0.004059458, 0.082209011, 0.099462002, 0.042627242, 0.038566612, 0.008118916, 0.07510408, 0.08931277, 0.02841738, 0.039581769, 0.008118916, 0.006393969, 0.065970006, 0.021313621, 0.033491996, 0.005074616], [0.015660071, 0.012324, 0.005722, 0.003961, 0.0017, 0.038031456, 0.038478742, 0.024161543, 0.013869913, 0.005369458, 0.040716185, 0.043848199, 0.022818671, 0.016107357, 0.0035793, 0.063982141, 0.059955556, 0.028635414, 0.008948757, 0.003132014, 0.338255506, 0.161521868, 0.028635414, 0.011633486, 0.008949708]]
GRID_RESULT = ["A1", "A2", "A3", "A4", "A5", "A6", "B1", "B2", "B3", "B4", "B5", "B6", "C1", "C2", "C3", "C4", "C5", "C6", "D1", "D2", "D3", "D4", "D5", "D6", "E1", "E2", "E3", "E4", "E5", "E6", "F1", "F2", "F3", "F4", "F5", "F6", "G1", "G2", "G3", "G4", "G5", "G6", "H1", "H2", "H3", "H4", "H5", "H6", "I1", "I2", "I3", "I4", "I5", "I6", "J1", "J2", "J3", "J4", "J5", "J6", "ZA1", "ZA2", "ZA3", "ZA4", "ZA5", "ZA6", "ZJ1", "ZJ2", "ZJ3", "ZJ4", "ZJ5", "ZJ6"]
HALF_LENGTH_IN_SECONDS = 2400
HOOKER_METRES_PER_MINUTE = 0.75057377
HOOKER_TACKLES_PER_MINUTE = 0.547090164
HOOKER_TRIES_PER_MINUTE = 0.00232582
INITIAL_SOLVER_HANDICAP = -3
INITIAL_SOLVER_TOTAL_POINTS = 45
KICKOFF_RESULTS = ["A1KickAway", "A1Retain", "A2KickAway", "A2Retain", "A4KickAway", "A4Retain", "A5Retain", "B1KickAway", "B1Retain", "B2KickAway", "B2Retain", "B3KickAway", "B3Retain", "B4KickAway", "B4Retain", "B5KickAway", "B5Retain", "B6KickAway", "B6Retain", "C1KickAway", "C2KickAway", "C2Retain", "C3KickAway", "C3Retain", "C4KickAway", "C4Retain", "C5KickAway", "C5Retain", "C6KickAway", "C6Retain", "D1KickAway", "D2KickAway", "D3KickAway", "D4KickAway", "D4Retain", "D5KickAway", "D6KickAway", "E1KickAway", "E1Retain", "E2KickAway", "E2Retain", "E3KickAway", "E4KickAway", "E4Retain", "E5KickAway", "E6KickAway", "F1KickAway", "F1Retain", "F2KickAway", "F2Retain", "F3KickAway", "F4KickAway", "F4Retain", "F5KickAway", "F6KickAway", "F6Retain", "G1KickAway", "G1Retain", "G2KickAway", "G2Retain", "G3KickAway", "G3Retain", "G4KickAway", "G4Retain", "G5KickAway", "G5Retain", "G6KickAway", "G6Retain", "H1KickAway", "H1Retain", "H2KickAway", "H2Retain", "H3KickAway", "H4KickAway", "H4Retain", "H5KickAway", "H5Retain", "H6KickAway", "H6Retain", "I1KickAway", "I1Retain", "I2KickAway", "I2Retain", "I3KickAway", "I3Retain", "I4KickAway", "I4Retain", "I5KickAway", "I5Retain", "I6KickAway", "I6Retain", "J1KickAway", "J2KickAway", "J3KickAway", "J4KickAway", "J5KickAway", "J6KickAway", "J6Retain"]
KICKOFF_RETAIN_RESULTS = ["A1Retain", "A2Retain", "A4Retain", "A5Retain", "B1Retain", "B2Retain", "B3Retain", "B4Retain", "B5Retain", "B6Retain", "C2Retain", "C3Retain", "C4Retain", "C6Retain", "D4Retain", "E1Retain", "E2Retain", "E4Retain", "F1Retain", "F2Retain", "F4Retain", "F6Retain", "G1Retain", "G2Retain", "G3Retain", "G4Retain", "G5Retain", "G6Retain", "H1Retain", "H2Retain", "H4Retain", "H5Retain", "H6Retain", "I1Retain", "I2Retain", "I3Retain", "I4Retain", "I6Retain", "J6Retain"]
KICKOFF_TO_GRID_RESULT = {"A1Retain": "A1", "A1KickAway": "A1", "A2Retain": "A2", "A2KickAway": "A2", "A4Retain": "A4", "A4KickAway": "A4", "A5Retain": "A5", "B1Retain": "B1", "B1KickAway": "B1", "B2Retain": "B2", "B2KickAway": "B2", "B3Retain": "B3", "B3KickAway": "B3", "B4Retain": "B4", "B4KickAway": "B4", "B5Retain": "B5", "B5KickAway": "B5", "B6Retain": "B6", "B6KickAway": "B6", "C1KickAway": "C1", "C2Retain": "C2", "C2KickAway": "C2", "C3Retain": "C3", "C3KickAway": "C3", "C4Retain": "C4", "C4KickAway": "C4", "C5KickAway": "C5", "C6Retain": "C6", "C6KickAway": "C6", "D1KickAway": "D1", "D2KickAway": "D2", "D3KickAway": "D3", "D4Retain": "D4", "D4KickAway": "D4", "D5KickAway": "D5", "D6KickAway": "D6", "E1Retain": "E1", "E1KickAway": "E1", "E2Retain": "E2", "E2KickAway": "E2", "E3KickAway": "E3", "E4Retain": "E4", "E4KickAway": "E4", "E5KickAway": "E5", "E6KickAway": "E6", "F1Retain": "F1", "F1KickAway": "F1", "F2Retain": "F2", "F2KickAway": "F2", "F3KickAway": "F3", "F4Retain": "F4", "F4KickAway": "F4", "F5KickAway": "F5", "F6Retain": "F6", "F6KickAway": "F6", "G1Retain": "G1", "G1KickAway": "G1", "G2Retain": "G2", "G2KickAway": "G2", "G3Retain": "G3", "G3KickAway": "G3", "G4Retain": "G4", "G4KickAway": "G4", "G5Retain": "G5", "G5KickAway": "G5", "G6Retain": "G6", "G6KickAway": "G6", "H1Retain": "H1", "H1KickAway": "H1", "H2Retain": "H2", "H2KickAway": "H2", "H3KickAway": "H3", "H4Retain": "H4", "H4KickAway": "H4", "H5Retain": "H5", "H5KickAway": "H5", "H6Retain": "H6", "H6KickAway": "H6", "I1Retain": "I1", "I1KickAway": "I1", "I2Retain": "I2", "I2KickAway": "I2", "I3Retain": "I3", "I3KickAway": "I3", "I4Retain": "I4", "I4KickAway": "I4", "I5KickAway": "I5", "I6Retain": "I6", "I6KickAway": "I6", "J1KickAway": "J1", "J2KickAway": "J2", "J3KickAway": "J3", "J4KickAway": "J4", "J5KickAway": "J5", "J6Retain": "J6", "J6KickAway": "J6"}
KICKOFF_X = 500
KICKOFF_Y = 350
LAST_EIGHT_MINUTES_SECONDS = 4500
LAST_MINUTE_SECONDS = 60
LAST_TEN_MINUTES_SECONDS = 600
LOCK_METRES_PER_MINUTE = 2.519467213
LOCK_TACKLES_PER_MINUTE = 0.563770492
LOCK_TRIES_PER_MINUTE = 0.001212746
MAX_INTERCHANGES = 8
MIN_POM_WEIGHT = 1e-4
MISSED_FIELD_GOAL_RESTART_X = 200
NEAR_END_SECONDS = 4700
NEXT_PLAY_TYPES = ["Pass", "Run", "RunTackle", "RunTry", "KickRetain", "KickRetainTackle", "KickRetainTry", "KickTurnover", "ErrorAttack", "ErrorDefence", "ConcededPenalty", "WonPenalty", "LineDropout", "FieldGoalAttempt"]
NUDGES_PERCENT = 1.0
NUMBER_OF_INTERCHANGE_PLAYERS = 4
NUMBER_OF_SIMULATIONS_DEFAULT = 40000
NUM_PERIODS = 4
NUM_PLAYERS_PER_TEAM = 17
NUM_STARTING_PLAYERS_PER_TEAM = 13
PENALTY_POINTS = 2
PLAYER_MODEL_POSITIONS = ["FullBack", "WingerOne", "CentreOne", "CentreTwo", "WingerTwo", "FiveEighth", "HalfBack", "PropOne", "Hooker", "PropTwo", "SecondRowOne", "SecondRowTwo", "Lock"]
PLAYER_POSITION_SEQUENCE = ["FullBack", "WingerOne", "CentreOne", "CentreTwo", "WingerTwo", "FiveEighth", "HalfBack", "PropOne", "Hooker", "PropTwo", "SecondRowOne", "SecondRowTwo", "Lock"]
PLAYING_FIELD_BASE = 0
PLAYING_FIELD_HEIGHT = 700
PLAYING_FIELD_WIDTH = 1000
PROP_METRES_PER_MINUTE = 2.519467213
PROP_TACKLES_PER_MINUTE = 0.563770492
PROP_TRIES_PER_MINUTE = 0.001212746
SECOND_HALF_INDEX = 1
SOLVER_INITIAL_GRADIENT_NUMBER_OF_SIMULATIONS = 15000
STARTING_TACKLE = 1
TACKLES_PER_SET = 6
TRY_POINTS = 4
TRY_SCORER_DEFAULT_INDEXES = 1
TRY_SCORER_MATCH_INDEXES = 15
TWO_POINT_FIELD_GOAL_DISTANCE = 400
TWO_POINT_FIELD_GOAL_POINTS = 2
X_VALUES = [(0, 100), (100, 200), (200, 300), (300, 400), (400, 500), (500, 600), (600, 700), (700, 800), (800, 900), (900, 1000), (-100, 1), (1000, 1100)]
X_VALUES_AWAY = [(900, 1000), (800, 900), (700, 800), (600, 700), (500, 600), (400, 500), (300, 400), (200, 300), (100, 200), (0, 100), (1000, 1100), (-100, 1)]
YELLOW_CARD_SECONDS = 600
Y_VALUES = [(0, 118), (118, 234), (234, 351), (351, 468), (468, 584), (584, 700)]
Y_VALUES_AWAY = [(584, 700), (468, 584), (351, 468), (234, 351), (118, 234), (0, 118)]

# From: event_debug.py
# @depyler: custom_attribute_ignore = "event"
def debug(state: State) -> None:
    away_running_score: int = 0
    home_running_score: int = 0
    race_to_targets: list[int] = list()
    remaining_targets: list[int] = list()
    target_index: int = 0

    if state.is_over:
        print("[Debug] Log")
        # Log
        print(f"State is over, home score = {state.home_match_score}, away score = {state.away_match_score}")

    return None

# From: event_output-anytime.py
# @depyler: custom_attribute_ignore = "event"
def anytime_output_event(state: State) -> None:
    away_extra_time_stats: Statistics | None = None
    away_second_half_stats: Statistics | None = None
    ended_half_1: bool = False
    ended_match: bool = False
    home_extra_time_stats: Statistics | None = None
    home_second_half_stats: Statistics | None = None
    team_a_second_half_points: int = 0
    team_b_second_half_points: int = 0

    if False:
        print("[Anytime Output Event] Get period and match statistics")
        # Get period and match statistics
        away_extra_time_stats = state.away_statistics.period_statistics[EXTRA_TIME_INDEX]
        away_second_half_stats = state.away_statistics.period_statistics[SECOND_HALF_INDEX]
        home_second_half_stats = state.home_statistics.period_statistics[SECOND_HALF_INDEX]
        home_extra_time_stats = state.home_statistics.period_statistics[EXTRA_TIME_INDEX]

        print("[Anytime Output Event] Calculate status flags")
        # Calculate status flags
        ended_half_1 = (state.period.number > 1)
        ended_match = state.is_over
        team_a_second_half_points = home_second_half_stats.scores.total
        team_b_second_half_points = away_second_half_stats.scores.total

        print("[Anytime Output Event] Output match status")
        # Output match status
        record_bool("EndedHalf1", ended_half_1)
        record_bool("EndedMatch", ended_match)

        print("[Anytime Output Event] Output second half points")
        # Output second half points
        record_int("PointsHalf2A", team_a_second_half_points)
        record_int("PointsHalf2B", team_b_second_half_points)

        print("[Anytime Output Event] Output match tries")
        # Output match tries
        record_int("TriesMatchA", state.home_statistics.total_statistics.scores.tries)
        record_int("TriesMatchB", state.away_statistics.total_statistics.scores.tries)

        print("[Anytime Output Event] Output extra time tries")
        # Output extra time tries
        record_int("TriesExtraTimeA", home_extra_time_stats.scores.tries)
        record_int("TriesExtraTimeB", away_extra_time_stats.scores.tries)

    return None

# From: event_output-end-of-game.py
# @depyler: custom_attribute_ignore = "event"
def end_of_game_output_event(state: State) -> None:
    away_extra_time_points: int = 0
    away_extra_time_stats: Statistics | None = None
    away_first_half_stats: Statistics | None = None
    away_second_half_stats: Statistics | None = None
    away_second_half_with_et: int = 0
    draw_match: bool = False
    home_extra_time_points: int = 0
    home_extra_time_stats: Statistics | None = None
    home_first_half_stats: Statistics | None = None
    home_second_half_stats: Statistics | None = None
    home_second_half_with_et: int = 0
    team_a_won_match: bool = False
    team_a_won_second_half_and_et: bool = False
    team_b_won_second_half_and_et: bool = False

    if state.game_status == 'ENDED':
        print("[End Of Game Output Event] Get period statistics")
        # Get period statistics
        away_extra_time_stats = state.away_statistics.period_statistics[EXTRA_TIME_INDEX]
        home_second_half_stats = state.home_statistics.period_statistics[SECOND_HALF_INDEX]
        home_first_half_stats = state.home_statistics.period_statistics[FIRST_HALF_INDEX]
        away_first_half_stats = state.away_statistics.period_statistics[FIRST_HALF_INDEX]
        home_extra_time_stats = state.home_statistics.period_statistics[EXTRA_TIME_INDEX]
        away_second_half_stats = state.away_statistics.period_statistics[SECOND_HALF_INDEX]

        print("[End Of Game Output Event] Calculate extra time and combined scores")
        # Calculate extra time and combined scores
        home_extra_time_points = home_extra_time_stats.scores.total
        home_second_half_with_et = home_second_half_stats.scores.total + home_extra_time_stats.scores.total
        away_second_half_with_et = away_second_half_stats.scores.total + away_extra_time_stats.scores.total
        away_extra_time_points = away_extra_time_stats.scores.total

        print("[End Of Game Output Event] Determine second half and extra time winner")
        # Determine second half and extra time winner
        team_a_won_second_half_and_et = (home_second_half_with_et > away_second_half_with_et)
        team_b_won_second_half_and_et = (home_second_half_with_et < away_second_half_with_et)

        print("[End Of Game Output Event] Determine match results")
        # Determine match results
        team_a_won_match = (state.home_match_score > state.away_match_score)
        draw_match = (state.home_match_score == state.away_match_score)
        team_b_won_match = (state.away_match_score > state.home_match_score)

        print("[End Of Game Output Event] Output match totals")
        # Output match totals
        record_int("PointsMatchA", state.home_match_score)
        record_int("PointsMatchB", state.away_match_score)

        print("[End Of Game Output Event] Output extra time points")
        # Output extra time points
        record_int("PointsExtraTimeA", home_extra_time_points)
        record_int("PointsExtraTimeB", away_extra_time_points)

        print("[End Of Game Output Event] Output second half and extra time results")
        # Output second half and extra time results
        record_bool("WinHalf2AndExtraTimeA", team_a_won_second_half_and_et)
        record_bool("WinHalf2AndExtraTimeB", team_b_won_second_half_and_et)

        print("[End Of Game Output Event] Output match results")
        # Output match results
        record_bool("DrawMatch", draw_match)
        record_bool("WinMatchA", team_a_won_match)
        record_bool("WinMatchB", team_b_won_match)

    return None

# From: event_output-first-half.py
# @depyler: custom_attribute_ignore = "event"
def first_half_output_event(state: State) -> None:
    away_first_half_stats: Statistics | None = None
    away_first_half_total: int = 0
    away_first_half_tries: int = 0
    draw_first_half: bool = False
    home_first_half_stats: Statistics | None = None
    home_first_half_total: int = 0
    home_first_half_tries: int = 0
    team_a_won_first_half: bool = False
    team_b_won_first_half: bool = False

    if False:
        print("[First Half Output Event] Get first half period statistics")
        # Get first half period statistics
        home_first_half_stats = state.home_statistics.period_statistics[FIRST_HALF_INDEX]
        away_first_half_stats = state.away_statistics.period_statistics[FIRST_HALF_INDEX]

        print("[First Half Output Event] Calculate first half scores")
        # Calculate first half scores
        home_first_half_tries = home_first_half_stats.scores.tries
        away_first_half_tries = away_first_half_stats.scores.tries
        away_first_half_total = away_first_half_stats.scores.total
        home_first_half_total = home_first_half_stats.scores.total

        print("[First Half Output Event] Determine first half results")
        # Determine first half results
        team_a_won_first_half = (home_first_half_total > away_first_half_total)
        team_b_won_first_half = (away_first_half_total > home_first_half_total)
        draw_first_half = (home_first_half_total == away_first_half_total)

        print("[First Half Output Event] Output first half points")
        # Output first half points
        record_int("PointsHalf1A", home_first_half_total)
        record_int("PointsHalf1B", away_first_half_total)

        print("[First Half Output Event] Output first half win/draw results")
        # Output first half win/draw results
        record_bool("WinHalf1A", team_a_won_first_half)
        record_bool("WinHalf1B", team_b_won_first_half)
        record_bool("DrawHalf1", draw_first_half)

        print("[First Half Output Event] Output first half tries")
        # Output first half tries
        record_int("TriesHalf1A", home_first_half_tries)
        record_int("TriesHalf1B", away_first_half_tries)

    return None

# From: event_output-normal-time.py
# @depyler: custom_attribute_ignore = "event"
def normal_time_output_event(state: State) -> None:
    away_first_half_stats: Statistics | None = None
    away_normal_time_score: int = 0
    away_second_half_stats: Statistics | None = None
    extra_time_occurred: bool = False
    home_first_half_stats: Statistics | None = None
    home_normal_time_score: int = 0
    home_second_half_stats: Statistics | None = None

    if False:
        print("[Normal Time Output Event] Get period statistics")
        # Get period statistics
        away_first_half_stats = state.away_statistics.period_statistics[FIRST_HALF_INDEX]
        away_normal_time_score = 0
        away_second_half_stats = state.away_statistics.period_statistics[SECOND_HALF_INDEX]
        home_normal_time_score = 0
        home_first_half_stats = state.home_statistics.period_statistics[FIRST_HALF_INDEX]
        home_second_half_stats = state.home_statistics.period_statistics[SECOND_HALF_INDEX]

        print("[Normal Time Output Event] Calculate normal time scores")
        # Calculate normal time scores
        away_normal_time_score = away_first_half_stats.scores.total + away_second_half_stats.scores.total
        home_normal_time_score = home_first_half_stats.scores.total + home_second_half_stats.scores.total

        print("[Normal Time Output Event] Determine extra time occurrence")
        # Determine extra time occurrence
        extra_time_occurred = (home_normal_time_score == away_normal_time_score)

        print("[Normal Time Output Event] Output normal time results")
        # Output normal time results
        record_bool("ExtraTimeOccurredMatch", extra_time_occurred)
        record_int("PointsNormalTimeA", home_normal_time_score)
        record_int("PointsNormalTimeB", away_normal_time_score)
        record_bool("WinNormalTimeA", home_normal_time_score > away_normal_time_score)
        record_bool("WinNormalTimeB", away_normal_time_score > home_normal_time_score)

        print("[Normal Time Output Event] Output second half tries")
        # Output second half tries
        record_int("TriesHalf2A", home_second_half_stats.scores.tries)
        record_int("TriesHalf2B", away_second_half_stats.scores.tries)

    return None

# From: event_output-player-void.py
# @depyler: custom_attribute_ignore = "event"
def player_void_output_event(state: State) -> None:
    away_trader_players: list[Player] = list()
    home_trader_players: list[Player] = list()

    if True:
        print("[Player Void Output Event] Bind trader state players")
        # Bind trader state players
        home_trader_players = state.home_players
        away_trader_players = state.away_players

        print("[Player Void Output Event] Output home player void status")
        # Output home player void status
        for player in home_trader_players:
            print("[Player Void Output Event] record")
            # record
            record_bool(f"Player_Voided_{player.player_index}", player.is_voided)

        print("[Player Void Output Event] Output away player void status")
        # Output away player void status
        for player in away_trader_players:
            print("[Player Void Output Event] record")
            # record
            record_bool(f"Player_Voided_{player.player_index}", player.is_voided)

    return None

# From: event_period-first-last-score.py
# @depyler: custom_attribute_ignore = "event"
def period_first_last_score_helper(state: State) -> None:
    away_first_try_time: int = 0
    first_score_team: str = ""
    first_try_team: str = ""
    first_try_time: int = 0
    home_first_try_time: int = 0
    period_filter: bool = False
    period_name: str = ""

    if False:
        print("[Period First Last Score Helper] Find first try in period")
        # Find first try in period
        first_try_team = (([incident for incident in state.incidents if period_filter and incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try'][0].points_scored_team))

        print("[Period First Last Score Helper] Find first score in period")
        # Find first score in period
        first_score_team = ([incident for incident in state.incidents if period_filter and incident.has_points_confirmed][0].points_scored_team)

        print("[Period First Last Score Helper] Calculate first try times")
        # Calculate first try times
        away_first_try_time = (([incident for incident in state.incidents if period_filter and incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try' and incident.points_scored_team == "Away"][0].time_elapsed // 60 + 1))
        first_try_time = (([incident for incident in state.incidents if period_filter and incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try'][0].time_elapsed // 60 + 1))
        home_first_try_time = (([incident for incident in state.incidents if period_filter and incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try' and incident.points_scored_team == "Home"][0].time_elapsed // 60 + 1))

        print("[Period First Last Score Helper] Output first try results")
        # Output first try results
        record_bool(f"FirstTry{period_name}A", first_try_team == "Home")
        record_bool(f"FirstTry{period_name}B", first_try_team == "Away")

        print("[Period First Last Score Helper] Output first to score results")
        # Output first to score results
        record_bool(f"FirstToScore{period_name}A", first_score_team == "Home")
        record_bool(f"FirstToScore{period_name}B", first_score_team == "Away")

        print("[Period First Last Score Helper] Output first try time results")
        # Output first try time results
        record_int(f"FirstTryTime{period_name}", first_try_time)
        record_int(f"FirstTryTime{period_name}A", home_first_try_time)
        record_int(f"FirstTryTime{period_name}B", away_first_try_time)

    return None

# From: event_track-first-to-score.py
# @depyler: custom_attribute_ignore = "event"
def first_to_score_tracking_event(state: State) -> None:
    away_first_try_incident: Optional[Incident] = None
    away_first_try_time: Optional[int] = None
    extra_time_first_score_incident: Optional[Incident] = None
    extra_time_first_try_incident: Optional[Incident] = None
    half2_et_first_score_incident: Optional[Incident] = None
    half2_et_first_try_incident: Optional[Incident] = None
    home_first_try_incident: Optional[Incident] = None
    home_first_try_time: Optional[int] = None
    last_score_incident: Optional[Incident] = None
    last_try_incident: Optional[Incident] = None
    last_try_time: Optional[int] = None
    match_first_score_incident: Optional[Incident] = None
    match_first_try_incident: Optional[Incident] = None
    match_first_try_time: Optional[int] = None
    points_confirmed_incidents: list[Incident] = list()
    try_incidents: list[Incident] = list()

    if len(state.incidents) > 0 or state.is_over:
        print("[First To Score Tracking Event] Collect confirmed scoring incidents")
        # Collect confirmed scoring incidents
        points_confirmed_incidents = ([incident for incident in state.incidents if incident.has_points_confirmed and incident.points_scored_team in ("Home", "Away")])

        print("[First To Score Tracking Event] Collect confirmed try incidents")
        # Collect confirmed try incidents
        try_incidents = (([incident for incident in points_confirmed_incidents if incident.points_confirmed_score_type == 'Try']))

        print("[First To Score Tracking Event] Resolve key incidents across periods")
        # Resolve key incidents across periods
        match_first_score_incident = (points_confirmed_incidents[0] if points_confirmed_incidents else None)
        match_first_try_incident = (try_incidents[0] if try_incidents else None)
        half2_et_first_score_incident = ((next((incident for incident in points_confirmed_incidents if (int(incident.period.number) - 1) >= SECOND_HALF_INDEX), None)))
        away_first_try_incident = ((next((incident for incident in try_incidents if incident.points_scored_team == "Away"), None)))
        half2_et_first_try_incident = ((next((incident for incident in try_incidents if (int(incident.period.number) - 1) >= SECOND_HALF_INDEX), None)))
        home_first_try_incident = ((next((incident for incident in try_incidents if incident.points_scored_team == "Home"), None)))
        last_score_incident = (points_confirmed_incidents[-1] if points_confirmed_incidents else None)
        extra_time_first_try_incident = ((next((incident for incident in try_incidents if (int(incident.period.number) - 1) == EXTRA_TIME_INDEX), None)))
        extra_time_first_score_incident = ((next((incident for incident in points_confirmed_incidents if (int(incident.period.number) - 1) == EXTRA_TIME_INDEX), None)))
        last_try_incident = (try_incidents[-1] if try_incidents else None)

        print("[First To Score Tracking Event] Compute try timings")
        # Compute try timings
        last_try_time = ((int(last_try_incident.time_elapsed / 60) + 1) if last_try_incident else None)
        away_first_try_time = ((int(away_first_try_incident.time_elapsed / 60) + 1) if away_first_try_incident is not None else None)
        home_first_try_time = ((int(home_first_try_incident.time_elapsed / 60) + 1) if home_first_try_incident is not None else None)
        match_first_try_time = ((int(match_first_try_incident.time_elapsed / 60) + 1) if match_first_try_incident is not None else None)

    return None

# From: event_track-minute-winner.py
# @depyler: custom_attribute_ignore = "event"
def minute_winner_tracking_event(state: State) -> None:
    final_minute: int = 0
    minute_intervals: list[int] = list()

    if len(state.incidents) > 0 or state.is_over:
        print("[Minute Winner Tracking Event] Initialize interval context")
        # Initialize interval context
        minute_intervals = [10, 20, 30, 50, 60]
        final_minute = int(state.time_elapsed / 60)

        print("[Minute Winner Tracking Event] Track winner at each minute interval")
        # Track winner at each minute interval
        for minute in minute_intervals:
            print("[Minute Winner Tracking Event] Record minute winners when interval elapsed")
            # Record minute winners when interval elapsed
            if final_minute > minute:
                print("[Minute Winner Tracking Event] Calculate scores through interval")
                # Calculate scores through interval
                home_points_at_minute = (sum(incident.points_scored_points for incident in state.incidents if incident.has_points_confirmed and incident.points_scored_team == "Home" and incident.time_elapsed <= minute * 60))
                away_points_at_minute = (sum(incident.points_scored_points for incident in state.incidents if incident.has_points_confirmed and incident.points_scored_team == "Away" and incident.time_elapsed <= minute * 60))

                print("[Minute Winner Tracking Event] Output minute winner results")
                # Output minute winner results
                record_bool(f"WinMinute{minute}MatchA", home_points_at_minute > away_points_at_minute)
                record_bool(f"WinMinute{minute}MatchB", away_points_at_minute > home_points_at_minute)

    return None

# From: event_track-player-scores.py
# @depyler: custom_attribute_ignore = "event"
def player_score_tracking_event(state: State) -> None:
    away_first_try_jersey: int = 0
    away_try_scorer_indices: list[int] = list()
    first_try_jersey: int = 0
    home_first_try_jersey: int = 0
    home_try_scorer_indices: list[int] = list()
    last_try_jersey: int = 0
    three_unanswered_tries_team: str = ""
    try_scorer_indices: list[int] = list()
    try_teams: list[str] = list()

    if len(state.incidents) > 0:
        print("[Player Score Tracking Event] Collect try scorer information")
        # Collect try scorer information
        try_scorer_indices = ([incident.points_confirmed_player_index for incident in state.incidents if incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try'])
        try_teams = ([incident.points_scored_team for incident in state.incidents if incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try'])
        home_try_scorer_indices = ([incident.points_confirmed_player_index for incident in state.incidents if incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try' and incident.points_scored_team == "Home"])
        away_try_scorer_indices = ([incident.points_confirmed_player_index for incident in state.incidents if incident.has_points_confirmed and incident.points_confirmed_score_type == 'Try' and incident.points_scored_team == "Away"])

        print("[Player Score Tracking Event] Find jersey numbers for first and last try scorers")
        # Find jersey numbers for first and last try scorers
        first_try_jersey = state.all_players[try_scorer_indices[0]].jersey_number
        home_first_try_jersey = state.home_players[home_try_scorer_indices[0]].jersey_number
        last_try_jersey = state.all_players[try_scorer_indices[len(try_scorer_indices) - 1]].jersey_number
        away_first_try_jersey = state.away_players[away_try_scorer_indices[0]].jersey_number

        print("[Player Score Tracking Event] Output try scorer sequence (up to 15 tries)")
        # Output try scorer sequence (up to 15 tries)
        for index in range(1, min(15, len(try_scorer_indices))):
            print("[Player Score Tracking Event] record")
            # record
            record_int(f"{index}thTryScorer_Match", try_scorer_indices[index - 1])

        print("[Player Score Tracking Event] Output try team sequence")
        # Output try team sequence
        for index in range(1, len(try_teams)):
            print("[Player Score Tracking Event] record")
            # record
            record_bool(f"{index}thTry_Match_A", try_teams[index - 1] == "Home")
            record_bool(f"{index}thTry_Match_B", try_teams[index - 1] == "Away")

        print("[Player Score Tracking Event] Output first try scorer information")
        # Output first try scorer information
        record_int("FirstTryScorerJerseyMatch", first_try_jersey)

        print("[Player Score Tracking Event] Output home team first try scorer")
        # Output home team first try scorer
        record_int("HomeFirstTryScorerMatch", home_try_scorer_indices[0])
        record_int("HomeFirstTryScorerJerseyMatch", home_first_try_jersey)

        print("[Player Score Tracking Event] Output away team first try scorer")
        # Output away team first try scorer
        record_int("AwayFirstTryScorerMatch", away_try_scorer_indices[0])
        record_int("AwayFirstTryScorerJerseyMatch", away_first_try_jersey)

        print("[Player Score Tracking Event] Output three unanswered tries")
        # Output three unanswered tries
        record_bool("ThreeUnansweredTries", True)

        print("[Player Score Tracking Event] Output last try scorer (end of match only)")
        # Output last try scorer (end of match only)
        record_int("LastTryScorerMatch", try_scorer_indices[len(try_scorer_indices) - 1])
        record_int("LastTryScorerJerseyMatch", last_try_jersey)
        record_int("HomeLastTryScorerMatch", home_try_scorer_indices[len(home_try_scorer_indices) - 1])
        record_int("AwayLastTryScorerMatch", away_try_scorer_indices[len(away_try_scorer_indices) - 1])

        print("[Player Score Tracking Event] Output player statistics from trader state")
        # Output player statistics from trader state
        for player in state.home_players:
            print("[Player Score Tracking Event] record")
            # record
            record_bool(f"Player_{player.player_index}_AnytimeTryScorer", player.total_statistics.scores.tries > 0)
            record_bool(f"Player_{player.player_index}_2PlusTryScorer", player.total_statistics.scores.tries >= 2)
            record_bool(f"Player_{player.player_index}_3PlusTryScorer", player.total_statistics.scores.tries >= 3)

        print("[Player Score Tracking Event] Output away player statistics from trader state")
        # Output away player statistics from trader state
        for player in state.away_players:
            print("[Player Score Tracking Event] record")
            # record
            record_bool(f"Player_{player.player_index}_AnytimeTryScorer", player.total_statistics.scores.tries > 0)
            record_bool(f"Player_{player.player_index}_2PlusTryScorer", player.total_statistics.scores.tries >= 2)
            record_bool(f"Player_{player.player_index}_3PlusTryScorer", player.total_statistics.scores.tries >= 3)

    return None

# From: event_track-race-to-points.py
# @depyler: custom_attribute_ignore = "event"
def race_to_points_tracking_event(state: State) -> None:
    away_running_score: int = 0
    home_running_score: int = 0
    race_to_targets: list[int] = list()
    remaining_targets: list[int] = list()
    target_index: int = 0

    if len(state.incidents) > 0 or state.is_over:
        print("[Race To Points Tracking Event] Initialize race to context")
        # Initialize race to context
        target_index = 0
        home_running_score = 0
        away_running_score = 0
        race_to_targets = [10, 15, 20, 25, 30, 35, 40]

        print("[Race To Points Tracking Event] Process scoring incidents for race targets")
        # Process scoring incidents for race targets
        for incident in state.incidents:
            print("[Race To Points Tracking Event] Apply confirmed score to running totals")
            # Apply confirmed score to running totals
            if incident.has_points_confirmed and incident.points_scored_team in ("Home", "Away") and target_index < len(race_to_targets):
                print("[Race To Points Tracking Event] Update team totals")
                # Update team totals
                if incident.points_scored_team == "Home":
                    print("[Race To Points Tracking Event] Set variables")
                    # Set variables
                    home_running_score = home_running_score + incident.points_scored_points
                else:
                    print("[Race To Points Tracking Event] Set variables")
                    # Set variables
                    away_running_score = away_running_score + incident.points_scored_points

                print("[Race To Points Tracking Event] Set current race target")
                # Set current race target
                current_target = race_to_targets[target_index]

                print("[Race To Points Tracking Event] Check target completion")
                # Check target completion
                home_reached_target = (home_running_score >= current_target)
                away_reached_target = (away_running_score >= current_target)

                print("[Race To Points Tracking Event] Record race winner when exactly one team reaches target")
                # Record race winner when exactly one team reaches target
                if home_reached_target != away_reached_target:
                    print("[Race To Points Tracking Event] Record")
                    # Record
                    record_bool(f"FirstToPoints{current_target}A", home_reached_target)
                    record_bool(f"FirstToPoints{current_target}B", away_reached_target)

                    print("[Race To Points Tracking Event] Set variables")
                    # Set variables
                    target_index = target_index + 1

        print("[Race To Points Tracking Event] Capture remaining targets as false")
        # Capture remaining targets as false
        remaining_targets = race_to_targets[target_index:]

        print("[Race To Points Tracking Event] Record unresolved race targets")
        # Record unresolved race targets
        for pending_target in remaining_targets:
            print("[Race To Points Tracking Event] Record")
            # Record
            record_bool(f"FirstToPoints{pending_target}A", False)
            record_bool(f"FirstToPoints{pending_target}B", False)

    return None

# From: fn_conversion.py
# @depyler: custom_attribute = "function"
def add_conversion(state: State) -> None:
    team = state.team_in_possession
    period_idx = state.period.number - 1

    print(">>>>>>>>>>>>>>>> Adding conversion for team:", state.team_in_possession)

    if team == "Home":
      state.home_match_score += CONVERSION_POINTS
    else:
      state.away_match_score += CONVERSION_POINTS

    team_stats = state.home_statistics if team == "Home" else state.away_statistics

    team_stats.period_statistics[period_idx].scores.conversions += 1
    team_stats.period_statistics[period_idx].scores.total += CONVERSION_POINTS
    team_stats.total_statistics.scores.conversions += 1
    team_stats.total_statistics.scores.total += CONVERSION_POINTS

    if state.include_players:
      player = state.home_player_selected_for_points_market if team == "Home" else state.away_player_selected_for_points_market
      player.total_statistics.scores.conversions += 1
      player.total_statistics.scores.total += CONVERSION_POINTS
      player.period_statistics[period_idx].scores.conversions += 1
      player.period_statistics[period_idx].scores.total += CONVERSION_POINTS

# From: fn_distance.py
# @depyler: custom_attribute = "function"
def calculate_dist_from_centre(state: State) -> int:
    """Distance from centre of the field from the perspective of the team in possession."""
    if state.team_in_possession == "Home":
        return int(abs(CENTRE_OF_THE_FIELD_Y - state.ball_location.y))
    else:
        return int(abs(CENTRE_OF_THE_FIELD_Y - (PLAYING_FIELD_HEIGHT - state.ball_location.y)))


def calculate_dist_to_try_line(state: State) -> int:
    """Distance to the try line from the perspective of the team in possession."""
    if state.team_in_possession == "Home":
        return PLAYING_FIELD_WIDTH - state.ball_location.x
    else:
        return state.ball_location.x

# From: fn_field_goal.py
# @depyler: custom_attribute = "function"
def add_field_goal(state: State, is_two_pointer: bool) -> None:
    team = state.team_in_possession
    period_idx = state.period.number - 1
    points = TWO_POINT_FIELD_GOAL_POINTS if is_two_pointer else FIELD_GOAL_POINTS

    print(">>>>>>>>>>>>>>>> Adding field goal for team:", state.team_in_possession)

    if team == "Home":
      state.home_match_score += points
    else:
      state.away_match_score += points

    team_stats = state.home_statistics if team == "Home" else state.away_statistics

    team_stats.period_statistics[period_idx].scores.field_goals += 1
    team_stats.period_statistics[period_idx].scores.total += points
    team_stats.total_statistics.scores.field_goals += 1
    team_stats.total_statistics.scores.total += points

    player = state.home_player_selected_for_points_market if team == "Home" else state.away_player_selected_for_points_market
    player.period_statistics[period_idx].scores.total += points
    player.total_statistics.scores.total += points

# From: fn_interchange.py
import random

# @depyler: custom_attribute = "function"
def get_time_on_field_for_position(state: State, team: str, position: str) -> float:
    players = _team_players(state, team)
    candidate = next(p for p in players if p.position.position_type == position)
    return float(candidate.total_statistics.time_on_field)


def get_interchanges_used(state: State, team: str) -> float:
    remaining = _get_remaining_interchanges(state, team)
    used = MAX_INTERCHANGES - remaining
    return float(max(0, used))


def get_team_margin(state: State, team: str) -> float:
    home_score = state.home_match_score
    away_score = state.away_match_score
    return float(home_score - away_score) if team == "Home" else float(away_score - home_score)


def decrement_remaining_interchanges(state: State, team: str) -> None:
    current = _get_remaining_interchanges(state, team)
    remaining = max(0, current - 1)
    _set_remaining_interchanges(state, team, remaining)


def _select_bench_player_index(players: list[Player], starters: int) -> int:
    available = [idx for idx in range(starters, len(players)) if not players[idx].is_injured]
    return random.choice(available)


def _swap_team_statistics(state: State, team: str, off_slot: int, bench_slot: int) -> None:
    stats = _team_statistic(state, team)
    stats.player_statistics[off_slot], stats.player_statistics[bench_slot] = stats.player_statistics[bench_slot], stats.player_statistics[off_slot]
    stats.sin_bin_players[off_slot], stats.sin_bin_players[bench_slot] = stats.sin_bin_players[bench_slot], stats.sin_bin_players[off_slot]


def _swap_game_statistics(state: State, team: str, off_slot: int, bench_slot: int) -> None:
    mirrored = state.home_statistics if team == "Home" else state.away_statistics
    team_stats = _team_statistic(state, team)
    
    if mirrored is team_stats:
        return

    mirrored.player_statistics[off_slot], mirrored.player_statistics[bench_slot] = mirrored.player_statistics[bench_slot], mirrored.player_statistics[off_slot]
    mirrored.sin_bin_players[off_slot], mirrored.sin_bin_players[bench_slot] = mirrored.sin_bin_players[bench_slot], mirrored.sin_bin_players[off_slot]


def _refresh_all_players(state: State) -> None:
    combined = []
    for team in ("Home", "Away"):
        combined.extend(_team_players(state, team))
    
    state.all_players[:] = combined


def _team_players(state: State, team: str) -> list[Player]:
    return state.home_players if team == "Home" else state.away_players


def _team_statistic(state: State, team: str) -> TeamStatistics:
    return state.home_statistics if team == "Home" else state.away_statistics


def _get_remaining_interchanges(state: State, team: str) -> int:
    return state.home_remaining_interchanges if team == "Home" else state.away_remaining_interchanges


def _set_remaining_interchanges(state: State, team: str, value: int) -> None:
    if team == "Home":
        state.home_remaining_interchanges = value
    else:
        state.away_remaining_interchanges = value

# From: fn_margin.py
# @depyler: custom_attribute = "function"
def calculate_foul_team_margin(state: State) -> int:
    """Margin from the perspective of the penalty-awarded team (matches C# processors)."""
    if state.current_play_type == 'WonPenalty':
        penalty_team = state.team_in_possession
    else:
        penalty_team = "Away" if state.team_in_possession == "Home" else "Home"

    if penalty_team == "Home":
        return state.home_match_score - state.away_match_score
    else:
        return state.away_match_score - state.home_match_score


def calculate_margin(state: State) -> int:
    """Margin from the perspective of the team in possession."""
    if state.team_in_possession == "Home":
        return state.home_match_score - state.away_match_score
    else:
        return state.away_match_score - state.home_match_score

# From: fn_math.py
import math

# @depyler: custom_attribute = "function"
def btf(value: bool) -> float:
    return 1.0 if value else 0.0


def sample(distribution: list[float], k: float) -> int:
    """Sample from a discrete distribution using inverse transform sampling."""
    cumulative = 0.0
    for i, p in enumerate(distribution):
        cumulative += p
        if k < cumulative:
            return i
    return len(distribution) - 1


def sample_scaled(distribution: list[float], distribution_sum: float, k: float) -> int:
    """Sample from an unnormalized distribution, scaling k by the sum."""
    threshold = k * distribution_sum
    cumulative = 0.0
    for i, p in enumerate(distribution):
        cumulative += p
        if threshold < cumulative:
            return i
    return len(distribution) - 1


def softmax(logits: list[float]) -> list[float]:
    """Apply softmax transformation, returning a new list."""
    max_logit = max(logits)
    exps = [math.exp(x - max_logit) for x in logits]
    total = sum(exps)
    return [e / total for e in exps]


def normalize(values: list[float]) -> list[float]:
    """Normalize values to sum to 1, returning a new list."""
    total = sum(values)
    return [v / total for v in values]

# From: fn_penalty.py
# @depyler: custom_attribute = "function"
def add_penalty(state: State) -> None:
    team = state.team_in_possession
    period_idx = state.period.number - 1

    print(">>>>>>>>>>>>>>>> Adding penalty for team:", state.team_in_possession)

    if team == "Home":
      state.home_match_score += PENALTY_POINTS
    else:
      state.away_match_score += PENALTY_POINTS

    team_stats = state.home_statistics if team == "Home" else state.away_statistics

    team_stats.total_statistics.scores.penalties += 1
    team_stats.total_statistics.scores.total += PENALTY_POINTS
    team_stats.period_statistics[period_idx].scores.penalties += 1
    team_stats.period_statistics[period_idx].scores.total += PENALTY_POINTS

    player = state.home_player_selected_for_points_market if team == "Home" else state.away_player_selected_for_points_market
    player.total_statistics.scores.conversions += 1
    player.total_statistics.scores.total += PENALTY_POINTS
    player.period_statistics[period_idx].scores.conversions += 1
    player.period_statistics[period_idx].scores.total += PENALTY_POINTS

# From: fn_player_of_match.py
import math

# @depyler: custom_attribute = "function"
def calculate_player_of_match_distributions(state: State) -> list[float]:
    players = state.all_players
    home_score = state.home_match_score
    away_score = state.away_match_score
    margin = home_score - away_score
    total_points = home_score + away_score

    return [_calculate_percentage_chance(player, margin, total_points) for player in players]


def _calculate_percentage_chance(player: Player, margin: int, total_points: int) -> float:
    margin_factor = _calculate_margin_factor(player.delta_strength, margin)

    match_stats = player.total_statistics
    match_tries = match_stats.scores.tries
    match_score = match_stats.scores.total

    pom_percentage = player.player_of_the_match_percentage
    team_won = _team_won(player, margin)

    pom_factor = _calculate_pom_chance(
        player.tries_strength,
        match_tries,
        match_score,
        pom_percentage,
        team_won,
    )

    total_factor = _calculate_total_factor(player.total_strength, total_points)
    return margin_factor * pom_factor * total_factor + 0.5 * pom_percentage


def _calculate_margin_factor(strength: str, margin: int) -> float:
    abs_margin = abs(margin)

    if strength == "LOW":
        return max(2.5 - abs_margin / 10.0, 0.5)
    if strength == "HIGH":
        return min(0.5 * abs_margin / 10.0, 2.5)
    return 1.0


def _calculate_total_factor(strength: str, total_points: int) -> float:
    if strength == "LOW":
        return max(3.0 - math.exp(0.175 * total_points / 10.0), 0.1)
    if strength == "HIGH":
        return math.exp(0.175 * total_points / 9.9) - 1.0
    return 1.0


def _calculate_pom_chance(
    tries_strength: str,
    match_tries: int,
    match_score: int,
    pom_percentage: float,
    team_won: bool,
) -> float:
    try_factor = _calculate_try_factor(tries_strength, match_tries)
    win_factor = 500 if team_won else 0
    points_scored = match_score - match_tries * 4
    points_factor = max(1.0, 0.3357 * points_scored - 1.75)

    pom_weight = (pom_percentage + pom_percentage * win_factor + try_factor) * points_factor
    return max(pom_weight, MIN_POM_WEIGHT)


def _calculate_try_factor(strength: str, tries: int) -> float:
    coefficient = {
        "LOW": 1.0,
        "MID": 3.0,
        "HIGH": 10.0,
    }.get(strength, 3.0)

    if tries <= 0:
        return 0.0

    return (coefficient * math.pow(tries, 4.5)) / 2.0


def _team_won(player: Player, margin: int) -> bool:
    is_home = player.is_home_team
    if margin == 0:
        return False
    return margin > 0 if is_home else margin < 0

# From: fn_position.py
from __future__ import annotations

import random

# @depyler: custom_attribute = "function"
def _goal_line_bucket(grid_coordinate: int, k: float) -> int:
    if grid_coordinate < GOAL_LINE_GRID_START_INDEX:
        return -1

    y_band = grid_coordinate % 6
    weights = GOAL_LINE_ZONE_WEIGHTS[y_band]
    return sample(weights, k)


def compute_conversion_location_from_grid(grid_coordinate: int, k: float) -> int:
    bucket = _goal_line_bucket(grid_coordinate, k)

    if bucket < 0:
        default_band = len(CONVERSION_X_VALUES) // 2
        return CONVERSION_X_VALUES[default_band]

    bucket_row = bucket // 5
    base_y = int(bucket_row * GOAL_LINE_Y_WIDTH + (GOAL_LINE_Y_WIDTH // 2))
    goal_line_field_y = base_y + (grid_coordinate % 6) * GOAL_LINE_Y_HEIGHT_INCREMENT

    conversion_band = goal_line_field_y // 35
    conversion_band = max(0, min(conversion_band, len(CONVERSION_X_VALUES) - 1))
    conversion_location = CONVERSION_X_VALUES[conversion_band]

    return conversion_location


def goal_line_position(goal_line_index: int, grid_index: int) -> FieldPosition:
    x = (
        PLAYING_FIELD_WIDTH
        + (goal_line_index % 5) * GOAL_LINE_X_WIDTH
        + (GOAL_LINE_X_WIDTH // 2)
    )
    y = int(goal_line_index // 5 * GOAL_LINE_Y_WIDTH + (GOAL_LINE_Y_WIDTH / 2))
    y += (grid_index % 6) * GOAL_LINE_Y_HEIGHT_INCREMENT

    return FieldPosition(x=x, y=y)


def convert_field_position(state: State, kickoff_result: str, team: str) -> FieldPosition:
    grid_result = GRID_RESULT.index(KICKOFF_TO_GRID_RESULT[kickoff_result])

    y_position = grid_result % 6
    x_position = grid_result // 6
    
    if team == "Home":
        x_range = X_VALUES[x_position]
        y_range = Y_VALUES[y_position]
    else:
        x_range = X_VALUES_AWAY[x_position]
        y_range = Y_VALUES_AWAY[y_position]
    
    return FieldPosition(
        x=random.randint(x_range[0], x_range[1] - 1),
        y=random.randint(y_range[0], y_range[1] - 1)
    )


def derive_grid_coordinate(position: FieldPosition, team: str) -> int:
    if "Away" in team:
        x_ranges = X_VALUES_AWAY
        y_ranges = Y_VALUES_AWAY
    else:
        x_ranges = X_VALUES
        y_ranges = Y_VALUES

    x_index = _resolve_index(position.x, x_ranges)
    y_index = _resolve_index(position.y, y_ranges)

    if x_index < 0 or y_index < 0:
        return -1

    return x_index * 6 + y_index


def get_goal_line_coordinate(state: State) -> int:
    grid_coordinate = derive_grid_coordinate(state.ball_location, state.team_in_possession)

    if grid_coordinate < 0:
        return 65

    return 65 + (grid_coordinate % 6)


def _resolve_index(coordinate: int, ranges: list[tuple[int, int]]) -> int:
    for index, range_bounds in enumerate(ranges):
        lower, upper = range_bounds

        if lower <= coordinate < upper:
            return index

    return -1

# From: fn_sin_bin.py
# @depyler: custom_attribute = "function"
def record_player_sin_bin(state: State, player_index: int, team: str, sin_bin_type: str = "YellowCard") -> None:
    players = state.home_players if team == "Home" else state.away_players
    player = _resolve_player(players, player_index)

    if player is not None:
      sin_bin_collection = state.home_sin_bin if team == "Home" else state.away_sin_bin
      if player not in sin_bin_collection:
          sin_bin_collection.append(player)

      seconds_elapsed = state.time_elapsed
      duration = YELLOW_CARD_SECONDS if sin_bin_type == "YellowCard" else 0

      player.sin_bin_status = sin_bin_type
      player.return_from_sin_bin_time = seconds_elapsed + duration
      player.on_field = False
      player.sin_bin_sent_off = seconds_elapsed

      rebuild_sin_bin_players(state, team)


def _resolve_player(players: list[Player], candidate_index: int) -> Optional[Player]:
    for player in players:
        if player.player_index == candidate_index:
            return player

    target_position = PLAYER_MODEL_POSITIONS[candidate_index]

    for player in players:
        if player.position.position_type == target_position and player.on_field:
            return player

    for player in players:
        if player.position.position_type == target_position:
            return player

    return None


def rebuild_sin_bin_players(state: State, team: str) -> None:
    team_names = [team] if team else ["Home", "Away"]
    for team_name in team_names:
        sin_bin_collection = state.home_sin_bin if team_name == "Home" else state.away_sin_bin
        sin_bin_players = [player for player in sin_bin_collection if player.sin_bin_status != "NotSet"]

        team_stats = state.home_statistics if team_name == "Home" else state.away_statistics
        team_stats.sin_bin_players = sin_bin_players

# From: fn_tackle.py
# @depyler: custom_attribute = "function"
def record_tackle(state: State) -> None:
    print(state)

    team = state.team_in_possession
    period_idx = state.period.number - 1

    team_stats = state.home_statistics if team == "Home" else state.away_statistics
    team_stats.period_statistics[period_idx].tackles += 1
    team_stats.total_statistics.tackles += 1

# From: fn_team.py
from __future__ import annotations

from .sin_bin import rebuild_sin_bin_players

# @depyler: custom_attribute = "function"
def _create_scores() -> Scores:
    return Scores(
        conversions=0,
        field_goals=0,
        penalties=0,
        total=0,
        tries=0,
    )


def _create_statistics() -> Statistics:
    return Statistics(
        conversions_missed=0,
        penalties_awarded=0,
        scores=_create_scores(),
        tackles=0,
        turnovers=0,
    )


def _create_player_statistics(player: Player) -> PlayerStatistics:
    period_breakdown = [_create_statistics() for _ in range(NUM_PERIODS)]

    return PlayerStatistics(
        jersey_number=player.jersey_number,
        metres_gained=0,
        period_statistics=period_breakdown,
        player_index=player.player_index,
        scores=_create_scores(),
        tackles=0,
        time_on_field=0.0,
        total_statistics=_create_statistics(),
    )


def _create_team_statistics() -> TeamStatistics:
    return TeamStatistics(
        period_statistics=[_create_statistics() for _ in range(NUM_PERIODS)],
        player_statistics=[],
        sin_bin_players=[],
        total_statistics=_create_statistics(),
    )


def get_team_handicap(state: State) -> float:
    if state.team_in_possession == "Home":
        return state.simulation_invariants.home_handicap
    return state.simulation_invariants.away_handicap


def swap_possession(state: State) -> None:
    state.set += 1
    state.team_in_possession = "Away" if state.team_in_possession == "Home" else "Home"
    state.tackles = STARTING_TACKLE

def setup_statistics(state: State) -> None:
    state.home_statistics = _create_team_statistics()
    state.away_statistics = _create_team_statistics()
    state.home_sin_bin = []
    state.away_sin_bin = []

    for team in ("Home", "Away"):
        players = _get_players(state, team)
        team_stats = _get_team_stats(state, team)
        team_stats.player_statistics = []

        for player in players:
            player.period_statistics = [
                _create_player_statistics(player) for _ in range(NUM_PERIODS)
            ]
            total_statistics = _create_player_statistics(player)
            player.total_statistics = total_statistics
            team_stats.player_statistics.append(total_statistics)

        rebuild_sin_bin_players(state, team)


def log_state(state: State) -> None:
    print(state)


def get_team_try_distributions(state: State) -> list[float]:
    players = _get_players(state, state.team_in_possession)
    return [p.tries_percentage for p in players]


def apply_time_on_ground(state: State, team: str, seconds: float) -> None:
    players = _get_players(state, team)
    period_idx = state.period.number - 1

    for player in players[:NUM_STARTING_PLAYERS_PER_TEAM]:
        player.period_statistics[period_idx].time_on_field += seconds
        player.total_statistics.time_on_field += seconds

    for s in _get_team_stats(state, team).player_statistics[:NUM_STARTING_PLAYERS_PER_TEAM]:
        s.time_on_field += seconds


def _get_team_stats(state: State, team: str) -> TeamStatistics:
    return state.home_statistics if team == "Home" else state.away_statistics

def _get_players(state: State, team: str) -> list[Player]:
    return state.home_players if team == "Home" else state.away_players

# From: fn_try.py
from .team import _get_team_stats


# @depyler: custom_attribute = "function"
def add_try(state: State) -> None:
    period_idx = state.period.number - 1

    print(">>>>>>>>>>>>>>>> Adding try for team:", state.team_in_possession)

    if state.team_in_possession == "Home":
        state.home_match_score += TRY_POINTS
        state.home_statistics.period_statistics[period_idx].scores.tries += 1
        state.home_statistics.period_statistics[period_idx].scores.total += TRY_POINTS
        state.home_statistics.total_statistics.scores.tries += 1
        state.home_statistics.total_statistics.scores.total += TRY_POINTS
    else:
        state.away_match_score += TRY_POINTS
        state.away_statistics.period_statistics[period_idx].scores.tries += 1
        state.away_statistics.period_statistics[period_idx].scores.total += TRY_POINTS
        state.away_statistics.total_statistics.scores.tries += 1
        state.away_statistics.total_statistics.scores.total += TRY_POINTS

    print(state)


def assign_try(state: State, player_index: int, team: str) -> None:
    period_idx = state.period.number - 1

    players = state.home_players if team == "Home" else state.away_players
    player_list_idx, player = next((i, p) for i, p in enumerate(players) if p.player_index == player_index)

    state.period_try_scorers[period_idx].append(player_list_idx)
    state.total_try_scorers.append(player_list_idx)

    if team == "Home":
        state.home_period_try_scorers[period_idx].append(player_list_idx)
        state.home_total_try_scorers.append(player_list_idx)
    else:
        state.away_period_try_scorers[period_idx].append(player_list_idx)
        state.away_total_try_scorers.append(player_list_idx)

    player.period_statistics[period_idx].scores.tries += 1
    player.period_statistics[period_idx].scores.total += TRY_POINTS
    player.total_statistics.scores.tries += 1
    player.total_statistics.scores.total += TRY_POINTS

    team_stats = state.home_statistics if team == "Home" else state.away_statistics
    team_stats.player_statistics[player_list_idx].scores.tries += 1
    team_stats.player_statistics[player_list_idx].scores.total += TRY_POINTS

# From: game.py
# @depyler: custom_attribute_ignore = "game"
def nrl(state: State) -> None:
    current_minute: int = 0
    field_goal_attempt_result: int = 0
    minute_delta: int = 0
    previous_minute: int = 0

    print("[NRL] Setup Statistics")
    # Setup Statistics
    setup_statistics(state)

    execute_events(state)

    print("[NRL] Setup State")
    # Setup State
    state.include_players = False
    state.period.name = "FirstHalf"
    state.period.number = 1

    execute_events(state)

    print("[NRL] Log State")
    # Log State
    log_state(state)

    execute_events(state)

    print("[NRL] Game Loop")
    # Game Loop
    while True:
        print("[NRL] Store previous state")
        # Store previous state
        state.previous_ball_location = state.ball_location
        state.previous_play_type = state.current_play_type

        execute_events(state)

        print("[NRL] Evaluate field goal")
        # Evaluate field goal
        field_goal_decision: FieldGoalDecisionOutputs = field_goal_decision(state)

        execute_events(state)

        print("[NRL] Run field goal attempt model only if gating passes")
        # Run field goal attempt model only if gating passes
        if field_goal_decision.attempt:
            print("[NRL] Run field_goal_attempt")
            # Run field_goal_attempt
            field_goal_attempt_outcome: FieldGoalAttemptOutputs = field_goal_attempt(state)

            execute_events(state)

            print("[NRL] Set variables")
            # Set variables
            field_goal_attempt_result = int(field_goal_attempt_outcome.attempt)

            execute_events(state)
        else:
            print("[NRL] Set variables")
            # Set variables
            field_goal_attempt_result = 0

            execute_events(state)

        execute_events(state)

        print("[NRL] Branch on field goal attempt vs normal play")
        # Branch on field goal attempt vs normal play
        if field_goal_attempt_result == 1:
            print("[NRL] Run field_goal")
            # Run field_goal
            field_goal(state)

            execute_events(state)
        else:
            print("[NRL] Get next play type")
            # Get next play type
            next_play_result: NextPlayOutputs = next_play(state)

            execute_events(state)

            print("[NRL] Set current play type")
            # Set current play type
            state.current_play_type = str(next_play_result.play_type)

            execute_events(state)

            print("[NRL] Line Dropout")
            # Line Dropout
            if state.current_play_type == 'LineDropout':
                print("[NRL] Run swap_possession")
                # Run swap_possession
                swap_possession(state)

                execute_events(state)

                print("[NRL] Run kickoff")
                # Run kickoff
                kickoff(state, force_possession_change=True, is_line_dropout=True)

                execute_events(state)
            else:
                print("[NRL] Get XY model result")
                # Get XY model result
                xy_result: XyOutputs = xy(state)

                execute_events(state)

                print("[NRL] Set ball location")
                # Set ball location
                state.ball_location = xy_result.field_position

                execute_events(state)

                print("[NRL] Process play type")
                # Process play type
                if state.current_play_type == "KickTurnover":
                    print("[NRL] Run swap_possession")
                    # Run swap_possession
                    swap_possession(state)

                    execute_events(state)
                elif state.current_play_type == "Run":
                    pass
                elif state.current_play_type == "ConcededPenalty":
                    print("[NRL] Run process_penalty")
                    # Run process_penalty
                    process_penalty(state)

                    execute_events(state)
                elif state.current_play_type == "ErrorAttack":
                    print("[NRL] Run swap_possession")
                    # Run swap_possession
                    swap_possession(state)

                    execute_events(state)
                elif state.current_play_type == "ErrorDefence":
                    print("[NRL] Set variables")
                    # Set variables
                    state.tackles = STARTING_TACKLE

                    execute_events(state)
                elif state.current_play_type == "RunTry":
                    print("[NRL] Run process_try")
                    # Run process_try
                    process_try(state)

                    execute_events(state)
                elif state.current_play_type == "WonPenalty":
                    print("[NRL] Run process_penalty")
                    # Run process_penalty
                    process_penalty(state)

                    execute_events(state)
                elif state.current_play_type == "KickRetain":
                    pass
                elif state.current_play_type == "KickRetainTry":
                    print("[NRL] Run process_try")
                    # Run process_try
                    process_try(state)

                    execute_events(state)
                elif state.current_play_type == "KickRetainTackle":
                    print("[NRL] Run process_tackle")
                    # Run process_tackle
                    process_tackle(state)

                    execute_events(state)
                elif state.current_play_type == "RunTackle":
                    print("[NRL] Run process_tackle")
                    # Run process_tackle
                    process_tackle(state)

                    execute_events(state)
                elif state.current_play_type == "Pass":
                    pass

                execute_events(state)

            execute_events(state)

        execute_events(state)

        print("[NRL] Snapshot elapsed minute before clock")
        # Snapshot elapsed minute before clock
        previous_minute = int(state.time_elapsed / 60)

        execute_events(state)

        print("[NRL] Advance clock")
        # Advance clock
        clock(state)

        execute_events(state)

        print("[NRL] Compute elapsed minute after clock")
        # Compute elapsed minute after clock
        current_minute = int(state.time_elapsed / 60)

        execute_events(state)

        print("[NRL] Compute minute delta")
        # Compute minute delta
        minute_delta = current_minute - previous_minute

        execute_events(state)

        print("[NRL] Process interchanges when minute advances")
        # Process interchanges when minute advances
        if state.include_players and minute_delta > 0:
            print("[NRL] Run apply_time_on_ground")
            # Run apply_time_on_ground
            apply_time_on_ground(state, seconds=float(minute_delta * 60), team="Home")

            execute_events(state)

            print("[NRL] Run apply_time_on_ground")
            # Run apply_time_on_ground
            apply_time_on_ground(state, seconds=float(minute_delta * 60), team="Away")

            execute_events(state)

            print("[NRL] Run interchange")
            # Run interchange
            interchange(state)

            execute_events(state)

        execute_events(state)

        print("[NRL] Check game status and period transitions")
        # Check game status and period transitions
        check_game_status(state)

        execute_events(state)

        print("[NRL] Stop if complete")
        # Stop if complete
        if state.is_over:
            print("[NRL] Exit")
            # Exit
            break

            execute_events(state)

        execute_events(state)

    execute_events(state)

    return None

# From: outputs.py
def unnamed(state: State) -> None:

    return None

# From: state.py
from dataclasses import dataclass

from typing import Any

@dataclass
class State:
    all_players: list[Player]
    away_match_score: int
    away_period_try_scorers: list[list[int]]
    away_player_selected_for_points_market: Player
    away_players_trader_state: list[Player]
    away_players: list[Player]
    away_remaining_interchanges: int
    away_sin_bin: list[Player]
    away_statistics: TeamStatistics
    away_total_try_scorers: list[int]
    ball_location: FieldPosition
    current_play_type: str
    end_zone_type: str
    has_started: bool
    is_extratime: bool
    game_status: str
    home_match_score: int
    home_period_try_scorers: list[list[int]]
    home_player_selected_for_points_market: Player
    home_players_trader_state: list[Player]
    home_players: list[Player]
    home_remaining_interchanges: int
    home_sin_bin: list[Player]
    home_statistics: TeamStatistics
    home_total_try_scorers: list[int]
    incidents: list[Incident]
    include_players: bool
    is_in_end_zone: bool
    is_over: bool
    last_play_type: str
    period_try_scorers: list[list[int]]
    period: Period
    player_of_the_match: int
    previous_ball_location: FieldPosition
    previous_play_type: str
    set: int
    simulation_invariants: SimulationInvariants
    tackles: int
    team_in_possession: str
    time_elapsed: int
    total_match_score: int
    total_period: str
    total_try_scorers: list[int]

@dataclass
class FieldPosition:
    x: int
    y: int

@dataclass
class Incident:
    has_points_confirmed: bool
    has_points_scored: bool
    period: Period
    points_confirmed_player_index: int
    points_confirmed_score_id: str
    points_confirmed_score_type: str
    points_scored_points: int
    points_scored_score_id: str
    points_scored_score_type: str
    points_scored_team: str
    time_elapsed: int

@dataclass
class Period:
    number: int
    name: str

@dataclass
class Player:
    delta_strength: str
    expected_tries_per_min: float
    is_home_team: bool
    is_injured: bool
    is_interchange: bool
    is_starter: bool
    is_suspended: bool
    is_voided: bool
    jersey_number: int
    on_field: bool
    period_statistics: list[PlayerStatistics]
    player_index: int
    player_of_the_match_percentage: float
    position: PlayerPosition
    return_from_sin_bin_time: int
    selected_for_points_market: bool
    sin_bin_sent_off: int
    sin_bin_status: str
    total_statistics: PlayerStatistics
    total_strength: str
    tries_per_minute_in_use: bool
    tries_per_minute: float
    tries_percentage_in_use: bool
    tries_percentage: float
    tries_strength: str

@dataclass
class PlayerPosition:
    position_type: str

@dataclass
class PlayerStatistics:
    jersey_number: int
    metres_gained: int
    period_statistics: list[Statistics]
    player_index: int
    scores: Scores
    tackles: int
    time_on_field: float
    total_statistics: Statistics

@dataclass
class Scores:
    conversions: int
    field_goals: int
    penalties: int
    total: int
    tries: int

@dataclass
class Statistics:
    conversions_missed: int
    penalties_awarded: int
    scores: Scores
    tackles: int
    turnovers: int

@dataclass
class TeamStatistics:
    period_statistics: list[Statistics]
    player_statistics: list[PlayerStatistics]
    sin_bin_players: list[Player]
    total_statistics: Statistics

@dataclass
class PlayerInvariants:
    player_solution: PlayerSolution
    player_trader_state: PlayerTraderState

@dataclass
class PlayerSolution:
    solved_expected_tries_per_min: float
    solved_expected_tries_percentage: float

@dataclass
class SimulationInvariants:
    all_players_invariants: list[PlayerInvariants]
    away_handicap: float
    away_price: float
    away_team_players_invariants: list[PlayerInvariants]
    home_handicap: float
    home_price: float
    home_team_player_invariants: list[PlayerInvariants]
    include_players: bool
    player_of_the_total_enabled: bool
    total_points: float

@dataclass
class PlayerTraderState:
    expected_tries_per_minute: float
    home_team: bool
    is_injured: bool
    is_interchange: bool
    is_starter: bool
    player_index: int
    player_of_the_match_percentage: float
    position: PlayerPosition
    selected_for_points_market: bool
    tries_per_minute_in_use: bool
    tries_percentage_in_use: bool
    tries_percentage: float

# From: step_check_game_status.py
# @depyler: custom_attribute_ignore = "step"
def check_game_status(state: State) -> None:
    print("[CheckGameStatus] Check if normal time half is complete")
    # Check if normal time half is complete
    if state.time_elapsed > HALF_LENGTH_IN_SECONDS and state.period.name != 'ExtraTime':
        print("[CheckGameStatus] Handle end of first half")
        # Handle end of first half
        if state.period.name == 'FirstHalf':
            print("[CheckGameStatus] Transition to second half")
            # Transition to second half
            state.game_status = "Period2"
            state.period.number = 2
            state.period.name = "SecondHalf"

            execute_events(state)
        else:
            print("[CheckGameStatus] Check if normal time is complete")
            # Check if normal time is complete
            if state.time_elapsed >= GAME_LENGTH_IN_SECONDS:
                print("[CheckGameStatus] Check for extra time or game end")
                # Check for extra time or game end
                if state.home_match_score == state.away_match_score:
                    print("[CheckGameStatus] Enter extra time")
                    # Enter extra time
                    state.period.name = "ExtraTime"
                    state.game_status = "ExtraTime"
                    state.period.number = 3
                    state.is_extratime = True

                    execute_events(state)
                else:
                    print("[CheckGameStatus] End game")
                    # End game
                    state.game_status = "Ended"
                    state.is_over = True

                    execute_events(state)

                execute_events(state)

            execute_events(state)

        execute_events(state)

    execute_events(state)

    print("[CheckGameStatus] Check extra time conditions")
    # Check extra time conditions
    if state.period.name == 'ExtraTime':
        print("[CheckGameStatus] Check if extra time should end")
        # Check if extra time should end
        if state.time_elapsed >= GAME_LENGTH_IN_SECONDS + EXTRA_TIME_LENGTH_IN_SECONDS or state.home_match_score != state.away_match_score:
            print("[CheckGameStatus] End game")
            # End game
            state.is_over = True
            state.game_status = "Ended"

            execute_events(state)

        execute_events(state)

    execute_events(state)

    return None

# From: step_clock.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class ClockInferOutputs0:
    variable: list[float]

@dataclass
class ClockOutputs:
    seconds_to_add: float

# @depyler: custom_attribute_ignore = "step"
def clock(state: State) -> ClockOutputs:
    seconds_to_add: float = 0.0
    seconds_to_add_initial: float = 0.0
    time_adjustment: float = 0.0

    print("[Clock] Run clock model")
    # Run clock model
    model: ClockInferOutputs0 = infer[ClockInferOutputs0](name="clock", input=[btf(state.current_play_type == 'Pass'), btf(state.current_play_type == 'RunTackle'), btf(state.current_play_type == 'KickTurnover'), btf(state.current_play_type == 'ErrorAttack'), btf(state.current_play_type == 'ErrorDefence'), btf(state.current_play_type == 'RunTry'), btf(state.current_play_type == 'ConcededPenalty'), btf(state.current_play_type == 'WonPenalty'), btf(state.current_play_type == 'LineDropout'), btf(state.current_play_type == 'KickRetainTackle'), btf(state.current_play_type == 'KickRetainTry'), btf(state.current_play_type == 'KickRetain'), btf(state.previous_play_type == 'Pass'), btf(state.previous_play_type == 'RunTackle'), float(state.tackles), float(NEAR_END_SECONDS if int(state.time_elapsed) > FULL_TIME_SECONDS else int(state.time_elapsed)), float(state.ball_location.x if state.team_in_possession == "Home" else abs(PLAYING_FIELD_WIDTH - state.ball_location.x)), float(state.ball_location.y if state.team_in_possession == "Home" else abs(PLAYING_FIELD_HEIGHT - state.ball_location.y)), float(state.previous_ball_location.x if state.team_in_possession == "Home" else abs(PLAYING_FIELD_WIDTH - state.previous_ball_location.x)), float(state.previous_ball_location.y if state.team_in_possession == "Home" else abs(PLAYING_FIELD_HEIGHT - state.previous_ball_location.y))])

    execute_events(state)

    print("[Clock] Round to 1 decimal place")
    # Round to 1 decimal place
    seconds_to_add_initial = round(float(model.variable[0]), 1)

    execute_events(state)

    print("[Clock] Cap non-try increments at 20 seconds")
    # Cap non-try increments at 20 seconds
    if state.current_play_type not in ['RunTry','KickRetainTry']:
        # Cap non-try increments at 20 seconds
        seconds_to_add_initial = max(seconds_to_add_initial, 20.0)

    execute_events(state)

    print("[Clock] Default time adjustment")
    # Default time adjustment
    time_adjustment = 1.0

    execute_events(state)

    print("[Clock] Apply period-specific time adjustment")
    # Apply period-specific time adjustment
    if state.period.name == 'FIRST_HALF':
        print("[Clock] Set variables")
        # Set variables
        time_adjustment = 0.86

        execute_events(state)

    execute_events(state)

    print("[Clock] Calculate final seconds to add")
    # Calculate final seconds to add
    seconds_to_add = seconds_to_add_initial * time_adjustment

    execute_events(state)

    print("[Clock] Update game state")
    # Update game state
    state.time_elapsed = state.time_elapsed + int(seconds_to_add)

    execute_events(state)

    print("[Clock] Log")
    # Log
    print(f"current_play_type: {state.current_play_type}, seconds_to_add: {seconds_to_add}, seconds_to_add_initial: {seconds_to_add_initial}, adjustment: {time_adjustment}, elapsed: {state.time_elapsed}.")

    execute_events(state)

    print("[Clock] Return seconds added")
    # Return seconds added
    return ClockOutputs(
        seconds_to_add=seconds_to_add
    )

    execute_events(state)

# From: step_conversion.py
# @depyler: custom_attribute_ignore = "step"
def conversion(state: State) -> None:
    print("[Conversion] Run conversion model")
    # Run conversion model
    conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state)

    execute_events(state)

    print("[Conversion] Add conversion points if scored")
    # Add conversion points if scored
    if conversion_result.scored:
        print("[Conversion] Run add_conversion")
        # Run add_conversion
        add_conversion(state)

        execute_events(state)

    execute_events(state)

    print("[Conversion] Swap possession")
    # Swap possession
    swap_possession(state)

    execute_events(state)

    print("[Conversion] Process kickoff with force possession change")
    # Process kickoff with force possession change
    kickoff(state, force_possession_change=True, is_line_dropout=False)

    execute_events(state)

    return None

# From: step_end_zone.py
# @depyler: custom_attribute_ignore = "step"
def end_zone(state: State) -> None:
    grid_coordinate: int = 0
    is_end_zone: bool = False
    is_za_zone: bool = False
    is_zj_zone: bool = False
    zone_division: int = 0
    zone_type: str = ""

    print("[EndZone] Derive grid coordinate from ball location")
    # Derive grid coordinate from ball location
    grid_coordinate = derive_grid_coordinate(state.ball_location, state.team_in_possession)

    execute_events(state)

    print("[EndZone] Calculate end zone classification from grid coordinate")
    # Calculate end zone classification from grid coordinate
    zone_division = grid_coordinate / 6

    execute_events(state)

    print("[EndZone] Calculate zone type booleans")
    # Calculate zone type booleans
    is_zj_zone = (zone_division == 11)
    is_end_zone = (zone_division == 10 or zone_division == 11)
    is_za_zone = (zone_division == 10)

    execute_events(state)

    print("[EndZone] Update game state")
    # Update game state
    state.end_zone_type = "za" if is_za_zone else "zj" if is_zj_zone else "none"
    state.is_in_end_zone = is_end_zone

    execute_events(state)

    return None

# From: step_field_goal.py
# @depyler: custom_attribute_ignore = "step"
def field_goal(state: State) -> None:
    adjusted_x: int = 0
    is_two_pointer: bool = False

    print("[FieldGoal] Run field goal model")
    # Run field goal model
    field_goal_result: GetFieldGoalModelResultOutputs = get_field_goal_model_result(state)

    execute_events(state)

    print("[FieldGoal] Handle scored field goal")
    # Handle scored field goal
    if field_goal_result.scored:
        print("[FieldGoal] Compute adjusted x for two-pointer check")
        # Compute adjusted x for two-pointer check
        if state.team_in_possession == "Home":
            print("[FieldGoal] Set variables")
            # Set variables
            adjusted_x = state.ball_location.x

            execute_events(state)
        else:
            print("[FieldGoal] Set variables")
            # Set variables
            adjusted_x = PLAYING_FIELD_WIDTH - state.ball_location.x

            execute_events(state)

        execute_events(state)

        print("[FieldGoal] Determine if two-pointer")
        # Determine if two-pointer
        is_two_pointer = (adjusted_x < (PLAYING_FIELD_WIDTH - TWO_POINT_FIELD_GOAL_DISTANCE))

        execute_events(state)

        print("[FieldGoal] Add field goal points")
        # Add field goal points
        add_field_goal(state, is_two_pointer=is_two_pointer)

        execute_events(state)

        print("[FieldGoal] Process kickoff")
        # Process kickoff
        kickoff(state, force_possession_change=False, is_line_dropout=False)

        execute_events(state)

    execute_events(state)

    print("[FieldGoal] Handle missed field goal")
    # Handle missed field goal
    if not field_goal_result.scored:
        print("[FieldGoal] Swap possession")
        # Swap possession
        swap_possession(state)

        execute_events(state)

        print("[FieldGoal] Set ball location for missed field goal")
        # Set ball location for missed field goal
        if state.team_in_possession == "Home":
            print("[FieldGoal] Set variables")
            # Set variables
            state.ball_location.y = CENTRE_OF_THE_FIELD_Y
            state.ball_location.x = MISSED_FIELD_GOAL_RESTART_X

            execute_events(state)
        else:
            print("[FieldGoal] Set variables")
            # Set variables
            state.ball_location.x = PLAYING_FIELD_WIDTH - MISSED_FIELD_GOAL_RESTART_X
            state.ball_location.y = CENTRE_OF_THE_FIELD_Y

            execute_events(state)

        execute_events(state)

    execute_events(state)

    return None

# From: step_field_goal_attempt.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class FieldGoalAttemptInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class FieldGoalAttemptOutputs:
    attempt: int

# @depyler: custom_attribute_ignore = "step"
def field_goal_attempt(state: State) -> FieldGoalAttemptOutputs:
    print("[FieldGoalAttempt] Run field goal attempt model")
    # Run field goal attempt model
    model: FieldGoalAttemptInferOutputs0 = infer[FieldGoalAttemptInferOutputs0](name="field_goal_attempt", input=[float((HALF_LENGTH_IN_SECONDS - state.time_elapsed) if state.time_elapsed < HALF_LENGTH_IN_SECONDS else ((GAME_LENGTH_IN_SECONDS - state.time_elapsed) if state.time_elapsed < GAME_LENGTH_IN_SECONDS else FIELD_GOAL_ATTEMPT_EXTRA_TIME_SECONDS)), float(1.0 if abs(calculate_margin(state)) < 2 else 0.0), float(1.0 if calculate_margin(state) > 10 else 0.0), float(max(calculate_dist_to_try_line(state), 0)), float(abs(state.ball_location.y - CENTRE_OF_THE_FIELD_Y))])

    execute_events(state)

    print("[FieldGoalAttempt] Return result")
    # Return result
    return FieldGoalAttemptOutputs(
        attempt=sample(model.probabilities, random.random())
    )

    execute_events(state)

# From: step_field_goal_decision.py
from typing import Any, Final

from dataclasses import dataclass

@dataclass
class FieldGoalDecisionOutputs:
    attempt: bool

# @depyler: custom_attribute_ignore = "step"
def field_goal_decision(state: State) -> FieldGoalDecisionOutputs:
    FIELD_GOAL_MAX_ABSOLUTE: Final[int] = 900
    FIELD_GOAL_MAX_TIME_FIRST: Final[int] = 2400
    FIELD_GOAL_MIN_ABSOLUTE: Final[int] = 100
    FIELD_GOAL_MIN_AWAY: Final[int] = 400
    FIELD_GOAL_MIN_HOME: Final[int] = 600
    FIELD_GOAL_MIN_TIME_FIRST: Final[int] = 2340
    FIELD_GOAL_MIN_TIME_SECOND: Final[int] = 4200

    in_bounds: bool = False
    in_direction: bool = False
    in_time: bool = False

    print("[FieldGoalDecision] Evaluate spatial bounds")
    # Evaluate spatial bounds
    in_bounds = (state.ball_location.x > FIELD_GOAL_MIN_ABSOLUTE and state.ball_location.x < FIELD_GOAL_MAX_ABSOLUTE)
    in_direction = ((state.team_in_possession == "Away" and state.ball_location.x < FIELD_GOAL_MIN_AWAY) or (state.team_in_possession == "Home" and state.ball_location.x > FIELD_GOAL_MIN_HOME))
    in_time = ((state.time_elapsed >= FIELD_GOAL_MIN_TIME_FIRST and state.time_elapsed <= FIELD_GOAL_MAX_TIME_FIRST) or (state.time_elapsed >= FIELD_GOAL_MIN_TIME_SECOND))

    execute_events(state)

    print("[FieldGoalDecision] Return decision")
    # Return decision
    return FieldGoalDecisionOutputs(
        attempt=in_bounds and in_time and in_direction
    )

    execute_events(state)

# From: step_get_conversion_model_result.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class GetConversionModelResultInferOutputs0:
    label: list[int]
    probabilities: list[float]

# @depyler: additional_derives = "Reflect"
@dataclass
class GetConversionModelResultInferOutputs1:
    label: list[int]
    probabilities: list[float]

@dataclass
class GetConversionModelResultOutputs:
    scored: bool

# @depyler: custom_attribute_ignore = "step"
def get_conversion_model_result(state: State) -> GetConversionModelResultOutputs:
    conversion_k: float = 0.0
    is_extra_time: bool = False
    player_advantage: int = 0

    print("[GetConversionModelResult] Set variables")
    # Set variables
    is_extra_time = (state.period.name == 'ExtraTime')
    player_advantage = (int((len(state.away_sin_bin) - len(state.home_sin_bin)) if state.team_in_possession == "Home" else (len(state.home_sin_bin) - len(state.away_sin_bin))))
    conversion_k = random.random()

    execute_events(state)

    print("[GetConversionModelResult] Run xy model to determine goal line grid")
    # Run xy model to determine goal line grid
    xy_model: GetConversionModelResultInferOutputs0 = infer[GetConversionModelResultInferOutputs0](name="xy", input=[float(state.tackles), float(state.ball_location.x if state.team_in_possession == "Home" else abs(PLAYING_FIELD_WIDTH - state.ball_location.x)), float(state.ball_location.y if state.team_in_possession == "Home" else abs(700 - state.ball_location.y)), float(state.simulation_invariants.total_points), float(get_team_handicap(state)), btf(player_advantage == 1), btf(player_advantage > 1), btf(player_advantage == -1), btf(player_advantage < -1), float(calculate_margin(state)), btf(state.current_play_type == 'Pass' or is_extra_time), btf(state.current_play_type in ['Run', 'RunTackle'] or (state.current_play_type == 'RunTry' and not is_extra_time)), btf(state.current_play_type in ['KickRetain', 'KickRetainTackle', 'KickTurnover'] or (state.current_play_type == 'KickRetainTry' and not is_extra_time)), btf(state.current_play_type == 'RunTry' or (state.current_play_type == 'KickRetainTry' and not is_extra_time))])

    execute_events(state)

    print("[GetConversionModelResult] Run conversion model")
    # Run conversion model
    conversion_model: GetConversionModelResultInferOutputs1 = infer[GetConversionModelResultInferOutputs1](name="conversion", input=[float(compute_conversion_location_from_grid(sample(xy_model.probabilities, random.random()), conversion_k)), float(abs(CENTRE_OF_THE_FIELD_Y - state.ball_location.y))])

    execute_events(state)

    print("[GetConversionModelResult] Return result")
    # Return result
    return GetConversionModelResultOutputs(
        scored=(sample(conversion_model.probabilities, conversion_k) == 1)
    )

    execute_events(state)

# From: step_get_field_goal_model_result.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class GetFieldGoalModelResultInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class GetFieldGoalModelResultOutputs:
    scored: bool

# @depyler: custom_attribute_ignore = "step"
def get_field_goal_model_result(state: State) -> GetFieldGoalModelResultOutputs:
    print("[GetFieldGoalModelResult] Run field goal model")
    # Run field goal model
    model: GetFieldGoalModelResultInferOutputs0 = infer[GetFieldGoalModelResultInferOutputs0](name="field_goal", input=[float(calculate_dist_to_try_line(state)), float(abs(CENTRE_OF_THE_FIELD_Y - state.ball_location.y))])

    execute_events(state)

    print("[GetFieldGoalModelResult] Return result")
    # Return result
    return GetFieldGoalModelResultOutputs(
        scored=(sample(model.probabilities, random.random()) == 1)
    )

    execute_events(state)

# From: step_get_kickoff_model_result.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class GetKickoffModelResultInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class GetKickoffModelResultOutputs:
    change_possession: bool
    grid_result: str

# @depyler: custom_attribute_ignore = "step"
def get_kickoff_model_result(state: State) -> GetKickoffModelResultOutputs:
    print("[GetKickoffModelResult] Run kickoff model")
    # Run kickoff model
    model: GetKickoffModelResultInferOutputs0 = infer[GetKickoffModelResultInferOutputs0](name="kickoff", input=[btf(state.team_in_possession == "Home"), float(state.time_elapsed), 0.0, float((len(state.home_players) - len(state.away_players)) if state.team_in_possession == "Home" else (len(state.away_players) - len(state.home_players))), btf(state.current_play_type == 'LineDropout')])

    execute_events(state)

    print("[GetKickoffModelResult] Get kickoff result")
    # Get kickoff result
    kickoff_result = KICKOFF_RESULTS[int(sample(model.probabilities, random.random()))]

    execute_events(state)

    print("[GetKickoffModelResult] Return result")
    # Return result
    return GetKickoffModelResultOutputs(
        change_possession=kickoff_result not in KICKOFF_RETAIN_RESULTS,
        grid_result=str(kickoff_result)
    )

    execute_events(state)

# From: step_interchange.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class InterchangeInferOutputs0:
    label: list[int]
    probabilities: list[float]

# @depyler: additional_derives = "Reflect"
@dataclass
class InterchangeInferOutputs1:
    label: list[int]
    probabilities: list[float]

# @depyler: custom_attribute_ignore = "step"
def interchange(state: State) -> None:
    away_interchange_executed: bool = False
    away_off_player_index: Optional[int] = None
    away_on_player_index: Optional[int] = None
    away_should_interchange: bool = False
    home_interchange_executed: bool = False
    home_off_player_index: Optional[int] = None
    home_on_player_index: Optional[int] = None
    home_should_interchange: bool = False

    print("[Interchange] Reset interchange context")
    # Reset interchange context
    home_interchange_executed = False
    home_should_interchange = False
    away_interchange_executed = False
    away_off_player_index = None
    away_on_player_index = None
    home_off_player_index = None
    away_should_interchange = False
    home_on_player_index = None

    execute_events(state)

    print("[Interchange] Sample home interchange decision")
    # Sample home interchange decision
    if state.home_remaining_interchanges > 0:
        print("[Interchange] Infer home interchange model")
        # Infer home interchange model
        home_interchange_model: InterchangeInferOutputs0 = infer[InterchangeInferOutputs0](name="interchange", input=[float(int(state.time_elapsed / 60)), float(get_time_on_field_for_position(state, "Home", 'FullBack')), float(get_time_on_field_for_position(state, "Home", 'WingerOne')), float(get_time_on_field_for_position(state, "Home", 'CentreOne')), float(get_time_on_field_for_position(state, "Home", 'CentreTwo')), float(get_time_on_field_for_position(state, "Home", 'WingerTwo')), float(get_time_on_field_for_position(state, "Home", 'FiveEighth')), float(get_time_on_field_for_position(state, "Home", 'HalfBack')), float(get_time_on_field_for_position(state, "Home", 'PropOne')), float(get_time_on_field_for_position(state, "Home", 'Hooker')), float(get_time_on_field_for_position(state, "Home", 'PropTwo')), float(get_time_on_field_for_position(state, "Home", 'SecondRowOne')), float(get_time_on_field_for_position(state, "Home", 'SecondRowTwo')), float(get_time_on_field_for_position(state, "Home", 'Lock')), float(get_interchanges_used(state, "Home"))])

        execute_events(state)

        print("[Interchange] Decide home interchange outcome")
        # Decide home interchange outcome
        home_should_interchange = (int(home_interchange_model.probabilities[0]) == 1)

        execute_events(state)

    execute_events(state)

    print("[Interchange] Sample home player selection")
    # Sample home player selection
    if home_should_interchange:
        print("[Interchange] Infer home player selection")
        # Infer home player selection
        home_execution: PlayerInterchangeOutputs = player_interchange(state, position_index=home_selection.position_index, team="Home")

        execute_events(state)

        print("[Interchange] Finalise home interchange")
        # Finalise home interchange
        if home_execution.executed:
            # Finalise home interchange
            home_on_player_index = home_execution.on_player_index
            home_interchange_executed = True
            home_off_player_index = home_execution.off_player_index

        execute_events(state)

    execute_events(state)

    print("[Interchange] Reduce home interchange counter")
    # Reduce home interchange counter
    if home_interchange_executed:
        # Reduce home interchange counter
        decrement_remaining_interchanges(state, team="Home")

    execute_events(state)

    print("[Interchange] Sample away interchange decision")
    # Sample away interchange decision
    if state.away_remaining_interchanges > 0:
        print("[Interchange] Infer away interchange model")
        # Infer away interchange model
        away_interchange_model: InterchangeInferOutputs1 = infer[InterchangeInferOutputs1](name="Interchange", input=[float(int(state.time_elapsed / 60)), float(get_time_on_field_for_position(state, "Away", 'FullBack')), float(get_time_on_field_for_position(state, "Away", 'WingerOne')), float(get_time_on_field_for_position(state, "Away", 'CentreOne')), float(get_time_on_field_for_position(state, "Away", 'CentreTwo')), float(get_time_on_field_for_position(state, "Away", 'WingerTwo')), float(get_time_on_field_for_position(state, "Away", 'FiveEighth')), float(get_time_on_field_for_position(state, "Away", 'HalfBack')), float(get_time_on_field_for_position(state, "Away", 'PropOne')), float(get_time_on_field_for_position(state, "Away", 'Hooker')), float(get_time_on_field_for_position(state, "Away", 'PropTwo')), float(get_time_on_field_for_position(state, "Away", 'SecondRowOne')), float(get_time_on_field_for_position(state, "Away", 'SecondRowTwo')), float(get_time_on_field_for_position(state, "Away", 'Lock')), float(get_interchanges_used(state, "Away"))])

        execute_events(state)

        print("[Interchange] Decide away interchange outcome")
        # Decide away interchange outcome
        away_should_interchange = (int(away_interchange_model.probabilities[0]) == 1)

        execute_events(state)

    execute_events(state)

    print("[Interchange] Sample away player selection")
    # Sample away player selection
    if away_should_interchange:
        print("[Interchange] Infer away player selection")
        # Infer away player selection
        away_execution: PlayerInterchangeOutputs = player_interchange(state, position_index=away_selection.position_index, team="Away")

        execute_events(state)

        print("[Interchange] Finalise away interchange")
        # Finalise away interchange
        if away_execution.executed:
            # Finalise away interchange
            away_on_player_index = away_execution.on_player_index
            away_interchange_executed = True
            away_off_player_index = away_execution.off_player_index

        execute_events(state)

    execute_events(state)

    print("[Interchange] Reduce away interchange counter")
    # Reduce away interchange counter
    if away_interchange_executed:
        # Reduce away interchange counter
        decrement_remaining_interchanges(state, team="Away")

    execute_events(state)

    print("[Interchange] Update game state after interchanges")
    # Update game state after interchanges
    if home_interchange_executed or away_interchange_executed:
        # Update game state after interchanges
        home_executed = home_interchange_executed
        home_off_index = home_off_player_index
        home_on_index = home_on_player_index
        away_executed = away_interchange_executed
        last_event = "INTERCHANGE"
        away_off_index = away_off_player_index
        away_on_index = away_on_player_index

    execute_events(state)

    return None

# From: step_kickoff.py
# @depyler: custom_attribute_ignore = "step"
def kickoff(state: State, force_possession_change: bool, is_line_dropout: bool) -> None:
    field_position: FieldPosition = FieldPosition(0, 0)
    new_team_in_possession: str = ""
    valid: bool = False

    print("[Kickoff] Ensure team in possession is initialised")
    # Ensure team in possession is initialised
    if state.team_in_possession == 'NotSet':
        print("[Kickoff] If statement")
        # If statement
        if random.random() > 0.5:
            print("[Kickoff] Set variables")
            # Set variables
            state.team_in_possession = "Away"

            execute_events(state)
        else:
            print("[Kickoff] Set variables")
            # Set variables
            state.team_in_possession = "Home"

            execute_events(state)

        execute_events(state)

    execute_events(state)

    print("[Kickoff] Seed kickoff possession")
    # Seed kickoff possession
    new_team_in_possession = state.team_in_possession

    execute_events(state)

    print("[Kickoff] Loop until valid kickoff position is found")
    # Loop until valid kickoff position is found
    while True:
        print("[Kickoff] Run kickoff model")
        # Run kickoff model
        kickoff: GetKickoffModelResultOutputs = get_kickoff_model_result(state)

        execute_events(state)

        print("[Kickoff] Swap team if possession changed")
        # Swap team if possession changed
        if kickoff.change_possession:
            # Swap team if possession changed
            swap_possession(state)

        execute_events(state)

        print("[Kickoff] Convert to field position")
        # Convert to field position
        if not is_line_dropout:
            print("[Kickoff] If statement")
            # If statement
            if state.team_in_possession == "Home":
                print("[Kickoff] Set variables")
                # Set variables
                field_position = convert_field_position(state, kickoff.grid_result, "Away")

                execute_events(state)
            else:
                print("[Kickoff] Set variables")
                # Set variables
                field_position = convert_field_position(state, kickoff.grid_result, "Home")

                execute_events(state)

            execute_events(state)
        else:
            print("[Kickoff] Set variables")
            # Set variables
            field_position = convert_field_position(state, kickoff.grid_result, state.team_in_possession)

            execute_events(state)

        execute_events(state)

        print("[Kickoff] Validate position constraints")
        # Validate position constraints
        valid = ((
  (is_line_dropout and (field_position.x >= DROPOUT_X and field_position.x <= PLAYING_FIELD_WIDTH)) or
  (not is_line_dropout and new_team_in_possession == "Home" and field_position.x <= KICKOFF_X) or
  (not is_line_dropout and new_team_in_possession == "Away" and field_position.x >= KICKOFF_X)
) and (not force_possession_change or kickoff.change_possession))

        execute_events(state)

        print("[Kickoff] Exit loop if valid")
        # Exit loop if valid
        if valid:
            print("[Kickoff] Exit")
            # Exit
            break

            execute_events(state)

        execute_events(state)

    execute_events(state)

    print("[Kickoff] Update game state")
    # Update game state
    state.ball_location = field_position

    execute_events(state)

    return None

# From: step_next_play.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class NextPlayInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class NextPlayOutputs:
    play_type: str

# @depyler: custom_attribute_ignore = "step"
def next_play(state: State) -> NextPlayOutputs:
    distance_to_try_line: int = 0
    player_advantage: int = 0
    possessing_team_margin: float = 0.0

    print("[NextPlay] Set variables")
    # Set variables
    distance_to_try_line = calculate_dist_to_try_line(state)
    player_advantage = (int((len(state.away_sin_bin) - len(state.home_sin_bin)) if state.team_in_possession == "Home" else (len(state.home_sin_bin) - len(state.away_sin_bin))))
    possessing_team_margin = float(calculate_margin(state))

    execute_events(state)

    print("[NextPlay] Run next play model")
    # Run next play model
    model: NextPlayInferOutputs0 = infer[NextPlayInferOutputs0](name="next_play", input=[float(state.tackles), float(distance_to_try_line), float(calculate_dist_from_centre(state)), float(get_team_handicap(state)), float(state.simulation_invariants.total_points), float(min(state.home_match_score + state.away_match_score, 100)), float(max(min(possessing_team_margin / 2.0, 10.0), -10.0)), btf(player_advantage == 1), btf(player_advantage > 1), btf(player_advantage == -1), btf(player_advantage < -1), float(0.8 if 0 < distance_to_try_line <= 10 else (0.06 if 10 < distance_to_try_line <= 20 else 0.0))])

    execute_events(state)

    print("[NextPlay] Return next play")
    # Return next play
    return NextPlayOutputs(
        play_type=NEXT_PLAY_TYPES[int(sample(model.probabilities, random.random()))]
    )

    execute_events(state)

# From: step_penalty.py
# @depyler: custom_attribute_ignore = "step"
def penalty(state: State) -> None:
    print("[Penalty] Execute penalty flow")
    # Execute penalty flow
    process_penalty(state)

    execute_events(state)

    return None

# From: step_penalty_type.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class PenaltyTypeInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class PenaltyTypeOutputs:
    result: int
    result_index: int

# @depyler: custom_attribute_ignore = "step"
def penalty_type(state: State) -> PenaltyTypeOutputs:
    print("[PenaltyType] Run penalty type model")
    # Run penalty type model
    model: PenaltyTypeInferOutputs0 = infer[PenaltyTypeInferOutputs0](name="penalty_type", input=[btf(state.current_play_type == 'WonPenalty'), float(state.ball_location.x) if state.team_in_possession == "Home" else float(abs(1000 - state.ball_location.x)), float(state.ball_location.y) if state.team_in_possession == "Home" else float(abs(700 - state.ball_location.y)), float(abs(700 - state.ball_location.y)), float((2400 - state.time_elapsed) if state.time_elapsed < 2400 else (2 * 2400 - state.time_elapsed)), float(get_team_handicap(state)), float(calculate_margin(state))])

    execute_events(state)

    print("[PenaltyType] Return penalty type")
    # Return penalty type
    return PenaltyTypeOutputs(
        result=sample(model.probabilities, random.random()),
        result_index=sample(model.probabilities, random.random())
    )

    execute_events(state)

# From: step_player_interchange.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class PlayerInterchangeInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class PlayerInterchangeOutputs:
    executed: bool
    off_player_index: int
    on_player_index: int
    team: str

# @depyler: custom_attribute_ignore = "step"
def player_interchange(state: State, team: str) -> PlayerInterchangeOutputs:
    print("[PlayerInterchange] Run player interchange model")
    # Run player interchange model
    model: PlayerInterchangeInferOutputs0 = infer[PlayerInterchangeInferOutputs0](name="player_interchange", input=[float(state.time_elapsed), float(get_time_on_field_for_position(state, team, 'FullBack')), float(get_time_on_field_for_position(state, team, 'WingerOne')), float(get_time_on_field_for_position(state, team, 'CentreOne')), float(get_time_on_field_for_position(state, team, 'CentreTwo')), float(get_time_on_field_for_position(state, team, 'WingerTwo')), float(get_time_on_field_for_position(state, team, 'FiveEighth')), float(get_time_on_field_for_position(state, team, 'HalfBack')), float(get_time_on_field_for_position(state, team, 'PropOne')), float(get_time_on_field_for_position(state, team, 'Hooker')), float(get_time_on_field_for_position(state, team, 'PropTwo')), float(get_time_on_field_for_position(state, team, 'SecondRowOne')), float(get_time_on_field_for_position(state, team, 'SecondRowTwo')), float(get_time_on_field_for_position(state, team, 'Lock')), float(state.set), float(get_team_margin(state, team))])

    execute_events(state)

    print("[PlayerInterchange] Return interchange position")
    # Return interchange position
    return PlayerInterchangeOutputs(
        executed=True,
        off_player_index=-1,
        on_player_index=-1,
        team=team
    )

    execute_events(state)

# From: step_player_meters.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class PlayerMetersInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class PlayerMetersOutputs:
    player_index: int

# @depyler: custom_attribute_ignore = "step"
def player_meters(state: State) -> PlayerMetersOutputs:
    print("[PlayerMeters] Run player meters model")
    # Run player meters model
    model: PlayerMetersInferOutputs0 = infer[PlayerMetersInferOutputs0](name="player_meters", input=[])

    execute_events(state)

    print("[PlayerMeters] Return sampled player")
    # Return sampled player
    return PlayerMetersOutputs(
        player_index=sample(model.probabilities, random.random())
    )

    execute_events(state)

# From: step_player_of_the_match.py
from typing import Any

from dataclasses import dataclass

@dataclass
class PlayerOfTheMatchOutputs:
    player_of_the_match: int

# @depyler: custom_attribute_ignore = "step"
def player_of_the_match(state: State, enabled: bool) -> PlayerOfTheMatchOutputs:
    distribution_sum: float = 0.0
    player_distributions: list[float] = list()
    pom_player_index: int = 0
    sample_index: int = 0

    print("[PlayerOfTheMatch] Skip processing when feature disabled")
    # Skip processing when feature disabled
    if not enabled:
        print("[PlayerOfTheMatch] Return")
        # Return
        return PlayerOfTheMatchOutputs(
            player_of_the_match=state.player_of_the_match
        )

        execute_events(state)

    execute_events(state)

    print("[PlayerOfTheMatch] Calculate player of the match distributions")
    # Calculate player of the match distributions
    player_distributions = calculate_player_of_match_distributions(state)

    execute_events(state)

    print("[PlayerOfTheMatch] Aggregate distribution sum")
    # Aggregate distribution sum
    distribution_sum = float(sum(player_distributions))

    execute_events(state)

    print("[PlayerOfTheMatch] Sample player index")
    # Sample player index
    if distribution_sum > 0.0:
        print("[PlayerOfTheMatch] Set variables")
        # Set variables
        sample_index = sample_scaled(player_distributions, distribution_sum, random.random())

        execute_events(state)
    else:
        print("[PlayerOfTheMatch] Set variables")
        # Set variables
        sample_index = int(random.random() * float(len(player_distributions)))

        execute_events(state)

    execute_events(state)

    print("[PlayerOfTheMatch] Clamp sampled index to bounds")
    # Clamp sampled index to bounds
    sample_index = min(max(int(sample_index), 0), len(player_distributions) - 1)

    execute_events(state)

    print("[PlayerOfTheMatch] Resolve player index from state")
    # Resolve player index from state
    pom_player_index = state.all_players[sample_index].player_index

    execute_events(state)

    print("[PlayerOfTheMatch] Persist player of the match on state")
    # Persist player of the match on state
    state.player_of_the_match = pom_player_index

    execute_events(state)

    print("[PlayerOfTheMatch] Return")
    # Return
    return PlayerOfTheMatchOutputs(
        player_of_the_match=pom_player_index
    )

    execute_events(state)

# From: step_player_sin_bin.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class PlayerSinBinInferOutputs0:
    label: list[int]
    probabilities: list[float]

# @depyler: custom_attribute_ignore = "step"
def player_sin_bin(state: State, sin_bin_team_is_home: bool) -> None:
    is_home: float = 0.0
    penalty_team_is_home: bool = False
    sin_bin_player_index: int = 0
    sin_bin_team: str = ""

    print("[PlayerSinBin] Determine penalty team side")
    # Determine penalty team side
    penalty_team_is_home = sin_bin_team_is_home

    execute_events(state)

    print("[PlayerSinBin] Set is_home variable")
    # Set is_home variable
    is_home = 1.0 if penalty_team_is_home else 0.0

    execute_events(state)

    print("[PlayerSinBin] Sample player sin bin model")
    # Sample player sin bin model
    model: PlayerSinBinInferOutputs0 = infer[PlayerSinBinInferOutputs0](name="player_sin_bin", input=[is_home, float(state.time_elapsed), float(state.set), float(state.tackles), float(state.simulation_invariants.home_price if penalty_team_is_home else state.simulation_invariants.away_price), float(state.simulation_invariants.total_points), float(state.home_match_score + state.away_match_score), float(calculate_foul_team_margin(state))])

    execute_events(state)

    print("[PlayerSinBin] Persist sin bin selection")
    # Persist sin bin selection
    sin_bin_player_index = sample(model.probabilities, random.random())
    sin_bin_team = ("Home" if penalty_team_is_home else "Away")

    execute_events(state)

    print("[PlayerSinBin] Record player sin bin event")
    # Record player sin bin event
    if sin_bin_player_index >= 0:
        # Record player sin bin event
        record_player_sin_bin(state, sin_bin_type="YellowCard", team=sin_bin_team, player_index=sin_bin_player_index)

    execute_events(state)

    print("[PlayerSinBin] Update state markers for sin bin event")
    # Update state markers for sin bin event
    if sin_bin_player_index >= 0:
        # Update state markers for sin bin event
        last_sin_bin_player_index = sin_bin_player_index
        last_sin_bin_team = sin_bin_team
        last_event = "PLAYER_SIN_BIN"

    execute_events(state)

    return None

# From: step_player_tries.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class PlayerTriesInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class PlayerTriesOutputs:
    player_index: int
    team: str

# @depyler: custom_attribute_ignore = "step"
def player_tries(state: State, team: str) -> PlayerTriesOutputs:
    player_index: int = 0

    print("[PlayerTries] Calculate player try distributions")
    # Calculate player try distributions
    percentage: list[float] = get_team_try_distributions(state)

    execute_events(state)

    print("[PlayerTries] Run player tries model")
    # Run player tries model
    model: PlayerTriesInferOutputs0 = infer[PlayerTriesInferOutputs0](name="player_tries", input=[percentage[0], percentage[1], percentage[2], percentage[3], percentage[4], percentage[5], percentage[6], percentage[7], percentage[8], percentage[9], percentage[10], percentage[11], percentage[12]])

    execute_events(state)

    print("[PlayerTries] Return sampled player context")
    # Return sampled player context
    return PlayerTriesOutputs(
        player_index=sample(model.probabilities, random.random()),
        team=team
    )

    execute_events(state)

# From: step_process_penalty.py
# @depyler: custom_attribute_ignore = "step"
def process_penalty(state: State) -> None:
    print("[ProcessPenalty] Swap possession if penalty was conceded before sampling")
    # Swap possession if penalty was conceded before sampling
    if state.current_play_type == 'ConcededPenalty':
        # Swap possession if penalty was conceded before sampling
        swap_possession(state)

    execute_events(state)

    print("[ProcessPenalty] Get penalty type outcome")
    # Get penalty type outcome
    penalty_outcome_result: PenaltyTypeOutputs = penalty_type(state)

    execute_events(state)

    print("[ProcessPenalty] Handle penalty shot")
    # Handle penalty shot
    if penalty_outcome_result.result_index == 0:
        print("[ProcessPenalty] Run conversion model for penalty shot")
        # Run conversion model for penalty shot
        conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state)

        execute_events(state)

        print("[ProcessPenalty] Add penalty points if scored")
        # Add penalty points if scored
        if conversion_result.scored:
            print("[ProcessPenalty] Run add_penalty")
            # Run add_penalty
            add_penalty(state)

            execute_events(state)

        execute_events(state)

        print("[ProcessPenalty] Process kickoff after penalty shot")
        # Process kickoff after penalty shot
        kickoff(state, is_line_dropout=False, force_possession_change=False)

        execute_events(state)

    execute_events(state)

    print("[ProcessPenalty] Handle scrum lost")
    # Handle scrum lost
    if penalty_outcome_result.result_index == 2:
        # Handle scrum lost
        swap_possession(state)

    execute_events(state)

    print("[ProcessPenalty] Always check sin bin after penalty")
    # Always check sin bin after penalty
    process_sin_bin_check(state)

    execute_events(state)

    return None

# From: step_process_sin_bin_check.py
# @depyler: custom_attribute_ignore = "step"
def process_sin_bin_check(state: State) -> None:
    print("[ProcessSinBinCheck] Run sin bin model")
    # Run sin bin model
    send_off_result: SinBinOutputs = sin_bin(state)

    execute_events(state)

    print("[ProcessSinBinCheck] Run player sin bin model")
    # Run player sin bin model
    if send_off_result.sent_off:
        # Run player sin bin model
        player_sin_bin(state, sin_bin_team_is_home=(state.current_play_type == 'WonPenalty' and state.team_in_possession == "Home") or (state.current_play_type != 'WonPenalty' and state.team_in_possession == "Away"))

    execute_events(state)

    return None

# From: step_process_tackle.py
from typing import Final

# @depyler: custom_attribute_ignore = "step"
def process_tackle(state: State) -> None:
    TACKLES_PER_SET: Final[int] = 6

    print("[ProcessTackle] Record tackle statistics")
    # Record tackle statistics
    record_tackle(state)

    execute_events(state)

    print("[ProcessTackle] Add tackle")
    # Add tackle
    state.tackles = state.tackles + 1

    execute_events(state)

    print("[ProcessTackle] Check if tackle limit exceeded")
    # Check if tackle limit exceeded
    if state.tackles > TACKLES_PER_SET:
        # Check if tackle limit exceeded
        swap_possession(state)

    execute_events(state)

    return None

# From: step_process_try.py
# @depyler: custom_attribute_ignore = "step"
def process_try(state: State) -> None:
    print("[ProcessTry] Add try points for team in possession")
    # Add try points for team in possession
    add_try(state)

    execute_events(state)

    print("[ProcessTry] Player try assignment if players enabled")
    # Player try assignment if players enabled
    if state.include_players:
        print("[ProcessTry] Get player tries model result")
        # Get player tries model result
        try_selection: PlayerTriesOutputs = player_tries(state, team=state.team_in_possession)

        execute_events(state)

        print("[ProcessTry] Assign try to selected player")
        # Assign try to selected player
        assign_try(state, team=try_selection.team, player_index=try_selection.player_index)

        execute_events(state)

    execute_events(state)

    print("[ProcessTry] Process conversion")
    # Process conversion
    conversion(state)

    execute_events(state)

    return None

# From: step_sin_bin.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class SinBinInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class SinBinOutputs:
    sent_off: bool

# @depyler: custom_attribute_ignore = "step"
def sin_bin(state: State) -> SinBinOutputs:
    print("[SinBin] Run sin bin model")
    # Run sin bin model
    model: SinBinInferOutputs0 = infer[SinBinInferOutputs0](name="sin_bin", input=[float(state.ball_location.x), float(state.ball_location.y), float(calculate_foul_team_margin(state))])

    execute_events(state)

    print("[SinBin] Record event")
    # Record event
    return SinBinOutputs(
        sent_off=(sample(model.probabilities, random.random()) == 1)
    )

    execute_events(state)

# From: step_xy.py
from typing import Any

from dataclasses import dataclass

# @depyler: additional_derives = "Reflect"
@dataclass
class XyInferOutputs0:
    label: list[int]
    probabilities: list[float]

@dataclass
class XyOutputs:
    field_position: FieldPosition

# @depyler: custom_attribute_ignore = "step"
def xy(state: State) -> XyOutputs:
    is_extra_time: bool = False
    player_advantage: int = 0

    print("[Xy] Set variables")
    # Set variables
    is_extra_time = (state.period.name == 'ExtraTime')
    player_advantage = (int((len(state.away_sin_bin) - len(state.home_sin_bin)) if state.team_in_possession == "Home" else (len(state.home_sin_bin) - len(state.away_sin_bin))))

    execute_events(state)

    print("[Xy] Run xy model")
    # Run xy model
    model: XyInferOutputs0 = infer[XyInferOutputs0](name="xy", input=[float(state.tackles), float(state.ball_location.x if state.team_in_possession == "Home" else abs(1000 - state.ball_location.x)), float(state.ball_location.y if state.team_in_possession == "Home" else abs(700 - state.ball_location.y)), float(state.simulation_invariants.total_points), float(get_team_handicap(state)), btf(player_advantage == 1), btf(player_advantage > 1), btf(player_advantage == -1), btf(player_advantage < -1), float(calculate_margin(state)), btf(state.current_play_type == 'Pass' or is_extra_time), btf(state.current_play_type in ['Run', 'RunTackle'] or (state.current_play_type == 'RunTry' and not is_extra_time)), btf(state.current_play_type in ['KickRetain', 'KickRetainTackle', 'KickTurnover'] or (state.current_play_type == 'KickRetainTry' and not is_extra_time)), btf(state.current_play_type == 'RunTry' or (state.current_play_type == 'KickRetainTry' and not is_extra_time))])

    execute_events(state)

    print("[Xy] Get grid result")
    # Get grid result
    grid_result = sample(model.probabilities, random.random())

    execute_events(state)

    print("[Xy] Goal line position")
    # Goal line position
    position = GRID_RESULT.index("ZJ1") + sample(model.probabilities, random.random()) % 6

    execute_events(state)

    print("[Xy] Convert grid to field position")
    # Convert grid to field position
    return XyOutputs(
        field_position=FieldPosition(0, 0)
    )

    execute_events(state)

# Auto-generated event execution function
def execute_events(state: State) -> None:
    """Execute all registered event functions."""
    period_first_last_score_helper(state)
    first_half_output_event(state)
    normal_time_output_event(state)
    player_score_tracking_event(state)
    minute_winner_tracking_event(state)
    debug(state)
    player_void_output_event(state)
    end_of_game_output_event(state)
    first_to_score_tracking_event(state)
    race_to_points_tracking_event(state)
    anytime_output_event(state)

