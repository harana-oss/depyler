#[doc = "// NOTE: Map Python module 'state'()"]
#[doc = "// NOTE: Map Python module 'functions'()"]
#[doc = "// NOTE: Map Python module '__future__'()"]
#[doc = "// NOTE: Map Python module '__future__'()"]
#[doc = "// NOTE: Map Python module 'sin_bin'()"]
#[doc = "// NOTE: Map Python module 'team'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
#[doc = "// NOTE: Map Python module 'dataclasses'()"]
use rand as random;
use std::f64 as math;
pub const CENTRE_OF_THE_FIELD_X: i32 = 500;
pub const CENTRE_OF_THE_FIELD_Y: i32 = 350;
pub const CONVERSION_POINTS: i32 = 2;
pub const DROPOUT_X: i32 = 0;
pub const DROPOUT_Y: i32 = 350;
pub const EXTRA_TIME_INDEX: i32 = 2;
pub const EXTRA_TIME_LENGTH_IN_SECONDS: i32 = 600;
pub const FIELD_GOAL_ATTEMPT_EXTRA_TIME_SECONDS: i32 = 180;
pub const FIELD_GOAL_MAX_ABSOLUTE: i32 = 900;
pub const FIELD_GOAL_MAX_TIME_FIRST: i32 = 2400;
pub const FIELD_GOAL_MIN_ABSOLUTE: i32 = 100;
pub const FIELD_GOAL_MIN_AWAY: i32 = 400;
pub const FIELD_GOAL_MIN_HOME: i32 = 600;
pub const FIELD_GOAL_MIN_TIME_FIRST: i32 = 2340;
pub const FIELD_GOAL_MIN_TIME_SECOND: i32 = 4200;
pub const FIELD_GOAL_PARAM_EXTRA_TIME_VALUE: i32 = 25;
pub const FIELD_GOAL_POINTS: i32 = 1;
pub const FIRST_EIGHT_MINUTES_SECONDS: i32 = 480;
pub const FIRST_HALF_INDEX: i32 = 0;
pub const FULL_TIME_SECONDS: i32 = 4800;
pub const GAME_LENGTH_IN_SECONDS: i32 = 4800;
pub const GOAL_LINE_GRID_START_INDEX: i32 = 60;
pub const GOAL_LINE_X_WIDTH: i32 = 20;
pub const GOAL_LINE_Y_HEIGHT_INCREMENT: i32 = 113;
pub const GOAL_LINE_Y_WIDTH: f64 = 23.34;
pub const HALF_LENGTH_IN_SECONDS: i32 = 2400;
pub const HOOKER_METRES_PER_MINUTE: f64 = 0.75057377;
pub const HOOKER_TACKLES_PER_MINUTE: f64 = 0.547090164;
pub const HOOKER_TRIES_PER_MINUTE: f64 = 0.00232582;
pub const INITIAL_SOLVER_HANDICAP: i32 = -3;
pub const INITIAL_SOLVER_TOTAL_POINTS: i32 = 45;
pub const KICKOFF_X: i32 = 500;
pub const KICKOFF_Y: i32 = 350;
pub const LAST_EIGHT_MINUTES_SECONDS: i32 = 4500;
pub const LAST_MINUTE_SECONDS: i32 = 60;
pub const LAST_TEN_MINUTES_SECONDS: i32 = 600;
pub const LOCK_METRES_PER_MINUTE: f64 = 2.519467213;
pub const LOCK_TACKLES_PER_MINUTE: f64 = 0.563770492;
pub const LOCK_TRIES_PER_MINUTE: f64 = 0.001212746;
pub const MAX_INTERCHANGES: i32 = 8;
pub const MIN_POM_WEIGHT: f64 = 0.0001;
pub const MISSED_FIELD_GOAL_RESTART_X: i32 = 200;
pub const NEAR_END_SECONDS: i32 = 4700;
pub const NUDGES_PERCENT: f64 = 1.0;
pub const NUMBER_OF_INTERCHANGE_PLAYERS: i32 = 4;
pub const NUMBER_OF_SIMULATIONS_DEFAULT: i32 = 40000;
pub const NUM_PERIODS: i32 = 4;
pub const NUM_PLAYERS_PER_TEAM: i32 = 17;
pub const NUM_STARTING_PLAYERS_PER_TEAM: i32 = 13;
pub const PENALTY_POINTS: i32 = 2;
pub const PLAYING_FIELD_BASE: i32 = 0;
pub const PLAYING_FIELD_HEIGHT: i32 = 700;
pub const PLAYING_FIELD_WIDTH: i32 = 1000;
pub const PROP_METRES_PER_MINUTE: f64 = 2.519467213;
pub const PROP_TACKLES_PER_MINUTE: f64 = 0.563770492;
pub const PROP_TRIES_PER_MINUTE: f64 = 0.001212746;
pub const SECOND_HALF_INDEX: i32 = 1;
pub const SOLVER_INITIAL_GRADIENT_NUMBER_OF_SIMULATIONS: i32 = 15000;
pub const STARTING_TACKLE: i32 = 1;
pub const TACKLES_PER_SET: i32 = 6;
pub const TRY_POINTS: i32 = 4;
pub const TRY_SCORER_DEFAULT_INDEXES: i32 = 1;
pub const TRY_SCORER_MATCH_INDEXES: i32 = 15;
pub const TWO_POINT_FIELD_GOAL_DISTANCE: i32 = 400;
pub const TWO_POINT_FIELD_GOAL_POINTS: i32 = 2;
pub const YELLOW_CARD_SECONDS: i32 = 600;
lazy_static! {
    pub static ref CONVERSION_X_VALUES: Vec<i32> = vec![
        769, 773, 779, 788, 801, 823, 845, 866, 878, 890, 890, 882, 868, 847, 825, 801, 786, 779,
        771, 769
    ];
    pub static ref GOAL_LINE_Y_VALUES: Vec<(i32, i32)> = vec![
        (0, 113),
        (113, 226),
        (226, 339),
        (339, 452),
        (452, 565),
        (565, 678),
        (678, 700)
    ];
    pub static ref GOAL_LINE_ZONE_WEIGHTS: Vec<Vec<f64>> = vec![
        vec![
            0.338255506,
            0.161521868,
            0.028635414,
            0.011633486,
            0.008945708,
            0.063982141,
            0.059955556,
            0.028635414,
            0.008948757,
            0.003132014,
            0.040716185,
            0.043848199,
            0.022818671,
            0.016107357,
            0.0035793,
            0.038031456,
            0.038478742,
            0.024161543,
            0.013869913,
            0.005369458,
            0.015660071,
            0.012324,
            0.005722,
            0.003967,
            0.0017
        ],
        vec![
            0.006393969,
            0.065970006,
            0.021313621,
            0.033491996,
            0.005074616,
            0.07510408,
            0.08931277,
            0.02841738,
            0.039581769,
            0.008118916,
            0.082209011,
            0.099462002,
            0.042627242,
            0.038566612,
            0.008118916,
            0.061910548,
            0.079163538,
            0.031462853,
            0.024357922,
            0.004059458,
            0.050746158,
            0.049731001,
            0.035522311,
            0.016239005,
            0.003044301
        ],
        vec![
            0.04926153,
            0.045038504,
            0.028853258,
            0.021815592,
            0.00492562,
            0.065446776,
            0.068261416,
            0.027445938,
            0.020408272,
            0.00281464,
            0.071077122,
            0.064743116,
            0.023222912,
            0.016186312,
            0.00281464,
            0.038705564,
            0.033075218,
            0.031667898,
            0.021111932,
            0.00211098,
            0.10696698,
            0.098521994,
            0.084447728,
            0.067557756,
            0.0035183
        ],
        vec![
            0.10696698,
            0.098521994,
            0.084447728,
            0.067557756,
            0.0035183,
            0.038705564,
            0.033075218,
            0.031667898,
            0.021111932,
            0.00211098,
            0.071077122,
            0.064743116,
            0.023222912,
            0.016186312,
            0.00281464,
            0.065446776,
            0.068261416,
            0.027445938,
            0.020408272,
            0.00281464,
            0.04926153,
            0.045038504,
            0.028853258,
            0.021815592,
            0.00492562
        ],
        vec![
            0.050746158,
            0.049731001,
            0.035522311,
            0.016239005,
            0.003044301,
            0.061910548,
            0.079163538,
            0.031462853,
            0.024357922,
            0.004059458,
            0.082209011,
            0.099462002,
            0.042627242,
            0.038566612,
            0.008118916,
            0.07510408,
            0.08931277,
            0.02841738,
            0.039581769,
            0.008118916,
            0.006393969,
            0.065970006,
            0.021313621,
            0.033491996,
            0.005074616
        ],
        vec![
            0.015660071,
            0.012324,
            0.005722,
            0.003961,
            0.0017,
            0.038031456,
            0.038478742,
            0.024161543,
            0.013869913,
            0.005369458,
            0.040716185,
            0.043848199,
            0.022818671,
            0.016107357,
            0.0035793,
            0.063982141,
            0.059955556,
            0.028635414,
            0.008948757,
            0.003132014,
            0.338255506,
            0.161521868,
            0.028635414,
            0.011633486,
            0.008949708
        ]
    ];
    pub static ref GRID_RESULT: Vec<String> = vec![
        "A1".to_string(),
        "A2".to_string(),
        "A3".to_string(),
        "A4".to_string(),
        "A5".to_string(),
        "A6".to_string(),
        "B1".to_string(),
        "B2".to_string(),
        "B3".to_string(),
        "B4".to_string(),
        "B5".to_string(),
        "B6".to_string(),
        "C1".to_string(),
        "C2".to_string(),
        "C3".to_string(),
        "C4".to_string(),
        "C5".to_string(),
        "C6".to_string(),
        "D1".to_string(),
        "D2".to_string(),
        "D3".to_string(),
        "D4".to_string(),
        "D5".to_string(),
        "D6".to_string(),
        "E1".to_string(),
        "E2".to_string(),
        "E3".to_string(),
        "E4".to_string(),
        "E5".to_string(),
        "E6".to_string(),
        "F1".to_string(),
        "F2".to_string(),
        "F3".to_string(),
        "F4".to_string(),
        "F5".to_string(),
        "F6".to_string(),
        "G1".to_string(),
        "G2".to_string(),
        "G3".to_string(),
        "G4".to_string(),
        "G5".to_string(),
        "G6".to_string(),
        "H1".to_string(),
        "H2".to_string(),
        "H3".to_string(),
        "H4".to_string(),
        "H5".to_string(),
        "H6".to_string(),
        "I1".to_string(),
        "I2".to_string(),
        "I3".to_string(),
        "I4".to_string(),
        "I5".to_string(),
        "I6".to_string(),
        "J1".to_string(),
        "J2".to_string(),
        "J3".to_string(),
        "J4".to_string(),
        "J5".to_string(),
        "J6".to_string(),
        "ZA1".to_string(),
        "ZA2".to_string(),
        "ZA3".to_string(),
        "ZA4".to_string(),
        "ZA5".to_string(),
        "ZA6".to_string(),
        "ZJ1".to_string(),
        "ZJ2".to_string(),
        "ZJ3".to_string(),
        "ZJ4".to_string(),
        "ZJ5".to_string(),
        "ZJ6".to_string()
    ];
    pub static ref KICKOFF_RESULTS: Vec<String> = vec![
        "A1KickAway".to_string(),
        "A1Retain".to_string(),
        "A2KickAway".to_string(),
        "A2Retain".to_string(),
        "A4KickAway".to_string(),
        "A4Retain".to_string(),
        "A5Retain".to_string(),
        "B1KickAway".to_string(),
        "B1Retain".to_string(),
        "B2KickAway".to_string(),
        "B2Retain".to_string(),
        "B3KickAway".to_string(),
        "B3Retain".to_string(),
        "B4KickAway".to_string(),
        "B4Retain".to_string(),
        "B5KickAway".to_string(),
        "B5Retain".to_string(),
        "B6KickAway".to_string(),
        "B6Retain".to_string(),
        "C1KickAway".to_string(),
        "C2KickAway".to_string(),
        "C2Retain".to_string(),
        "C3KickAway".to_string(),
        "C3Retain".to_string(),
        "C4KickAway".to_string(),
        "C4Retain".to_string(),
        "C5KickAway".to_string(),
        "C5Retain".to_string(),
        "C6KickAway".to_string(),
        "C6Retain".to_string(),
        "D1KickAway".to_string(),
        "D2KickAway".to_string(),
        "D3KickAway".to_string(),
        "D4KickAway".to_string(),
        "D4Retain".to_string(),
        "D5KickAway".to_string(),
        "D6KickAway".to_string(),
        "E1KickAway".to_string(),
        "E1Retain".to_string(),
        "E2KickAway".to_string(),
        "E2Retain".to_string(),
        "E3KickAway".to_string(),
        "E4KickAway".to_string(),
        "E4Retain".to_string(),
        "E5KickAway".to_string(),
        "E6KickAway".to_string(),
        "F1KickAway".to_string(),
        "F1Retain".to_string(),
        "F2KickAway".to_string(),
        "F2Retain".to_string(),
        "F3KickAway".to_string(),
        "F4KickAway".to_string(),
        "F4Retain".to_string(),
        "F5KickAway".to_string(),
        "F6KickAway".to_string(),
        "F6Retain".to_string(),
        "G1KickAway".to_string(),
        "G1Retain".to_string(),
        "G2KickAway".to_string(),
        "G2Retain".to_string(),
        "G3KickAway".to_string(),
        "G3Retain".to_string(),
        "G4KickAway".to_string(),
        "G4Retain".to_string(),
        "G5KickAway".to_string(),
        "G5Retain".to_string(),
        "G6KickAway".to_string(),
        "G6Retain".to_string(),
        "H1KickAway".to_string(),
        "H1Retain".to_string(),
        "H2KickAway".to_string(),
        "H2Retain".to_string(),
        "H3KickAway".to_string(),
        "H4KickAway".to_string(),
        "H4Retain".to_string(),
        "H5KickAway".to_string(),
        "H5Retain".to_string(),
        "H6KickAway".to_string(),
        "H6Retain".to_string(),
        "I1KickAway".to_string(),
        "I1Retain".to_string(),
        "I2KickAway".to_string(),
        "I2Retain".to_string(),
        "I3KickAway".to_string(),
        "I3Retain".to_string(),
        "I4KickAway".to_string(),
        "I4Retain".to_string(),
        "I5KickAway".to_string(),
        "I5Retain".to_string(),
        "I6KickAway".to_string(),
        "I6Retain".to_string(),
        "J1KickAway".to_string(),
        "J2KickAway".to_string(),
        "J3KickAway".to_string(),
        "J4KickAway".to_string(),
        "J5KickAway".to_string(),
        "J6KickAway".to_string(),
        "J6Retain".to_string()
    ];
    pub static ref KICKOFF_RETAIN_RESULTS: Vec<String> = vec![
        "A1Retain".to_string(),
        "A2Retain".to_string(),
        "A4Retain".to_string(),
        "A5Retain".to_string(),
        "B1Retain".to_string(),
        "B2Retain".to_string(),
        "B3Retain".to_string(),
        "B4Retain".to_string(),
        "B5Retain".to_string(),
        "B6Retain".to_string(),
        "C2Retain".to_string(),
        "C3Retain".to_string(),
        "C4Retain".to_string(),
        "C6Retain".to_string(),
        "D4Retain".to_string(),
        "E1Retain".to_string(),
        "E2Retain".to_string(),
        "E4Retain".to_string(),
        "F1Retain".to_string(),
        "F2Retain".to_string(),
        "F4Retain".to_string(),
        "F6Retain".to_string(),
        "G1Retain".to_string(),
        "G2Retain".to_string(),
        "G3Retain".to_string(),
        "G4Retain".to_string(),
        "G5Retain".to_string(),
        "G6Retain".to_string(),
        "H1Retain".to_string(),
        "H2Retain".to_string(),
        "H4Retain".to_string(),
        "H5Retain".to_string(),
        "H6Retain".to_string(),
        "I1Retain".to_string(),
        "I2Retain".to_string(),
        "I3Retain".to_string(),
        "I4Retain".to_string(),
        "I6Retain".to_string(),
        "J6Retain".to_string()
    ];
    pub static ref KICKOFF_TO_GRID_RESULT: HashMap<String, String> = {
        let mut map = HashMap::new();
        map.insert("A1Retain".to_string(), "A1".to_string());
        map.insert("A1KickAway".to_string(), "A1".to_string());
        map.insert("A2Retain".to_string(), "A2".to_string());
        map.insert("A2KickAway".to_string(), "A2".to_string());
        map.insert("A4Retain".to_string(), "A4".to_string());
        map.insert("A4KickAway".to_string(), "A4".to_string());
        map.insert("A5Retain".to_string(), "A5".to_string());
        map.insert("B1Retain".to_string(), "B1".to_string());
        map.insert("B1KickAway".to_string(), "B1".to_string());
        map.insert("B2Retain".to_string(), "B2".to_string());
        map.insert("B2KickAway".to_string(), "B2".to_string());
        map.insert("B3Retain".to_string(), "B3".to_string());
        map.insert("B3KickAway".to_string(), "B3".to_string());
        map.insert("B4Retain".to_string(), "B4".to_string());
        map.insert("B4KickAway".to_string(), "B4".to_string());
        map.insert("B5Retain".to_string(), "B5".to_string());
        map.insert("B5KickAway".to_string(), "B5".to_string());
        map.insert("B6Retain".to_string(), "B6".to_string());
        map.insert("B6KickAway".to_string(), "B6".to_string());
        map.insert("C1KickAway".to_string(), "C1".to_string());
        map.insert("C2Retain".to_string(), "C2".to_string());
        map.insert("C2KickAway".to_string(), "C2".to_string());
        map.insert("C3Retain".to_string(), "C3".to_string());
        map.insert("C3KickAway".to_string(), "C3".to_string());
        map.insert("C4Retain".to_string(), "C4".to_string());
        map.insert("C4KickAway".to_string(), "C4".to_string());
        map.insert("C5KickAway".to_string(), "C5".to_string());
        map.insert("C6Retain".to_string(), "C6".to_string());
        map.insert("C6KickAway".to_string(), "C6".to_string());
        map.insert("D1KickAway".to_string(), "D1".to_string());
        map.insert("D2KickAway".to_string(), "D2".to_string());
        map.insert("D3KickAway".to_string(), "D3".to_string());
        map.insert("D4Retain".to_string(), "D4".to_string());
        map.insert("D4KickAway".to_string(), "D4".to_string());
        map.insert("D5KickAway".to_string(), "D5".to_string());
        map.insert("D6KickAway".to_string(), "D6".to_string());
        map.insert("E1Retain".to_string(), "E1".to_string());
        map.insert("E1KickAway".to_string(), "E1".to_string());
        map.insert("E2Retain".to_string(), "E2".to_string());
        map.insert("E2KickAway".to_string(), "E2".to_string());
        map.insert("E3KickAway".to_string(), "E3".to_string());
        map.insert("E4Retain".to_string(), "E4".to_string());
        map.insert("E4KickAway".to_string(), "E4".to_string());
        map.insert("E5KickAway".to_string(), "E5".to_string());
        map.insert("E6KickAway".to_string(), "E6".to_string());
        map.insert("F1Retain".to_string(), "F1".to_string());
        map.insert("F1KickAway".to_string(), "F1".to_string());
        map.insert("F2Retain".to_string(), "F2".to_string());
        map.insert("F2KickAway".to_string(), "F2".to_string());
        map.insert("F3KickAway".to_string(), "F3".to_string());
        map.insert("F4Retain".to_string(), "F4".to_string());
        map.insert("F4KickAway".to_string(), "F4".to_string());
        map.insert("F5KickAway".to_string(), "F5".to_string());
        map.insert("F6Retain".to_string(), "F6".to_string());
        map.insert("F6KickAway".to_string(), "F6".to_string());
        map.insert("G1Retain".to_string(), "G1".to_string());
        map.insert("G1KickAway".to_string(), "G1".to_string());
        map.insert("G2Retain".to_string(), "G2".to_string());
        map.insert("G2KickAway".to_string(), "G2".to_string());
        map.insert("G3Retain".to_string(), "G3".to_string());
        map.insert("G3KickAway".to_string(), "G3".to_string());
        map.insert("G4Retain".to_string(), "G4".to_string());
        map.insert("G4KickAway".to_string(), "G4".to_string());
        map.insert("G5Retain".to_string(), "G5".to_string());
        map.insert("G5KickAway".to_string(), "G5".to_string());
        map.insert("G6Retain".to_string(), "G6".to_string());
        map.insert("G6KickAway".to_string(), "G6".to_string());
        map.insert("H1Retain".to_string(), "H1".to_string());
        map.insert("H1KickAway".to_string(), "H1".to_string());
        map.insert("H2Retain".to_string(), "H2".to_string());
        map.insert("H2KickAway".to_string(), "H2".to_string());
        map.insert("H3KickAway".to_string(), "H3".to_string());
        map.insert("H4Retain".to_string(), "H4".to_string());
        map.insert("H4KickAway".to_string(), "H4".to_string());
        map.insert("H5Retain".to_string(), "H5".to_string());
        map.insert("H5KickAway".to_string(), "H5".to_string());
        map.insert("H6Retain".to_string(), "H6".to_string());
        map.insert("H6KickAway".to_string(), "H6".to_string());
        map.insert("I1Retain".to_string(), "I1".to_string());
        map.insert("I1KickAway".to_string(), "I1".to_string());
        map.insert("I2Retain".to_string(), "I2".to_string());
        map.insert("I2KickAway".to_string(), "I2".to_string());
        map.insert("I3Retain".to_string(), "I3".to_string());
        map.insert("I3KickAway".to_string(), "I3".to_string());
        map.insert("I4Retain".to_string(), "I4".to_string());
        map.insert("I4KickAway".to_string(), "I4".to_string());
        map.insert("I5KickAway".to_string(), "I5".to_string());
        map.insert("I6Retain".to_string(), "I6".to_string());
        map.insert("I6KickAway".to_string(), "I6".to_string());
        map.insert("J1KickAway".to_string(), "J1".to_string());
        map.insert("J2KickAway".to_string(), "J2".to_string());
        map.insert("J3KickAway".to_string(), "J3".to_string());
        map.insert("J4KickAway".to_string(), "J4".to_string());
        map.insert("J5KickAway".to_string(), "J5".to_string());
        map.insert("J6Retain".to_string(), "J6".to_string());
        map.insert("J6KickAway".to_string(), "J6".to_string());
        map
    };
    pub static ref NEXT_PLAY_TYPES: Vec<String> = vec![
        "Pass".to_string(),
        "Run".to_string(),
        "RunTackle".to_string(),
        "RunTry".to_string(),
        "KickRetain".to_string(),
        "KickRetainTackle".to_string(),
        "KickRetainTry".to_string(),
        "KickTurnover".to_string(),
        "ErrorAttack".to_string(),
        "ErrorDefence".to_string(),
        "ConcededPenalty".to_string(),
        "WonPenalty".to_string(),
        "LineDropout".to_string(),
        "FieldGoalAttempt".to_string()
    ];
    pub static ref PLAYER_MODEL_POSITIONS: Vec<String> = vec![
        "FullBack".to_string(),
        "WingerOne".to_string(),
        "CentreOne".to_string(),
        "CentreTwo".to_string(),
        "WingerTwo".to_string(),
        "FiveEighth".to_string(),
        "HalfBack".to_string(),
        "PropOne".to_string(),
        "Hooker".to_string(),
        "PropTwo".to_string(),
        "SecondRowOne".to_string(),
        "SecondRowTwo".to_string(),
        "Lock".to_string()
    ];
    pub static ref PLAYER_POSITION_SEQUENCE: Vec<String> = vec![
        "FullBack".to_string(),
        "WingerOne".to_string(),
        "CentreOne".to_string(),
        "CentreTwo".to_string(),
        "WingerTwo".to_string(),
        "FiveEighth".to_string(),
        "HalfBack".to_string(),
        "PropOne".to_string(),
        "Hooker".to_string(),
        "PropTwo".to_string(),
        "SecondRowOne".to_string(),
        "SecondRowTwo".to_string(),
        "Lock".to_string()
    ];
    pub static ref X_VALUES: Vec<(i32, i32)> = vec![
        (0, 100),
        (100, 200),
        (200, 300),
        (300, 400),
        (400, 500),
        (500, 600),
        (600, 700),
        (700, 800),
        (800, 900),
        (900, 1000),
        (-100, 1),
        (1000, 1100)
    ];
    pub static ref X_VALUES_AWAY: Vec<(i32, i32)> = vec![
        (900, 1000),
        (800, 900),
        (700, 800),
        (600, 700),
        (500, 600),
        (400, 500),
        (300, 400),
        (200, 300),
        (100, 200),
        (0, 100),
        (1000, 1100),
        (-100, 1)
    ];
    pub static ref Y_VALUES: Vec<(i32, i32)> = vec![
        (0, 118),
        (118, 234),
        (234, 351),
        (351, 468),
        (468, 584),
        (584, 700)
    ];
    pub static ref Y_VALUES_AWAY: Vec<(i32, i32)> = vec![
        (584, 700),
        (468, 584),
        (351, 468),
        (234, 351),
        (118, 234),
        (0, 118)
    ];
}
use lazy_static::lazy_static;
use rand::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct ZeroDivisionError {
    message: String,
}
impl std::fmt::Display for ZeroDivisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "division by zero: {}", self.message)
    }
}
impl std::error::Error for ZeroDivisionError {}
impl ZeroDivisionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct IndexError {
    message: String,
}
impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "index out of range: {}", self.message)
    }
}
impl std::error::Error for IndexError {}
impl IndexError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub all_players: Vec<Player>,
    pub away_match_score: i32,
    pub away_period_try_scorers: Vec<Vec<i32>>,
    pub away_player_selected_for_points_market: Player,
    pub away_players_trader_state: Vec<Player>,
    pub away_players: Vec<Player>,
    pub away_remaining_interchanges: i32,
    pub away_sin_bin: Vec<Player>,
    pub away_statistics: TeamStatistics,
    pub away_total_try_scorers: Vec<i32>,
    pub ball_location: FieldPosition,
    pub current_play_type: String,
    pub end_zone_type: String,
    pub has_started: bool,
    pub is_extratime: bool,
    pub game_status: String,
    pub home_match_score: i32,
    pub home_period_try_scorers: Vec<Vec<i32>>,
    pub home_player_selected_for_points_market: Player,
    pub home_players_trader_state: Vec<Player>,
    pub home_players: Vec<Player>,
    pub home_remaining_interchanges: i32,
    pub home_sin_bin: Vec<Player>,
    pub home_statistics: TeamStatistics,
    pub home_total_try_scorers: Vec<i32>,
    pub incidents: Vec<Incident>,
    pub include_players: bool,
    pub is_in_end_zone: bool,
    pub is_over: bool,
    pub last_play_type: String,
    pub period_try_scorers: Vec<Vec<i32>>,
    pub period: Period,
    pub player_of_the_match: i32,
    pub previous_ball_location: FieldPosition,
    pub previous_play_type: String,
    pub set: i32,
    pub simulation_invariants: SimulationInvariants,
    pub tackles: i32,
    pub team_in_possession: String,
    pub time_elapsed: i32,
    pub total_match_score: i32,
    pub total_period: String,
    pub total_try_scorers: Vec<i32>,
}
impl State {
    pub fn new(
        all_players: Vec<Player>,
        away_match_score: i32,
        away_period_try_scorers: Vec<Vec<i32>>,
        away_player_selected_for_points_market: Player,
        away_players_trader_state: Vec<Player>,
        away_players: Vec<Player>,
        away_remaining_interchanges: i32,
        away_sin_bin: Vec<Player>,
        away_statistics: TeamStatistics,
        away_total_try_scorers: Vec<i32>,
        ball_location: FieldPosition,
        current_play_type: String,
        end_zone_type: String,
        has_started: bool,
        is_extratime: bool,
        game_status: String,
        home_match_score: i32,
        home_period_try_scorers: Vec<Vec<i32>>,
        home_player_selected_for_points_market: Player,
        home_players_trader_state: Vec<Player>,
        home_players: Vec<Player>,
        home_remaining_interchanges: i32,
        home_sin_bin: Vec<Player>,
        home_statistics: TeamStatistics,
        home_total_try_scorers: Vec<i32>,
        incidents: Vec<Incident>,
        include_players: bool,
        is_in_end_zone: bool,
        is_over: bool,
        last_play_type: String,
        period_try_scorers: Vec<Vec<i32>>,
        period: Period,
        player_of_the_match: i32,
        previous_ball_location: FieldPosition,
        previous_play_type: String,
        set: i32,
        simulation_invariants: SimulationInvariants,
        tackles: i32,
        team_in_possession: String,
        time_elapsed: i32,
        total_match_score: i32,
        total_period: String,
        total_try_scorers: Vec<i32>,
    ) -> Self {
        Self {
            all_players,
            away_match_score,
            away_period_try_scorers,
            away_player_selected_for_points_market,
            away_players_trader_state,
            away_players,
            away_remaining_interchanges,
            away_sin_bin,
            away_statistics,
            away_total_try_scorers,
            ball_location,
            current_play_type,
            end_zone_type,
            has_started,
            is_extratime,
            game_status,
            home_match_score,
            home_period_try_scorers,
            home_player_selected_for_points_market,
            home_players_trader_state,
            home_players,
            home_remaining_interchanges,
            home_sin_bin,
            home_statistics,
            home_total_try_scorers,
            incidents,
            include_players,
            is_in_end_zone,
            is_over,
            last_play_type,
            period_try_scorers,
            period,
            player_of_the_match,
            previous_ball_location,
            previous_play_type,
            set,
            simulation_invariants,
            tackles,
            team_in_possession,
            time_elapsed,
            total_match_score,
            total_period,
            total_try_scorers,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "all_players" => Some(Box::new(self.all_players.clone()) as Box<dyn std::any::Any>),
            "away_match_score" => {
                Some(Box::new(self.away_match_score.clone()) as Box<dyn std::any::Any>)
            }
            "away_period_try_scorers" => {
                Some(Box::new(self.away_period_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            "away_player_selected_for_points_market" => Some(Box::new(
                self.away_player_selected_for_points_market.clone(),
            )
                as Box<dyn std::any::Any>),
            "away_players_trader_state" => {
                Some(Box::new(self.away_players_trader_state.clone()) as Box<dyn std::any::Any>)
            }
            "away_players" => Some(Box::new(self.away_players.clone()) as Box<dyn std::any::Any>),
            "away_remaining_interchanges" => {
                Some(Box::new(self.away_remaining_interchanges.clone()) as Box<dyn std::any::Any>)
            }
            "away_sin_bin" => Some(Box::new(self.away_sin_bin.clone()) as Box<dyn std::any::Any>),
            "away_statistics" => {
                Some(Box::new(self.away_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "away_total_try_scorers" => {
                Some(Box::new(self.away_total_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            "ball_location" => Some(Box::new(self.ball_location.clone()) as Box<dyn std::any::Any>),
            "current_play_type" => {
                Some(Box::new(self.current_play_type.clone()) as Box<dyn std::any::Any>)
            }
            "end_zone_type" => Some(Box::new(self.end_zone_type.clone()) as Box<dyn std::any::Any>),
            "has_started" => Some(Box::new(self.has_started.clone()) as Box<dyn std::any::Any>),
            "is_extratime" => Some(Box::new(self.is_extratime.clone()) as Box<dyn std::any::Any>),
            "game_status" => Some(Box::new(self.game_status.clone()) as Box<dyn std::any::Any>),
            "home_match_score" => {
                Some(Box::new(self.home_match_score.clone()) as Box<dyn std::any::Any>)
            }
            "home_period_try_scorers" => {
                Some(Box::new(self.home_period_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            "home_player_selected_for_points_market" => Some(Box::new(
                self.home_player_selected_for_points_market.clone(),
            )
                as Box<dyn std::any::Any>),
            "home_players_trader_state" => {
                Some(Box::new(self.home_players_trader_state.clone()) as Box<dyn std::any::Any>)
            }
            "home_players" => Some(Box::new(self.home_players.clone()) as Box<dyn std::any::Any>),
            "home_remaining_interchanges" => {
                Some(Box::new(self.home_remaining_interchanges.clone()) as Box<dyn std::any::Any>)
            }
            "home_sin_bin" => Some(Box::new(self.home_sin_bin.clone()) as Box<dyn std::any::Any>),
            "home_statistics" => {
                Some(Box::new(self.home_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "home_total_try_scorers" => {
                Some(Box::new(self.home_total_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            "incidents" => Some(Box::new(self.incidents.clone()) as Box<dyn std::any::Any>),
            "include_players" => {
                Some(Box::new(self.include_players.clone()) as Box<dyn std::any::Any>)
            }
            "is_in_end_zone" => {
                Some(Box::new(self.is_in_end_zone.clone()) as Box<dyn std::any::Any>)
            }
            "is_over" => Some(Box::new(self.is_over.clone()) as Box<dyn std::any::Any>),
            "last_play_type" => {
                Some(Box::new(self.last_play_type.clone()) as Box<dyn std::any::Any>)
            }
            "period_try_scorers" => {
                Some(Box::new(self.period_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            "period" => Some(Box::new(self.period.clone()) as Box<dyn std::any::Any>),
            "player_of_the_match" => {
                Some(Box::new(self.player_of_the_match.clone()) as Box<dyn std::any::Any>)
            }
            "previous_ball_location" => {
                Some(Box::new(self.previous_ball_location.clone()) as Box<dyn std::any::Any>)
            }
            "previous_play_type" => {
                Some(Box::new(self.previous_play_type.clone()) as Box<dyn std::any::Any>)
            }
            "set" => Some(Box::new(self.set.clone()) as Box<dyn std::any::Any>),
            "simulation_invariants" => {
                Some(Box::new(self.simulation_invariants.clone()) as Box<dyn std::any::Any>)
            }
            "tackles" => Some(Box::new(self.tackles.clone()) as Box<dyn std::any::Any>),
            "team_in_possession" => {
                Some(Box::new(self.team_in_possession.clone()) as Box<dyn std::any::Any>)
            }
            "time_elapsed" => Some(Box::new(self.time_elapsed.clone()) as Box<dyn std::any::Any>),
            "total_match_score" => {
                Some(Box::new(self.total_match_score.clone()) as Box<dyn std::any::Any>)
            }
            "total_period" => Some(Box::new(self.total_period.clone()) as Box<dyn std::any::Any>),
            "total_try_scorers" => {
                Some(Box::new(self.total_try_scorers.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "all_players" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.all_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_match_score" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.away_match_score = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_period_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<Vec<i32>>>() {
                    self.away_period_try_scorers = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_player_selected_for_points_market" => {
                if let Some(v) = value.downcast_ref::<Player>() {
                    self.away_player_selected_for_points_market = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_players_trader_state" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.away_players_trader_state = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_players" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.away_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_remaining_interchanges" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.away_remaining_interchanges = v.clone();
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
            "away_statistics" => {
                if let Some(v) = value.downcast_ref::<TeamStatistics>() {
                    self.away_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_total_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.away_total_try_scorers = v.clone();
                    true
                } else {
                    false
                }
            }
            "ball_location" => {
                if let Some(v) = value.downcast_ref::<FieldPosition>() {
                    self.ball_location = v.clone();
                    true
                } else {
                    false
                }
            }
            "current_play_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.current_play_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "end_zone_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.end_zone_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "has_started" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.has_started = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_extratime" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_extratime = v.clone();
                    true
                } else {
                    false
                }
            }
            "game_status" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.game_status = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_match_score" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.home_match_score = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_period_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<Vec<i32>>>() {
                    self.home_period_try_scorers = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_player_selected_for_points_market" => {
                if let Some(v) = value.downcast_ref::<Player>() {
                    self.home_player_selected_for_points_market = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_players_trader_state" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.home_players_trader_state = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_players" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.home_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_remaining_interchanges" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.home_remaining_interchanges = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_sin_bin" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.home_sin_bin = v.clone();
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
            "home_total_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.home_total_try_scorers = v.clone();
                    true
                } else {
                    false
                }
            }
            "incidents" => {
                if let Some(v) = value.downcast_ref::<Vec<Incident>>() {
                    self.incidents = v.clone();
                    true
                } else {
                    false
                }
            }
            "include_players" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.include_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_in_end_zone" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_in_end_zone = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_over" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_over = v.clone();
                    true
                } else {
                    false
                }
            }
            "last_play_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.last_play_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "period_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<Vec<i32>>>() {
                    self.period_try_scorers = v.clone();
                    true
                } else {
                    false
                }
            }
            "period" => {
                if let Some(v) = value.downcast_ref::<Period>() {
                    self.period = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_of_the_match" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_of_the_match = v.clone();
                    true
                } else {
                    false
                }
            }
            "previous_ball_location" => {
                if let Some(v) = value.downcast_ref::<FieldPosition>() {
                    self.previous_ball_location = v.clone();
                    true
                } else {
                    false
                }
            }
            "previous_play_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.previous_play_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "set" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.set = v.clone();
                    true
                } else {
                    false
                }
            }
            "simulation_invariants" => {
                if let Some(v) = value.downcast_ref::<SimulationInvariants>() {
                    self.simulation_invariants = v.clone();
                    true
                } else {
                    false
                }
            }
            "tackles" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.tackles = v.clone();
                    true
                } else {
                    false
                }
            }
            "team_in_possession" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.team_in_possession = v.clone();
                    true
                } else {
                    false
                }
            }
            "time_elapsed" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.time_elapsed = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_match_score" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.total_match_score = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_period" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.total_period = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_try_scorers" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.total_try_scorers = v.clone();
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
pub struct FieldPosition {
    pub x: i32,
    pub y: i32,
}
impl FieldPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "x" => Some(Box::new(self.x.clone()) as Box<dyn std::any::Any>),
            "y" => Some(Box::new(self.y.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "x" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.x = v.clone();
                    true
                } else {
                    false
                }
            }
            "y" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.y = v.clone();
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
pub struct Incident {
    pub has_points_confirmed: bool,
    pub has_points_scored: bool,
    pub period: Period,
    pub points_confirmed_player_index: i32,
    pub points_confirmed_score_id: String,
    pub points_confirmed_score_type: String,
    pub points_scored_points: i32,
    pub points_scored_score_id: String,
    pub points_scored_score_type: String,
    pub points_scored_team: String,
    pub time_elapsed: i32,
}
impl Incident {
    pub fn new(
        has_points_confirmed: bool,
        has_points_scored: bool,
        period: Period,
        points_confirmed_player_index: i32,
        points_confirmed_score_id: String,
        points_confirmed_score_type: String,
        points_scored_points: i32,
        points_scored_score_id: String,
        points_scored_score_type: String,
        points_scored_team: String,
        time_elapsed: i32,
    ) -> Self {
        Self {
            has_points_confirmed,
            has_points_scored,
            period,
            points_confirmed_player_index,
            points_confirmed_score_id,
            points_confirmed_score_type,
            points_scored_points,
            points_scored_score_id,
            points_scored_score_type,
            points_scored_team,
            time_elapsed,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "has_points_confirmed" => {
                Some(Box::new(self.has_points_confirmed.clone()) as Box<dyn std::any::Any>)
            }
            "has_points_scored" => {
                Some(Box::new(self.has_points_scored.clone()) as Box<dyn std::any::Any>)
            }
            "period" => Some(Box::new(self.period.clone()) as Box<dyn std::any::Any>),
            "points_confirmed_player_index" => {
                Some(Box::new(self.points_confirmed_player_index.clone()) as Box<dyn std::any::Any>)
            }
            "points_confirmed_score_id" => {
                Some(Box::new(self.points_confirmed_score_id.clone()) as Box<dyn std::any::Any>)
            }
            "points_confirmed_score_type" => {
                Some(Box::new(self.points_confirmed_score_type.clone()) as Box<dyn std::any::Any>)
            }
            "points_scored_points" => {
                Some(Box::new(self.points_scored_points.clone()) as Box<dyn std::any::Any>)
            }
            "points_scored_score_id" => {
                Some(Box::new(self.points_scored_score_id.clone()) as Box<dyn std::any::Any>)
            }
            "points_scored_score_type" => {
                Some(Box::new(self.points_scored_score_type.clone()) as Box<dyn std::any::Any>)
            }
            "points_scored_team" => {
                Some(Box::new(self.points_scored_team.clone()) as Box<dyn std::any::Any>)
            }
            "time_elapsed" => Some(Box::new(self.time_elapsed.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "has_points_confirmed" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.has_points_confirmed = v.clone();
                    true
                } else {
                    false
                }
            }
            "has_points_scored" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.has_points_scored = v.clone();
                    true
                } else {
                    false
                }
            }
            "period" => {
                if let Some(v) = value.downcast_ref::<Period>() {
                    self.period = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_confirmed_player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.points_confirmed_player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_confirmed_score_id" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.points_confirmed_score_id = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_confirmed_score_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.points_confirmed_score_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_scored_points" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.points_scored_points = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_scored_score_id" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.points_scored_score_id = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_scored_score_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.points_scored_score_type = v.clone();
                    true
                } else {
                    false
                }
            }
            "points_scored_team" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.points_scored_team = v.clone();
                    true
                } else {
                    false
                }
            }
            "time_elapsed" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.time_elapsed = v.clone();
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
pub struct Period {
    pub number: i32,
    pub name: String,
}
impl Period {
    pub fn new(number: i32, name: String) -> Self {
        Self { number, name }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "number" => Some(Box::new(self.number.clone()) as Box<dyn std::any::Any>),
            "name" => Some(Box::new(self.name.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "number" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.number = v.clone();
                    true
                } else {
                    false
                }
            }
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
pub struct Player {
    pub delta_strength: String,
    pub expected_tries_per_min: f64,
    pub is_home_team: bool,
    pub is_injured: bool,
    pub is_interchange: bool,
    pub is_starter: bool,
    pub is_suspended: bool,
    pub is_voided: bool,
    pub jersey_number: i32,
    pub on_field: bool,
    pub period_statistics: Vec<PlayerStatistics>,
    pub player_index: i32,
    pub player_of_the_match_percentage: f64,
    pub position: PlayerPosition,
    pub return_from_sin_bin_time: i32,
    pub selected_for_points_market: bool,
    pub sin_bin_sent_off: i32,
    pub sin_bin_status: String,
    pub total_statistics: PlayerStatistics,
    pub total_strength: String,
    pub tries_per_minute_in_use: bool,
    pub tries_per_minute: f64,
    pub tries_percentage_in_use: bool,
    pub tries_percentage: f64,
    pub tries_strength: String,
}
impl Player {
    pub fn new(
        delta_strength: String,
        expected_tries_per_min: f64,
        is_home_team: bool,
        is_injured: bool,
        is_interchange: bool,
        is_starter: bool,
        is_suspended: bool,
        is_voided: bool,
        jersey_number: i32,
        on_field: bool,
        period_statistics: Vec<PlayerStatistics>,
        player_index: i32,
        player_of_the_match_percentage: f64,
        position: PlayerPosition,
        return_from_sin_bin_time: i32,
        selected_for_points_market: bool,
        sin_bin_sent_off: i32,
        sin_bin_status: String,
        total_statistics: PlayerStatistics,
        total_strength: String,
        tries_per_minute_in_use: bool,
        tries_per_minute: f64,
        tries_percentage_in_use: bool,
        tries_percentage: f64,
        tries_strength: String,
    ) -> Self {
        Self {
            delta_strength,
            expected_tries_per_min,
            is_home_team,
            is_injured,
            is_interchange,
            is_starter,
            is_suspended,
            is_voided,
            jersey_number,
            on_field,
            period_statistics,
            player_index,
            player_of_the_match_percentage,
            position,
            return_from_sin_bin_time,
            selected_for_points_market,
            sin_bin_sent_off,
            sin_bin_status,
            total_statistics,
            total_strength,
            tries_per_minute_in_use,
            tries_per_minute,
            tries_percentage_in_use,
            tries_percentage,
            tries_strength,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "delta_strength" => {
                Some(Box::new(self.delta_strength.clone()) as Box<dyn std::any::Any>)
            }
            "expected_tries_per_min" => {
                Some(Box::new(self.expected_tries_per_min.clone()) as Box<dyn std::any::Any>)
            }
            "is_home_team" => Some(Box::new(self.is_home_team.clone()) as Box<dyn std::any::Any>),
            "is_injured" => Some(Box::new(self.is_injured.clone()) as Box<dyn std::any::Any>),
            "is_interchange" => {
                Some(Box::new(self.is_interchange.clone()) as Box<dyn std::any::Any>)
            }
            "is_starter" => Some(Box::new(self.is_starter.clone()) as Box<dyn std::any::Any>),
            "is_suspended" => Some(Box::new(self.is_suspended.clone()) as Box<dyn std::any::Any>),
            "is_voided" => Some(Box::new(self.is_voided.clone()) as Box<dyn std::any::Any>),
            "jersey_number" => Some(Box::new(self.jersey_number.clone()) as Box<dyn std::any::Any>),
            "on_field" => Some(Box::new(self.on_field.clone()) as Box<dyn std::any::Any>),
            "period_statistics" => {
                Some(Box::new(self.period_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "player_index" => Some(Box::new(self.player_index.clone()) as Box<dyn std::any::Any>),
            "player_of_the_match_percentage" => {
                Some(Box::new(self.player_of_the_match_percentage.clone()) as Box<dyn std::any::Any>)
            }
            "position" => Some(Box::new(self.position.clone()) as Box<dyn std::any::Any>),
            "return_from_sin_bin_time" => {
                Some(Box::new(self.return_from_sin_bin_time.clone()) as Box<dyn std::any::Any>)
            }
            "selected_for_points_market" => {
                Some(Box::new(self.selected_for_points_market.clone()) as Box<dyn std::any::Any>)
            }
            "sin_bin_sent_off" => {
                Some(Box::new(self.sin_bin_sent_off.clone()) as Box<dyn std::any::Any>)
            }
            "sin_bin_status" => {
                Some(Box::new(self.sin_bin_status.clone()) as Box<dyn std::any::Any>)
            }
            "total_statistics" => {
                Some(Box::new(self.total_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "total_strength" => {
                Some(Box::new(self.total_strength.clone()) as Box<dyn std::any::Any>)
            }
            "tries_per_minute_in_use" => {
                Some(Box::new(self.tries_per_minute_in_use.clone()) as Box<dyn std::any::Any>)
            }
            "tries_per_minute" => {
                Some(Box::new(self.tries_per_minute.clone()) as Box<dyn std::any::Any>)
            }
            "tries_percentage_in_use" => {
                Some(Box::new(self.tries_percentage_in_use.clone()) as Box<dyn std::any::Any>)
            }
            "tries_percentage" => {
                Some(Box::new(self.tries_percentage.clone()) as Box<dyn std::any::Any>)
            }
            "tries_strength" => {
                Some(Box::new(self.tries_strength.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "delta_strength" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.delta_strength = v.clone();
                    true
                } else {
                    false
                }
            }
            "expected_tries_per_min" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.expected_tries_per_min = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_home_team" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_home_team = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_injured" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_injured = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_interchange" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_interchange = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_starter" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_starter = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_suspended" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_suspended = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_voided" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_voided = v.clone();
                    true
                } else {
                    false
                }
            }
            "jersey_number" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.jersey_number = v.clone();
                    true
                } else {
                    false
                }
            }
            "on_field" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.on_field = v.clone();
                    true
                } else {
                    false
                }
            }
            "period_statistics" => {
                if let Some(v) = value.downcast_ref::<Vec<PlayerStatistics>>() {
                    self.period_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_of_the_match_percentage" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.player_of_the_match_percentage = v.clone();
                    true
                } else {
                    false
                }
            }
            "position" => {
                if let Some(v) = value.downcast_ref::<PlayerPosition>() {
                    self.position = v.clone();
                    true
                } else {
                    false
                }
            }
            "return_from_sin_bin_time" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.return_from_sin_bin_time = v.clone();
                    true
                } else {
                    false
                }
            }
            "selected_for_points_market" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.selected_for_points_market = v.clone();
                    true
                } else {
                    false
                }
            }
            "sin_bin_sent_off" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.sin_bin_sent_off = v.clone();
                    true
                } else {
                    false
                }
            }
            "sin_bin_status" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.sin_bin_status = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_statistics" => {
                if let Some(v) = value.downcast_ref::<PlayerStatistics>() {
                    self.total_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_strength" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.total_strength = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_per_minute_in_use" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.tries_per_minute_in_use = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_per_minute" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.tries_per_minute = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_percentage_in_use" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.tries_percentage_in_use = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_percentage" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.tries_percentage = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_strength" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.tries_strength = v.clone();
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
pub struct PlayerPosition {
    pub position_type: String,
}
impl PlayerPosition {
    pub fn new(position_type: String) -> Self {
        Self { position_type }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "position_type" => Some(Box::new(self.position_type.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "position_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.position_type = v.clone();
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
pub struct PlayerStatistics {
    pub jersey_number: i32,
    pub metres_gained: i32,
    pub period_statistics: Vec<Statistics>,
    pub player_index: i32,
    pub scores: Scores,
    pub tackles: i32,
    pub time_on_field: f64,
    pub total_statistics: Statistics,
}
impl PlayerStatistics {
    pub fn new(
        jersey_number: i32,
        metres_gained: i32,
        period_statistics: Vec<Statistics>,
        player_index: i32,
        scores: Scores,
        tackles: i32,
        time_on_field: f64,
        total_statistics: Statistics,
    ) -> Self {
        Self {
            jersey_number,
            metres_gained,
            period_statistics,
            player_index,
            scores,
            tackles,
            time_on_field,
            total_statistics,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "jersey_number" => Some(Box::new(self.jersey_number.clone()) as Box<dyn std::any::Any>),
            "metres_gained" => Some(Box::new(self.metres_gained.clone()) as Box<dyn std::any::Any>),
            "period_statistics" => {
                Some(Box::new(self.period_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "player_index" => Some(Box::new(self.player_index.clone()) as Box<dyn std::any::Any>),
            "scores" => Some(Box::new(self.scores.clone()) as Box<dyn std::any::Any>),
            "tackles" => Some(Box::new(self.tackles.clone()) as Box<dyn std::any::Any>),
            "time_on_field" => Some(Box::new(self.time_on_field.clone()) as Box<dyn std::any::Any>),
            "total_statistics" => {
                Some(Box::new(self.total_statistics.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "jersey_number" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.jersey_number = v.clone();
                    true
                } else {
                    false
                }
            }
            "metres_gained" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.metres_gained = v.clone();
                    true
                } else {
                    false
                }
            }
            "period_statistics" => {
                if let Some(v) = value.downcast_ref::<Vec<Statistics>>() {
                    self.period_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "scores" => {
                if let Some(v) = value.downcast_ref::<Scores>() {
                    self.scores = v.clone();
                    true
                } else {
                    false
                }
            }
            "tackles" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.tackles = v.clone();
                    true
                } else {
                    false
                }
            }
            "time_on_field" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.time_on_field = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_statistics" => {
                if let Some(v) = value.downcast_ref::<Statistics>() {
                    self.total_statistics = v.clone();
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
pub struct Scores {
    pub conversions: i32,
    pub field_goals: i32,
    pub penalties: i32,
    pub total: i32,
    pub tries: i32,
}
impl Scores {
    pub fn new(conversions: i32, field_goals: i32, penalties: i32, total: i32, tries: i32) -> Self {
        Self {
            conversions,
            field_goals,
            penalties,
            total,
            tries,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "conversions" => Some(Box::new(self.conversions.clone()) as Box<dyn std::any::Any>),
            "field_goals" => Some(Box::new(self.field_goals.clone()) as Box<dyn std::any::Any>),
            "penalties" => Some(Box::new(self.penalties.clone()) as Box<dyn std::any::Any>),
            "total" => Some(Box::new(self.total.clone()) as Box<dyn std::any::Any>),
            "tries" => Some(Box::new(self.tries.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "conversions" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.conversions = v.clone();
                    true
                } else {
                    false
                }
            }
            "field_goals" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.field_goals = v.clone();
                    true
                } else {
                    false
                }
            }
            "penalties" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.penalties = v.clone();
                    true
                } else {
                    false
                }
            }
            "total" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.total = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.tries = v.clone();
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
pub struct Statistics {
    pub conversions_missed: i32,
    pub penalties_awarded: i32,
    pub scores: Scores,
    pub tackles: i32,
    pub turnovers: i32,
}
impl Statistics {
    pub fn new(
        conversions_missed: i32,
        penalties_awarded: i32,
        scores: Scores,
        tackles: i32,
        turnovers: i32,
    ) -> Self {
        Self {
            conversions_missed,
            penalties_awarded,
            scores,
            tackles,
            turnovers,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "conversions_missed" => {
                Some(Box::new(self.conversions_missed.clone()) as Box<dyn std::any::Any>)
            }
            "penalties_awarded" => {
                Some(Box::new(self.penalties_awarded.clone()) as Box<dyn std::any::Any>)
            }
            "scores" => Some(Box::new(self.scores.clone()) as Box<dyn std::any::Any>),
            "tackles" => Some(Box::new(self.tackles.clone()) as Box<dyn std::any::Any>),
            "turnovers" => Some(Box::new(self.turnovers.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "conversions_missed" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.conversions_missed = v.clone();
                    true
                } else {
                    false
                }
            }
            "penalties_awarded" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.penalties_awarded = v.clone();
                    true
                } else {
                    false
                }
            }
            "scores" => {
                if let Some(v) = value.downcast_ref::<Scores>() {
                    self.scores = v.clone();
                    true
                } else {
                    false
                }
            }
            "tackles" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.tackles = v.clone();
                    true
                } else {
                    false
                }
            }
            "turnovers" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.turnovers = v.clone();
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
    pub period_statistics: Vec<Statistics>,
    pub player_statistics: Vec<PlayerStatistics>,
    pub sin_bin_players: Vec<Player>,
    pub total_statistics: Statistics,
}
impl TeamStatistics {
    pub fn new(
        period_statistics: Vec<Statistics>,
        player_statistics: Vec<PlayerStatistics>,
        sin_bin_players: Vec<Player>,
        total_statistics: Statistics,
    ) -> Self {
        Self {
            period_statistics,
            player_statistics,
            sin_bin_players,
            total_statistics,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "period_statistics" => {
                Some(Box::new(self.period_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "player_statistics" => {
                Some(Box::new(self.player_statistics.clone()) as Box<dyn std::any::Any>)
            }
            "sin_bin_players" => {
                Some(Box::new(self.sin_bin_players.clone()) as Box<dyn std::any::Any>)
            }
            "total_statistics" => {
                Some(Box::new(self.total_statistics.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "period_statistics" => {
                if let Some(v) = value.downcast_ref::<Vec<Statistics>>() {
                    self.period_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_statistics" => {
                if let Some(v) = value.downcast_ref::<Vec<PlayerStatistics>>() {
                    self.player_statistics = v.clone();
                    true
                } else {
                    false
                }
            }
            "sin_bin_players" => {
                if let Some(v) = value.downcast_ref::<Vec<Player>>() {
                    self.sin_bin_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_statistics" => {
                if let Some(v) = value.downcast_ref::<Statistics>() {
                    self.total_statistics = v.clone();
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
pub struct PlayerInvariants {
    pub player_solution: PlayerSolution,
    pub player_trader_state: PlayerTraderState,
}
impl PlayerInvariants {
    pub fn new(player_solution: PlayerSolution, player_trader_state: PlayerTraderState) -> Self {
        Self {
            player_solution,
            player_trader_state,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "player_solution" => {
                Some(Box::new(self.player_solution.clone()) as Box<dyn std::any::Any>)
            }
            "player_trader_state" => {
                Some(Box::new(self.player_trader_state.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "player_solution" => {
                if let Some(v) = value.downcast_ref::<PlayerSolution>() {
                    self.player_solution = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_trader_state" => {
                if let Some(v) = value.downcast_ref::<PlayerTraderState>() {
                    self.player_trader_state = v.clone();
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
pub struct PlayerSolution {
    pub solved_expected_tries_per_min: f64,
    pub solved_expected_tries_percentage: f64,
}
impl PlayerSolution {
    pub fn new(solved_expected_tries_per_min: f64, solved_expected_tries_percentage: f64) -> Self {
        Self {
            solved_expected_tries_per_min,
            solved_expected_tries_percentage,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "solved_expected_tries_per_min" => {
                Some(Box::new(self.solved_expected_tries_per_min.clone()) as Box<dyn std::any::Any>)
            }
            "solved_expected_tries_percentage" => {
                Some(Box::new(self.solved_expected_tries_percentage.clone())
                    as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "solved_expected_tries_per_min" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.solved_expected_tries_per_min = v.clone();
                    true
                } else {
                    false
                }
            }
            "solved_expected_tries_percentage" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.solved_expected_tries_percentage = v.clone();
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
pub struct SimulationInvariants {
    pub all_players_invariants: Vec<PlayerInvariants>,
    pub away_handicap: f64,
    pub away_price: f64,
    pub away_team_players_invariants: Vec<PlayerInvariants>,
    pub home_handicap: f64,
    pub home_price: f64,
    pub home_team_player_invariants: Vec<PlayerInvariants>,
    pub include_players: bool,
    pub player_of_the_total_enabled: bool,
    pub total_points: f64,
}
impl SimulationInvariants {
    pub fn new(
        all_players_invariants: Vec<PlayerInvariants>,
        away_handicap: f64,
        away_price: f64,
        away_team_players_invariants: Vec<PlayerInvariants>,
        home_handicap: f64,
        home_price: f64,
        home_team_player_invariants: Vec<PlayerInvariants>,
        include_players: bool,
        player_of_the_total_enabled: bool,
        total_points: f64,
    ) -> Self {
        Self {
            all_players_invariants,
            away_handicap,
            away_price,
            away_team_players_invariants,
            home_handicap,
            home_price,
            home_team_player_invariants,
            include_players,
            player_of_the_total_enabled,
            total_points,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "all_players_invariants" => {
                Some(Box::new(self.all_players_invariants.clone()) as Box<dyn std::any::Any>)
            }
            "away_handicap" => Some(Box::new(self.away_handicap.clone()) as Box<dyn std::any::Any>),
            "away_price" => Some(Box::new(self.away_price.clone()) as Box<dyn std::any::Any>),
            "away_team_players_invariants" => {
                Some(Box::new(self.away_team_players_invariants.clone()) as Box<dyn std::any::Any>)
            }
            "home_handicap" => Some(Box::new(self.home_handicap.clone()) as Box<dyn std::any::Any>),
            "home_price" => Some(Box::new(self.home_price.clone()) as Box<dyn std::any::Any>),
            "home_team_player_invariants" => {
                Some(Box::new(self.home_team_player_invariants.clone()) as Box<dyn std::any::Any>)
            }
            "include_players" => {
                Some(Box::new(self.include_players.clone()) as Box<dyn std::any::Any>)
            }
            "player_of_the_total_enabled" => {
                Some(Box::new(self.player_of_the_total_enabled.clone()) as Box<dyn std::any::Any>)
            }
            "total_points" => Some(Box::new(self.total_points.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "all_players_invariants" => {
                if let Some(v) = value.downcast_ref::<Vec<PlayerInvariants>>() {
                    self.all_players_invariants = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_handicap" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.away_handicap = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_price" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.away_price = v.clone();
                    true
                } else {
                    false
                }
            }
            "away_team_players_invariants" => {
                if let Some(v) = value.downcast_ref::<Vec<PlayerInvariants>>() {
                    self.away_team_players_invariants = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_handicap" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.home_handicap = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_price" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.home_price = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_team_player_invariants" => {
                if let Some(v) = value.downcast_ref::<Vec<PlayerInvariants>>() {
                    self.home_team_player_invariants = v.clone();
                    true
                } else {
                    false
                }
            }
            "include_players" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.include_players = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_of_the_total_enabled" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.player_of_the_total_enabled = v.clone();
                    true
                } else {
                    false
                }
            }
            "total_points" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.total_points = v.clone();
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
pub struct PlayerTraderState {
    pub expected_tries_per_minute: f64,
    pub home_team: bool,
    pub is_injured: bool,
    pub is_interchange: bool,
    pub is_starter: bool,
    pub player_index: i32,
    pub player_of_the_match_percentage: f64,
    pub position: PlayerPosition,
    pub selected_for_points_market: bool,
    pub tries_per_minute_in_use: bool,
    pub tries_percentage_in_use: bool,
    pub tries_percentage: f64,
}
impl PlayerTraderState {
    pub fn new(
        expected_tries_per_minute: f64,
        home_team: bool,
        is_injured: bool,
        is_interchange: bool,
        is_starter: bool,
        player_index: i32,
        player_of_the_match_percentage: f64,
        position: PlayerPosition,
        selected_for_points_market: bool,
        tries_per_minute_in_use: bool,
        tries_percentage_in_use: bool,
        tries_percentage: f64,
    ) -> Self {
        Self {
            expected_tries_per_minute,
            home_team,
            is_injured,
            is_interchange,
            is_starter,
            player_index,
            player_of_the_match_percentage,
            position,
            selected_for_points_market,
            tries_per_minute_in_use,
            tries_percentage_in_use,
            tries_percentage,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "expected_tries_per_minute" => {
                Some(Box::new(self.expected_tries_per_minute.clone()) as Box<dyn std::any::Any>)
            }
            "home_team" => Some(Box::new(self.home_team.clone()) as Box<dyn std::any::Any>),
            "is_injured" => Some(Box::new(self.is_injured.clone()) as Box<dyn std::any::Any>),
            "is_interchange" => {
                Some(Box::new(self.is_interchange.clone()) as Box<dyn std::any::Any>)
            }
            "is_starter" => Some(Box::new(self.is_starter.clone()) as Box<dyn std::any::Any>),
            "player_index" => Some(Box::new(self.player_index.clone()) as Box<dyn std::any::Any>),
            "player_of_the_match_percentage" => {
                Some(Box::new(self.player_of_the_match_percentage.clone()) as Box<dyn std::any::Any>)
            }
            "position" => Some(Box::new(self.position.clone()) as Box<dyn std::any::Any>),
            "selected_for_points_market" => {
                Some(Box::new(self.selected_for_points_market.clone()) as Box<dyn std::any::Any>)
            }
            "tries_per_minute_in_use" => {
                Some(Box::new(self.tries_per_minute_in_use.clone()) as Box<dyn std::any::Any>)
            }
            "tries_percentage_in_use" => {
                Some(Box::new(self.tries_percentage_in_use.clone()) as Box<dyn std::any::Any>)
            }
            "tries_percentage" => {
                Some(Box::new(self.tries_percentage.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "expected_tries_per_minute" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.expected_tries_per_minute = v.clone();
                    true
                } else {
                    false
                }
            }
            "home_team" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.home_team = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_injured" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_injured = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_interchange" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_interchange = v.clone();
                    true
                } else {
                    false
                }
            }
            "is_starter" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.is_starter = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "player_of_the_match_percentage" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.player_of_the_match_percentage = v.clone();
                    true
                } else {
                    false
                }
            }
            "position" => {
                if let Some(v) = value.downcast_ref::<PlayerPosition>() {
                    self.position = v.clone();
                    true
                } else {
                    false
                }
            }
            "selected_for_points_market" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.selected_for_points_market = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_per_minute_in_use" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.tries_per_minute_in_use = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_percentage_in_use" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.tries_percentage_in_use = v.clone();
                    true
                } else {
                    false
                }
            }
            "tries_percentage" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.tries_percentage = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct ClockInferOutputs0 {
    pub variable: Vec<f64>,
}
impl ClockInferOutputs0 {
    pub fn new(variable: Vec<f64>) -> Self {
        Self { variable }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "variable" => Some(Box::new(self.variable.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "variable" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.variable = v.clone();
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
pub struct ClockOutputs {
    pub seconds_to_add: f64,
}
impl ClockOutputs {
    pub fn new(seconds_to_add: f64) -> Self {
        Self { seconds_to_add }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "seconds_to_add" => {
                Some(Box::new(self.seconds_to_add.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "seconds_to_add" => {
                if let Some(v) = value.downcast_ref::<f64>() {
                    self.seconds_to_add = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct FieldGoalAttemptInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl FieldGoalAttemptInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct FieldGoalAttemptOutputs {
    pub attempt: i32,
}
impl FieldGoalAttemptOutputs {
    pub fn new(attempt: i32) -> Self {
        Self { attempt }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "attempt" => Some(Box::new(self.attempt.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "attempt" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.attempt = v.clone();
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
pub struct FieldGoalDecisionOutputs {
    pub attempt: bool,
}
impl FieldGoalDecisionOutputs {
    pub fn new(attempt: bool) -> Self {
        Self { attempt }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "attempt" => Some(Box::new(self.attempt.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "attempt" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.attempt = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct GetConversionModelResultInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl GetConversionModelResultInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct GetConversionModelResultInferOutputs1 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl GetConversionModelResultInferOutputs1 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct GetConversionModelResultOutputs {
    pub scored: bool,
}
impl GetConversionModelResultOutputs {
    pub fn new(scored: bool) -> Self {
        Self { scored }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "scored" => Some(Box::new(self.scored.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "scored" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.scored = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct GetFieldGoalModelResultInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl GetFieldGoalModelResultInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct GetFieldGoalModelResultOutputs {
    pub scored: bool,
}
impl GetFieldGoalModelResultOutputs {
    pub fn new(scored: bool) -> Self {
        Self { scored }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "scored" => Some(Box::new(self.scored.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "scored" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.scored = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct GetKickoffModelResultInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl GetKickoffModelResultInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct GetKickoffModelResultOutputs {
    pub change_possession: bool,
    pub grid_result: String,
}
impl GetKickoffModelResultOutputs {
    pub fn new(change_possession: bool, grid_result: String) -> Self {
        Self {
            change_possession,
            grid_result,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "change_possession" => {
                Some(Box::new(self.change_possession.clone()) as Box<dyn std::any::Any>)
            }
            "grid_result" => Some(Box::new(self.grid_result.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "change_possession" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.change_possession = v.clone();
                    true
                } else {
                    false
                }
            }
            "grid_result" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.grid_result = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct InterchangeInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl InterchangeInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct InterchangeInferOutputs1 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl InterchangeInferOutputs1 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct NextPlayInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl NextPlayInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct NextPlayOutputs {
    pub play_type: String,
}
impl NextPlayOutputs {
    pub fn new(play_type: String) -> Self {
        Self { play_type }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "play_type" => Some(Box::new(self.play_type.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "play_type" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.play_type = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct PenaltyTypeInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl PenaltyTypeInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct PenaltyTypeOutputs {
    pub result: i32,
    pub result_index: i32,
}
impl PenaltyTypeOutputs {
    pub fn new(result: i32, result_index: i32) -> Self {
        Self {
            result,
            result_index,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "result" => Some(Box::new(self.result.clone()) as Box<dyn std::any::Any>),
            "result_index" => Some(Box::new(self.result_index.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "result" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.result = v.clone();
                    true
                } else {
                    false
                }
            }
            "result_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.result_index = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct PlayerInterchangeInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl PlayerInterchangeInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct PlayerInterchangeOutputs {
    pub executed: bool,
    pub off_player_index: i32,
    pub on_player_index: i32,
    pub team: String,
}
impl PlayerInterchangeOutputs {
    pub fn new(executed: bool, off_player_index: i32, on_player_index: i32, team: String) -> Self {
        Self {
            executed,
            off_player_index,
            on_player_index,
            team,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "executed" => Some(Box::new(self.executed.clone()) as Box<dyn std::any::Any>),
            "off_player_index" => {
                Some(Box::new(self.off_player_index.clone()) as Box<dyn std::any::Any>)
            }
            "on_player_index" => {
                Some(Box::new(self.on_player_index.clone()) as Box<dyn std::any::Any>)
            }
            "team" => Some(Box::new(self.team.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "executed" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.executed = v.clone();
                    true
                } else {
                    false
                }
            }
            "off_player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.off_player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "on_player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.on_player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "team" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.team = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct PlayerMetersInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl PlayerMetersInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct PlayerMetersOutputs {
    pub player_index: i32,
}
impl PlayerMetersOutputs {
    pub fn new(player_index: i32) -> Self {
        Self { player_index }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "player_index" => Some(Box::new(self.player_index.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_index = v.clone();
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
pub struct PlayerOfTheMatchOutputs {
    pub player_of_the_match: i32,
}
impl PlayerOfTheMatchOutputs {
    pub fn new(player_of_the_match: i32) -> Self {
        Self {
            player_of_the_match,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "player_of_the_match" => {
                Some(Box::new(self.player_of_the_match.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "player_of_the_match" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_of_the_match = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct PlayerSinBinInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl PlayerSinBinInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct PlayerTriesInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl PlayerTriesInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct PlayerTriesOutputs {
    pub player_index: i32,
    pub team: String,
}
impl PlayerTriesOutputs {
    pub fn new(player_index: i32, team: String) -> Self {
        Self { player_index, team }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "player_index" => Some(Box::new(self.player_index.clone()) as Box<dyn std::any::Any>),
            "team" => Some(Box::new(self.team.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "player_index" => {
                if let Some(v) = value.downcast_ref::<i32>() {
                    self.player_index = v.clone();
                    true
                } else {
                    false
                }
            }
            "team" => {
                if let Some(v) = value.downcast_ref::<String>() {
                    self.team = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct SinBinInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl SinBinInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct SinBinOutputs {
    pub sent_off: bool,
}
impl SinBinOutputs {
    pub fn new(sent_off: bool) -> Self {
        Self { sent_off }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "sent_off" => Some(Box::new(self.sent_off.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "sent_off" => {
                if let Some(v) = value.downcast_ref::<bool>() {
                    self.sent_off = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct XyInferOutputs0 {
    pub label: Vec<i32>,
    pub probabilities: Vec<f64>,
}
impl XyInferOutputs0 {
    pub fn new(label: Vec<i32>, probabilities: Vec<f64>) -> Self {
        Self {
            label,
            probabilities,
        }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "label" => Some(Box::new(self.label.clone()) as Box<dyn std::any::Any>),
            "probabilities" => Some(Box::new(self.probabilities.clone()) as Box<dyn std::any::Any>),
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "label" => {
                if let Some(v) = value.downcast_ref::<Vec<i32>>() {
                    self.label = v.clone();
                    true
                } else {
                    false
                }
            }
            "probabilities" => {
                if let Some(v) = value.downcast_ref::<Vec<f64>>() {
                    self.probabilities = v.clone();
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
pub struct XyOutputs {
    pub field_position: FieldPosition,
}
impl XyOutputs {
    pub fn new(field_position: FieldPosition) -> Self {
        Self { field_position }
    }
    pub fn _get_field(&self, name: &str) -> Option<Box<dyn std::any::Any>> {
        match name {
            "field_position" => {
                Some(Box::new(self.field_position.clone()) as Box<dyn std::any::Any>)
            }
            _ => None,
        }
    }
    pub fn _set_field(&mut self, name: &str, value: &dyn std::any::Any) -> bool {
        match name {
            "field_position" => {
                if let Some(v) = value.downcast_ref::<FieldPosition>() {
                    self.field_position = v.clone();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
pub fn debug(state: &State) {
    let race_to_targets: Vec<i32> = Vec::new();
    let remaining_targets: Vec<i32> = Vec::new();
    let target_index: i32 = 0;
    if state.is_over {}
    return;
}
pub fn anytime_output_event(state: &State) {
    let mut away_extra_time_stats: Option<Statistics> = None;
    let mut away_second_half_stats: Option<Statistics> = None;
    let mut ended_half_1: bool = false;
    let mut ended_match: bool = false;
    let mut home_extra_time_stats: Option<Statistics> = None;
    let mut home_second_half_stats: Option<Statistics> = None;
    let mut team_a_second_half_points: i32 = 0;
    let mut team_b_second_half_points: i32 = 0;
    let _cse_temp_0 = !["UnknownStatus", "NotStarted"].contains(&state.game_status.as_str());
    if _cse_temp_0 {
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_extra_time_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_extra_time_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        ended_match = state.is_over;
        team_b_second_half_points = away_second_half_stats.as_ref().unwrap().scores.total;
        let _cse_temp_1 = state.period.number > 1;
        ended_half_1 = _cse_temp_1;
        team_a_second_half_points = home_second_half_stats.as_ref().unwrap().scores.total;
        record_bool("EndedHalf1".to_string(), ended_half_1);
        record_bool("EndedMatch".to_string(), ended_match);
        record_int("PointsHalf2A".to_string(), team_a_second_half_points);
        record_int("PointsHalf2B".to_string(), team_b_second_half_points);
        record_int(
            "TriesMatchA".to_string(),
            state.home_statistics.total_statistics.scores.tries,
        );
        record_int(
            "TriesMatchB".to_string(),
            state.away_statistics.total_statistics.scores.tries,
        );
        record_int(
            "TriesExtraTimeA".to_string(),
            home_extra_time_stats.as_ref().unwrap().scores.tries,
        );
        record_int(
            "TriesExtraTimeB".to_string(),
            away_extra_time_stats.as_ref().unwrap().scores.tries,
        );
    }
    return;
}
pub fn end_of_game_output_event(state: &State) {
    let mut away_extra_time_points: i32 = 0;
    let mut away_extra_time_stats: Option<Statistics> = None;
    let mut away_first_half_stats: Option<Statistics> = None;
    let mut away_second_half_stats: Option<Statistics> = None;
    let mut away_second_half_with_et: i32 = 0;
    let mut draw_match: bool = false;
    let mut home_extra_time_points: i32 = 0;
    let mut home_extra_time_stats: Option<Statistics> = None;
    let mut home_first_half_stats: Option<Statistics> = None;
    let mut home_second_half_stats: Option<Statistics> = None;
    let mut home_second_half_with_et: i32 = 0;
    let mut team_a_won_match: bool = false;
    let mut team_a_won_second_half_and_et: bool = false;
    let mut team_b_won_second_half_and_et: bool = false;
    let _cse_temp_0 = state.game_status.clone() == "ENDED";
    if _cse_temp_0 {
        away_extra_time_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_extra_time_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        let _cse_temp_1 = away_second_half_stats.as_ref().unwrap().scores.total
            + away_extra_time_stats.as_ref().unwrap().scores.total;
        away_second_half_with_et = _cse_temp_1;
        home_extra_time_points = home_extra_time_stats.as_ref().unwrap().scores.total;
        away_extra_time_points = away_extra_time_stats.as_ref().unwrap().scores.total;
        home_second_half_with_et = _cse_temp_1;
        let _cse_temp_2 = home_second_half_with_et > away_second_half_with_et;
        team_a_won_second_half_and_et = _cse_temp_2;
        let _cse_temp_3 = home_second_half_with_et < away_second_half_with_et;
        team_b_won_second_half_and_et = _cse_temp_3;
        let _cse_temp_4 = state.home_match_score > state.away_match_score;
        team_a_won_match = _cse_temp_4;
        let team_b_won_match = _cse_temp_4;
        let _cse_temp_5 = state.home_match_score == state.away_match_score;
        draw_match = _cse_temp_5;
        record_int("PointsMatchA".to_string(), state.home_match_score);
        record_int("PointsMatchB".to_string(), state.away_match_score);
        record_int("PointsExtraTimeA".to_string(), home_extra_time_points);
        record_int("PointsExtraTimeB".to_string(), away_extra_time_points);
        record_bool(
            "WinHalf2AndExtraTimeA".to_string(),
            team_a_won_second_half_and_et,
        );
        record_bool(
            "WinHalf2AndExtraTimeB".to_string(),
            team_b_won_second_half_and_et,
        );
        record_bool("DrawMatch".to_string(), draw_match);
        record_bool("WinMatchA".to_string(), team_a_won_match);
        record_bool("WinMatchB".to_string(), team_b_won_match);
    }
    return;
}
pub fn first_half_output_event(state: &State) {
    let mut away_first_half_stats: Option<Statistics> = None;
    let mut away_first_half_total: i32 = 0;
    let mut away_first_half_tries: i32 = 0;
    let mut draw_first_half: bool = false;
    let mut home_first_half_stats: Option<Statistics> = None;
    let mut home_first_half_total: i32 = 0;
    let mut home_first_half_tries: i32 = 0;
    let mut team_a_won_first_half: bool = false;
    let mut team_b_won_first_half: bool = false;
    let _cse_temp_0 = [
        "HalfTime",
        "Period2",
        "AwaitingExtraTime",
        "ExtraTime",
        "Ended",
    ]
    .contains(&state.game_status.as_str());
    let _cse_temp_1 = (_cse_temp_0) || (state.is_over);
    if _cse_temp_1 {
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_first_half_tries = home_first_half_stats.as_ref().unwrap().scores.tries;
        away_first_half_tries = away_first_half_stats.as_ref().unwrap().scores.tries;
        away_first_half_total = away_first_half_stats.as_ref().unwrap().scores.total;
        home_first_half_total = home_first_half_stats.as_ref().unwrap().scores.total;
        let _cse_temp_2 = away_first_half_total > home_first_half_total;
        team_b_won_first_half = _cse_temp_2;
        let _cse_temp_3 = home_first_half_total == away_first_half_total;
        draw_first_half = _cse_temp_3;
        let _cse_temp_4 = home_first_half_total > away_first_half_total;
        team_a_won_first_half = _cse_temp_4;
        record_int("PointsHalf1A".to_string(), home_first_half_total);
        record_int("PointsHalf1B".to_string(), away_first_half_total);
        record_bool("WinHalf1A".to_string(), team_a_won_first_half);
        record_bool("WinHalf1B".to_string(), team_b_won_first_half);
        record_bool("DrawHalf1".to_string(), draw_first_half);
        record_int("TriesHalf1A".to_string(), home_first_half_tries);
        record_int("TriesHalf1B".to_string(), away_first_half_tries);
    }
    return;
}
pub fn normal_time_output_event(state: &State) {
    let mut away_first_half_stats: Option<Statistics> = None;
    let mut away_normal_time_score: i32 = 0;
    let mut away_second_half_stats: Option<Statistics> = None;
    let mut extra_time_occurred: bool = false;
    let mut home_first_half_stats: Option<Statistics> = None;
    let mut home_normal_time_score: i32 = 0;
    let mut home_second_half_stats: Option<Statistics> = None;
    let _cse_temp_0 =
        ["AwaitingExtraTime", "ExtraTime", "Ended"].contains(&state.game_status.as_str());
    if _cse_temp_0 {
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_normal_time_score = 0;
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_normal_time_score = 0;
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .clone()
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        let _cse_temp_1 = home_first_half_stats.as_ref().unwrap().scores.total
            + home_second_half_stats.as_ref().unwrap().scores.total;
        home_normal_time_score = _cse_temp_1;
        away_normal_time_score = _cse_temp_1;
        let _cse_temp_2 = home_normal_time_score == away_normal_time_score;
        extra_time_occurred = _cse_temp_2;
        record_bool("ExtraTimeOccurredMatch".to_string(), extra_time_occurred);
        record_int("PointsNormalTimeA".to_string(), home_normal_time_score);
        record_int("PointsNormalTimeB".to_string(), away_normal_time_score);
        record_bool(
            "WinNormalTimeA".to_string(),
            home_normal_time_score > away_normal_time_score,
        );
        record_bool(
            "WinNormalTimeB".to_string(),
            away_normal_time_score > home_normal_time_score,
        );
        record_int(
            "TriesHalf2A".to_string(),
            home_second_half_stats.as_ref().unwrap().scores.tries,
        );
        record_int(
            "TriesHalf2B".to_string(),
            away_second_half_stats.as_ref().unwrap().scores.tries,
        );
    }
    return;
}
pub fn player_void_output_event(state: &State) {
    let mut away_trader_players: Vec<Player> = Vec::new();
    let mut home_trader_players: Vec<Player> = Vec::new();
    if state.include_players {
        home_trader_players = state.home_players.clone();
        away_trader_players = state.away_players.clone();
        for player in home_trader_players.iter().cloned() {
            record_bool(
                format!("Player_Voided_{}", player.player_index),
                player.is_voided,
            );
        }
        for player in away_trader_players.iter().cloned() {
            record_bool(
                format!("Player_Voided_{}", player.player_index),
                player.is_voided,
            );
        }
    }
    return;
}
pub fn period_first_last_score_helper(state: &State) {
    let mut away_first_try_time: i32 = 0;
    let mut first_score_team: String = "".to_string();
    let mut first_try_team: String = "".to_string();
    let mut first_try_time: i32 = 0;
    let mut home_first_try_time: i32 = 0;
    let period_filter: bool = false;
    let period_name: String = "".to_string();
    if false {
        first_try_team = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                period_filter
                    && incident.has_points_confirmed
                    && incident.points_confirmed_score_type == "Try"
            })
            .map(|incident| incident)
            .collect::<Vec<_>>()
            .get(0usize)
            .cloned()
            .unwrap()
            .points_scored_team;
        first_score_team = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| period_filter && incident.has_points_confirmed)
            .map(|incident| incident)
            .collect::<Vec<_>>()
            .get(0usize)
            .cloned()
            .unwrap()
            .points_scored_team;
        let _cse_temp_0 = {
            let a = state
                .incidents
                .clone()
                .clone()
                .into_iter()
                .filter(|incident| {
                    period_filter
                        && incident.has_points_confirmed
                        && incident.points_confirmed_score_type == "Try"
                })
                .map(|incident| incident)
                .collect::<Vec<_>>()
                .get(0usize)
                .cloned()
                .unwrap()
                .time_elapsed;
            let b = 60;
            let q = a / b;
            let r = a % b;
            let r_negative = r < 0;
            let b_negative = b < 0;
            let r_nonzero = r != 0;
            let signs_differ = r_negative != b_negative;
            let needs_adjustment = r_nonzero && signs_differ;
            if needs_adjustment {
                q - 1
            } else {
                q
            }
        };
        first_try_time = _cse_temp_0 + 1;
        away_first_try_time = _cse_temp_0 + 1;
        home_first_try_time = _cse_temp_0 + 1;
        record_bool(
            format!("FirstTry{}A", period_name),
            first_try_team.clone() == "Home".to_string(),
        );
        record_bool(
            format!("FirstTry{}B", period_name),
            first_try_team == "Away".to_string(),
        );
        record_bool(
            format!("FirstToScore{}A", period_name),
            first_score_team.clone() == "Home".to_string(),
        );
        record_bool(
            format!("FirstToScore{}B", period_name),
            first_score_team == "Away".to_string(),
        );
        record_int(format!("FirstTryTime{}", period_name), first_try_time);
        record_int(format!("FirstTryTime{}A", period_name), home_first_try_time);
        record_int(format!("FirstTryTime{}B", period_name), away_first_try_time);
    }
    return;
}
pub fn first_to_score_tracking_event(state: &State) {
    let mut away_first_try_incident: Option<Incident> = None;
    let mut away_first_try_time: Option<i32> = None;
    let mut extra_time_first_score_incident: Option<Incident> = None;
    let mut extra_time_first_try_incident: Option<Incident> = None;
    let mut half2_et_first_score_incident: Option<Incident> = None;
    let mut half2_et_first_try_incident: Option<Incident> = None;
    let mut home_first_try_incident: Option<Incident> = None;
    let mut home_first_try_time: Option<i32> = None;
    let mut last_score_incident: Option<Incident> = None;
    let mut last_try_incident: Option<Incident> = None;
    let mut last_try_time: Option<i32> = None;
    let mut match_first_score_incident: Option<Incident> = None;
    let mut match_first_try_incident: Option<Incident> = None;
    let mut match_first_try_time: Option<i32> = None;
    let mut points_confirmed_incidents: Vec<Incident> = Vec::new();
    let mut try_incidents: Vec<Incident> = Vec::new();
    let _cse_temp_0 = state.incidents.clone().len() as i32;
    let _cse_temp_1 = _cse_temp_0 > 0;
    let _cse_temp_2 = (_cse_temp_1) || (state.is_over);
    if _cse_temp_2 {
        points_confirmed_incidents = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                incident.has_points_confirmed
                    && ["Home", "Away"].contains(&incident.points_scored_team.as_str())
            })
            .map(|incident| incident)
            .collect::<Vec<_>>();
        try_incidents = points_confirmed_incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| incident.points_confirmed_score_type == "Try")
            .map(|incident| incident)
            .collect::<Vec<_>>();
        match_first_score_incident = if !points_confirmed_incidents.clone().is_empty() {
            Some(
                points_confirmed_incidents
                    .clone()
                    .get(0usize)
                    .cloned()
                    .unwrap(),
            )
        } else {
            None
        };
        half2_et_first_score_incident = points_confirmed_incidents
            .clone()
            .iter()
            .cloned()
            .filter(|incident| (incident.period.number) as i32 - 1 >= SECOND_HALF_INDEX)
            .map(|incident| incident)
            .next();
        extra_time_first_score_incident = points_confirmed_incidents
            .iter()
            .cloned()
            .filter(|incident| (incident.period.number) as i32 - 1 == EXTRA_TIME_INDEX)
            .map(|incident| incident)
            .next();
        last_score_incident = if !points_confirmed_incidents.is_empty() {
            Some(points_confirmed_incidents.last().cloned().unwrap())
        } else {
            None
        };
        half2_et_first_try_incident = try_incidents
            .clone()
            .iter()
            .cloned()
            .filter(|incident| (incident.period.number) as i32 - 1 >= SECOND_HALF_INDEX)
            .map(|incident| incident)
            .next();
        away_first_try_incident = try_incidents
            .clone()
            .iter()
            .cloned()
            .filter(|incident| incident.points_scored_team == "Away")
            .map(|incident| incident)
            .next();
        extra_time_first_try_incident = try_incidents
            .clone()
            .iter()
            .cloned()
            .filter(|incident| (incident.period.number) as i32 - 1 == EXTRA_TIME_INDEX)
            .map(|incident| incident)
            .next();
        home_first_try_incident = try_incidents
            .iter()
            .cloned()
            .filter(|incident| incident.points_scored_team == "Home")
            .map(|incident| incident)
            .next();
        match_first_try_incident = if !try_incidents.is_empty() {
            Some(try_incidents.get(0usize).cloned().unwrap())
        } else {
            None
        };
        last_try_incident = if !try_incidents.is_empty() {
            Some(try_incidents.last().cloned().unwrap())
        } else {
            None
        };
        away_first_try_time = if away_first_try_incident.is_some() {
            Some((away_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1)
        } else {
            None
        };
        match_first_try_time = if match_first_try_incident.is_some() {
            Some((match_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1)
        } else {
            None
        };
        last_try_time = if last_try_incident.is_some() {
            Some((last_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1)
        } else {
            None
        };
        home_first_try_time = if home_first_try_incident.is_some() {
            Some((home_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1)
        } else {
            None
        };
        record_bool(
            "FirstToScore_Match_A".to_string(),
            match_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstToScore_Match_B".to_string(),
            match_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_bool(
            "FirstTry_Match_A".to_string(),
            match_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstTry_Match_B".to_string(),
            match_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_bool(
            "FirstToScore_Half2AndExtraTime_A".to_string(),
            half2_et_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstToScore_Half2AndExtraTime_B".to_string(),
            half2_et_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_bool(
            "FirstTry_Half2AndExtraTime_A".to_string(),
            half2_et_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstTry_Half2AndExtraTime_B".to_string(),
            half2_et_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_bool(
            "FirstToScore_ExtraTime_A".to_string(),
            extra_time_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstToScore_ExtraTime_B".to_string(),
            extra_time_first_score_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_bool(
            "FirstTry_ExtraTime_A".to_string(),
            extra_time_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Home",
        );
        record_bool(
            "FirstTry_ExtraTime_B".to_string(),
            extra_time_first_try_incident
                .as_ref()
                .unwrap()
                .points_scored_team
                == "Away",
        );
        record_int(
            "FirstTryTime_Half2AndExtraTime".to_string(),
            (half2_et_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1,
        );
        record_int(
            "FirstTryTime_Half2AndExtraTime_A".to_string(),
            if (half2_et_first_try_incident.is_some())
                && (half2_et_first_try_incident
                    .as_ref()
                    .unwrap()
                    .points_scored_team
                    == "Home")
            {
                (half2_et_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_Half2AndExtraTime_B".to_string(),
            if (half2_et_first_try_incident.is_some())
                && (half2_et_first_try_incident
                    .as_ref()
                    .unwrap()
                    .points_scored_team
                    == "Away")
            {
                (half2_et_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_ExtraTime".to_string(),
            if extra_time_first_try_incident.is_some() {
                (extra_time_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_ExtraTime_A".to_string(),
            if (extra_time_first_try_incident.is_some())
                && (extra_time_first_try_incident
                    .as_ref()
                    .unwrap()
                    .points_scored_team
                    == "Home")
            {
                (extra_time_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_ExtraTime_B".to_string(),
            if (extra_time_first_try_incident.is_some())
                && (extra_time_first_try_incident
                    .as_ref()
                    .unwrap()
                    .points_scored_team
                    == "Away")
            {
                (extra_time_first_try_incident.as_ref().unwrap().time_elapsed / 60) as i32 + 1
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_Match".to_string(),
            if match_first_try_time.is_some() {
                match_first_try_time
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_Match_A".to_string(),
            if home_first_try_time.is_some() {
                home_first_try_time
            } else {
                0
            },
        );
        record_int(
            "FirstTryTime_Match_B".to_string(),
            if away_first_try_time.is_some() {
                away_first_try_time
            } else {
                0
            },
        );
        record_bool(
            "LastTry_Match_A".to_string(),
            (last_try_incident.is_some())
                && (last_try_incident.as_ref().unwrap().points_scored_team == "Home"),
        );
        record_bool(
            "LastTry_Match_B".to_string(),
            (last_try_incident.is_some())
                && (last_try_incident.as_ref().unwrap().points_scored_team == "Away"),
        );
        record_bool(
            "LastToScore_Match_A".to_string(),
            (last_score_incident.is_some())
                && (last_score_incident.as_ref().unwrap().points_scored_team == "Home"),
        );
        record_bool(
            "LastToScore_Match_B".to_string(),
            (last_score_incident.is_some())
                && (last_score_incident.as_ref().unwrap().points_scored_team == "Away"),
        );
        record_int(
            "LastTryTime_Match".to_string(),
            if last_try_time.is_some() {
                last_try_time
            } else {
                0
            },
        );
    }
    return;
}
pub fn minute_winner_tracking_event(state: &State) {
    let mut final_minute: i32 = 0;
    let mut minute_intervals: Vec<i32> = Vec::new();
    let _cse_temp_0 = state.incidents.clone().len() as i32;
    let _cse_temp_1 = _cse_temp_0 > 0;
    let _cse_temp_2 = (_cse_temp_1) || (state.is_over);
    if _cse_temp_2 {
        minute_intervals = vec![10, 20, 30, 50, 60];
        let _cse_temp_3 = state.time_elapsed / 60;
        let _cse_temp_4 = (_cse_temp_3) as i32;
        final_minute = _cse_temp_4;
        for minute in minute_intervals.iter().cloned() {
            if final_minute > minute {
                let away_points_at_minute = state
                    .incidents
                    .clone()
                    .clone()
                    .into_iter()
                    .filter(|incident| {
                        ((incident.has_points_confirmed) && (incident.points_scored_team == "Away"))
                            && (incident.time_elapsed <= minute * 60)
                    })
                    .map(|incident| incident.points_scored_points)
                    .sum::<i32>();
                let home_points_at_minute = state
                    .incidents
                    .clone()
                    .clone()
                    .into_iter()
                    .filter(|incident| {
                        ((incident.has_points_confirmed) && (incident.points_scored_team == "Home"))
                            && (incident.time_elapsed <= minute * 60)
                    })
                    .map(|incident| incident.points_scored_points)
                    .sum::<i32>();
                record_bool(
                    format!("WinMinute{}MatchA", minute),
                    home_points_at_minute > away_points_at_minute,
                );
                record_bool(
                    format!("WinMinute{}MatchB", minute),
                    away_points_at_minute > home_points_at_minute,
                );
            }
        }
    }
    return;
}
pub fn player_score_tracking_event(state: &State) {
    let mut away_first_try_jersey: i32 = 0;
    let mut away_try_scorer_indices: Vec<i32> = Vec::new();
    let mut first_try_jersey: i32 = 0;
    let mut home_first_try_jersey: i32 = 0;
    let mut home_try_scorer_indices: Vec<i32> = Vec::new();
    let mut last_try_jersey: i32 = 0;
    let three_unanswered_tries_team: String = "".to_string();
    let mut try_scorer_indices: Vec<i32> = Vec::new();
    let mut try_teams: Vec<String> = Vec::new();
    if state.include_players {
        try_scorer_indices = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                incident.has_points_confirmed && incident.points_confirmed_score_type == "Try"
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        away_try_scorer_indices = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                incident.has_points_confirmed
                    && incident.points_confirmed_score_type == "Try"
                    && incident.points_scored_team == "Away"
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        try_teams = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                incident.has_points_confirmed && incident.points_confirmed_score_type == "Try"
            })
            .map(|incident| incident.points_scored_team)
            .collect::<Vec<_>>();
        home_try_scorer_indices = state
            .incidents
            .clone()
            .clone()
            .into_iter()
            .filter(|incident| {
                incident.has_points_confirmed
                    && incident.points_confirmed_score_type == "Try"
                    && incident.points_scored_team == "Home"
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        away_first_try_jersey = {
            let base = &state.away_players.clone();
            let idx: i32 = away_try_scorer_indices
                .clone()
                .get(0usize)
                .cloned()
                .unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        last_try_jersey = {
            let base = &state.all_players.clone();
            let idx: i32 = {
                let base = &try_scorer_indices.clone();
                let idx: i32 = (try_scorer_indices.clone().len() as i32).saturating_sub(1);
                let actual_idx = if idx < 0 {
                    base.len().saturating_sub(idx.abs() as usize)
                } else {
                    idx as usize
                };
                base.get(actual_idx).cloned().unwrap()
            };
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        first_try_jersey = {
            let base = &state.all_players.clone();
            let idx: i32 = try_scorer_indices.clone().get(0usize).cloned().unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        home_first_try_jersey = {
            let base = &state.home_players.clone();
            let idx: i32 = home_try_scorer_indices
                .clone()
                .get(0usize)
                .cloned()
                .unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        for index in 1..std::cmp::min(15, try_scorer_indices.clone().len() as i32) {
            record_int(format!("{}thTryScorer_Match", index), {
                let base = &try_scorer_indices.clone();
                let idx: i32 = index - 1;
                let actual_idx = if idx < 0 {
                    base.len().saturating_sub(idx.abs() as usize)
                } else {
                    idx as usize
                };
                base.get(actual_idx).cloned().unwrap()
            });
        }
        for index in 1..try_teams.clone().len() as i32 {
            record_bool(
                format!("{}thTry_Match_A", index),
                {
                    let base = &try_teams.clone();
                    let idx: i32 = index - 1;
                    let actual_idx = if idx < 0 {
                        base.len().saturating_sub(idx.abs() as usize)
                    } else {
                        idx as usize
                    };
                    base.get(actual_idx).cloned().unwrap()
                } == "Home",
            );
            record_bool(
                format!("{}thTry_Match_B", index),
                {
                    let base = &try_teams;
                    let idx: i32 = index - 1;
                    let actual_idx = if idx < 0 {
                        base.len().saturating_sub(idx.abs() as usize)
                    } else {
                        idx as usize
                    };
                    base.get(actual_idx).cloned().unwrap()
                } == "Away",
            );
        }
        record_int("FirstTryScorerJerseyMatch".to_string(), first_try_jersey);
        record_int(
            "HomeFirstTryScorerMatch".to_string(),
            home_try_scorer_indices
                .clone()
                .get(0usize)
                .cloned()
                .unwrap(),
        );
        record_int(
            "HomeFirstTryScorerJerseyMatch".to_string(),
            home_first_try_jersey,
        );
        record_int(
            "AwayFirstTryScorerMatch".to_string(),
            away_try_scorer_indices
                .clone()
                .get(0usize)
                .cloned()
                .unwrap(),
        );
        record_int(
            "AwayFirstTryScorerJerseyMatch".to_string(),
            away_first_try_jersey,
        );
        record_bool("ThreeUnansweredTries".to_string(), true);
        record_int("LastTryScorerMatch".to_string(), {
            let base = &try_scorer_indices.clone();
            let idx: i32 = (try_scorer_indices.len() as i32).saturating_sub(1);
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        });
        record_int("LastTryScorerJerseyMatch".to_string(), last_try_jersey);
        record_int("HomeLastTryScorerMatch".to_string(), {
            let base = &home_try_scorer_indices.clone();
            let idx: i32 = (home_try_scorer_indices.len() as i32).saturating_sub(1);
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        });
        record_int("AwayLastTryScorerMatch".to_string(), {
            let base = &away_try_scorer_indices.clone();
            let idx: i32 = (away_try_scorer_indices.len() as i32).saturating_sub(1);
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        });
        for player in &state.home_players {
            let player = player.clone();
            record_bool(
                format!("Player_{}_AnytimeTryScorer", player.player_index),
                player.total_statistics.scores.tries > 0,
            );
            record_bool(
                format!("Player_{}_2PlusTryScorer", player.player_index),
                player.total_statistics.scores.tries >= 2,
            );
            record_bool(
                format!("Player_{}_3PlusTryScorer", player.player_index),
                player.total_statistics.scores.tries >= 3,
            );
        }
        for player in &state.away_players {
            let player = player.clone();
            record_bool(
                format!("Player_{}_AnytimeTryScorer", player.player_index),
                player.total_statistics.scores.tries > 0,
            );
            record_bool(
                format!("Player_{}_2PlusTryScorer", player.player_index),
                player.total_statistics.scores.tries >= 2,
            );
            record_bool(
                format!("Player_{}_3PlusTryScorer", player.player_index),
                player.total_statistics.scores.tries >= 3,
            );
        }
    }
    return;
}
pub fn race_to_points_tracking_event(state: &State) {
    let mut away_running_score: i32 = 0;
    let mut home_running_score: i32 = 0;
    let mut race_to_targets: Vec<i32> = Vec::new();
    let mut remaining_targets: Vec<i32> = Vec::new();
    let mut target_index: i32 = 0;
    let _cse_temp_0 = state.incidents.clone().len() as i32;
    let _cse_temp_1 = _cse_temp_0 > 0;
    let _cse_temp_2 = (_cse_temp_1) || (state.is_over);
    if _cse_temp_2 {
        target_index = 0;
        race_to_targets = vec![10, 15, 20, 25, 30, 35, 40];
        away_running_score = 0;
        home_running_score = 0;
        for incident in &state.incidents {
            let incident = incident.clone();
            if ((incident.has_points_confirmed)
                && (["Home", "Away"].contains(&incident.points_scored_team.as_str())))
                && (target_index < race_to_targets.clone().len() as i32)
            {
                if incident.points_scored_team == "Home" {
                    home_running_score = home_running_score + incident.points_scored_points;
                } else {
                    away_running_score = away_running_score + incident.points_scored_points;
                }
                let current_target = race_to_targets
                    .clone()
                    .get(target_index as usize)
                    .cloned()
                    .unwrap();
                let home_reached_target = home_running_score >= current_target;
                let away_reached_target = away_running_score >= current_target;
                if home_reached_target != away_reached_target {
                    record_bool(
                        format!("FirstToPoints{}A", current_target),
                        home_reached_target,
                    );
                    record_bool(
                        format!("FirstToPoints{}B", current_target),
                        away_reached_target,
                    );
                    target_index = target_index + 1;
                }
            }
        }
        remaining_targets = {
            let base = &race_to_targets;
            let start = (target_index).max(0) as usize;
            if start < base.len() {
                base[start..].to_vec()
            } else {
                Vec::new()
            }
        };
        for pending_target in remaining_targets.iter().cloned() {
            record_bool(format!("FirstToPoints{}A", pending_target), false);
            record_bool(format!("FirstToPoints{}B", pending_target), false);
        }
    }
    return;
}
#[function]
pub fn add_conversion(state: &mut State) {
    let team = state.team_in_possession.clone();
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    println!(
        "{} {}",
        ">>>>>>>>>>>>>>>>Adding conversion for team:",
        state.team_in_possession.clone()
    );
    let _cse_temp_1 = team.clone() == "Home".to_string();
    if _cse_temp_1 {
        let _cse_temp_2 = state.home_match_score + CONVERSION_POINTS;
        state.home_match_score = _cse_temp_2;
    } else {
        let _cse_temp_3 = state.away_match_score + CONVERSION_POINTS;
        state.away_match_score = _cse_temp_3;
    }
    let mut team_stats = if team.clone() == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let _cse_temp_4 = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .conversions
        + 1;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .conversions = _cse_temp_4;
    let _cse_temp_5 = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + CONVERSION_POINTS;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total = _cse_temp_5;
    team_stats.total_statistics.scores.conversions = _cse_temp_4;
    team_stats.total_statistics.scores.total = _cse_temp_5;
    if state.include_players {
        let mut player = if team == "Home".to_string() {
            state.home_player_selected_for_points_market.clone()
        } else {
            state.away_player_selected_for_points_market.clone()
        };
        player.total_statistics.scores.conversions = _cse_temp_4;
        player.total_statistics.scores.total = _cse_temp_5;
        player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .conversions = _cse_temp_4;
        player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total = _cse_temp_5;
    }
}
#[doc = "Distance from centre of the field from the perspective of the team in possession."]
#[function]
pub fn calculate_dist_from_centre(state: &State) -> i32 {
    let _cse_temp_0 = state.team_in_possession.clone() == "Home";
    if _cse_temp_0 {
        return ((CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs()) as i32;
    } else {
        return ((CENTRE_OF_THE_FIELD_Y - PLAYING_FIELD_HEIGHT - state.ball_location.y).abs())
            as i32;
    }
}
#[doc = "Distance to the try line from the perspective of the team in possession."]
pub fn calculate_dist_to_try_line(state: &State) -> i32 {
    let _cse_temp_0 = state.team_in_possession.clone() == "Home";
    if _cse_temp_0 {
        return PLAYING_FIELD_WIDTH - state.ball_location.x;
    } else {
        return state.ball_location.x;
    }
}
#[function]
pub fn add_field_goal(state: &mut State, is_two_pointer: bool) {
    let team = state.team_in_possession.clone();
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    let points = if is_two_pointer {
        TWO_POINT_FIELD_GOAL_POINTS
    } else {
        FIELD_GOAL_POINTS
    };
    let _cse_temp_1 = team.clone() == "Home".to_string();
    if _cse_temp_1 {
        let _cse_temp_2 = state.home_match_score + points;
        state.home_match_score = _cse_temp_2;
    } else {
        let _cse_temp_3 = state.away_match_score + points;
        state.away_match_score = _cse_temp_3;
    }
    let mut team_stats = if team.clone() == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let _cse_temp_4 = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .field_goals
        + 1;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .field_goals = _cse_temp_4;
    let _cse_temp_5 = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + points;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total = _cse_temp_5;
    team_stats.total_statistics.scores.field_goals = _cse_temp_4;
    team_stats.total_statistics.scores.total = _cse_temp_5;
    if state.include_players {
        let mut player = if team == "Home".to_string() {
            state.home_player_selected_for_points_market.clone()
        } else {
            state.away_player_selected_for_points_market.clone()
        };
        player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total = _cse_temp_5;
        player.total_statistics.scores.total = _cse_temp_5;
    }
}
#[function]
pub fn get_time_on_field_for_position(state: &State, team: String, position: String) -> f64 {
    let players = _team_players(state, team);
    let candidate = players
        .iter()
        .cloned()
        .filter(|p| p.position.position_type == position)
        .map(|p| p)
        .next()
        .expect("StopIteration: iterator is empty");
    return (candidate.total_statistics.time_on_field) as f64;
}
pub fn get_interchanges_used(state: &State, team: String) -> f64 {
    let remaining = _get_remaining_interchanges(state, team);
    let used = MAX_INTERCHANGES - remaining;
    return (std::cmp::max(0, used)) as f64;
}
pub fn get_team_margin(state: &State, team: String) -> f64 {
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    return if team == "Home".to_string() {
        (home_score - away_score) as f64
    } else {
        (away_score - home_score) as f64
    };
}
pub fn decrement_remaining_interchanges(state: &mut State, team: String) {
    let current = _get_remaining_interchanges(state, team.clone());
    let _cse_temp_0 = std::cmp::max(0, current - 1);
    let remaining = _cse_temp_0;
    _set_remaining_interchanges(state, team, remaining);
}
pub fn _select_bench_player_index(players: &Vec<Player>, starters: i32) -> i32 {
    let available = (starters..players.clone().len() as i32)
        .filter(|&idx| !players.get(idx as usize).cloned().unwrap().is_injured)
        .map(|idx| idx)
        .collect::<Vec<_>>();
    return *available.choose(&mut rand::thread_rng()).unwrap();
}
pub fn _swap_team_statistics(state: &State, team: String, off_slot: i32, bench_slot: i32) {
    let mut stats = _team_statistic(state, team);
    {
        let (_swap_tmp0, _swap_tmp1) = (
            stats
                .player_statistics
                .clone()
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            stats
                .player_statistics
                .clone()
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        stats.player_statistics.clone()[off_slot as usize] = _swap_tmp0;
        stats.player_statistics.clone()[bench_slot as usize] = _swap_tmp1;
    }
    {
        let (_swap_tmp0, _swap_tmp1) = (
            stats
                .sin_bin_players
                .clone()
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            stats
                .sin_bin_players
                .clone()
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        stats.sin_bin_players.clone()[off_slot as usize] = _swap_tmp0;
        stats.sin_bin_players.clone()[bench_slot as usize] = _swap_tmp1;
    }
}
pub fn _swap_game_statistics(state: &State, team: String, off_slot: i32, bench_slot: i32) {
    let mut mirrored = if team.clone() == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let team_stats = _team_statistic(state, team);
    let _cse_temp_0 = mirrored == team_stats;
    if _cse_temp_0 {
        return;
    }
    {
        let (_swap_tmp0, _swap_tmp1) = (
            mirrored
                .player_statistics
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            mirrored
                .player_statistics
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        mirrored.player_statistics[off_slot as usize] = _swap_tmp0;
        mirrored.player_statistics[bench_slot as usize] = _swap_tmp1;
    }
    {
        let (_swap_tmp0, _swap_tmp1) = (
            mirrored
                .sin_bin_players
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            mirrored
                .sin_bin_players
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        mirrored.sin_bin_players[off_slot as usize] = _swap_tmp0;
        mirrored.sin_bin_players[bench_slot as usize] = _swap_tmp1;
    }
}
pub fn _refresh_all_players(state: &mut State) {
    let mut combined = vec![];
    for team in ["Home", "Away"] {
        combined.extend(_team_players(state, team.to_string()).iter().cloned());
    }
    state.all_players.clear();
    state.all_players.extend(combined);
}
pub fn _team_players(state: &State, team: String) -> Vec<Player> {
    return if team == "Home".to_string() {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
}
pub fn _team_statistic(state: &State, team: String) -> TeamStatistics {
    return if team == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
}
pub fn _get_remaining_interchanges(state: &State, team: String) -> i32 {
    return if team == "Home".to_string() {
        state.home_remaining_interchanges
    } else {
        state.away_remaining_interchanges
    };
}
pub fn _set_remaining_interchanges(state: &mut State, team: String, value: i32) {
    let _cse_temp_0 = team == "Home".to_string();
    if _cse_temp_0 {
        state.home_remaining_interchanges = value;
    } else {
        state.away_remaining_interchanges = value;
    }
}
#[doc = "Margin from the perspective of the penalty-awarded team(matches C# processors)."]
#[function]
pub fn calculate_foul_team_margin(state: &State) -> i32 {
    let _cse_temp_0 = state.current_play_type.clone() == "WonPenalty";
    let penalty_team;
    if _cse_temp_0 {
        penalty_team = state.team_in_possession.clone();
    } else {
        penalty_team = if state.team_in_possession.clone() == "Home" {
            "Away".to_string()
        } else {
            "Home".to_string()
        };
    }
    let _cse_temp_1 = penalty_team == "Home".to_string();
    if _cse_temp_1 {
        return state.home_match_score - state.away_match_score;
    } else {
        return state.away_match_score - state.home_match_score;
    }
}
#[doc = "Margin from the perspective of the team in possession."]
pub fn calculate_margin(state: &State) -> i32 {
    let _cse_temp_0 = state.team_in_possession.clone() == "Home";
    if _cse_temp_0 {
        return state.home_match_score - state.away_match_score;
    } else {
        return state.away_match_score - state.home_match_score;
    }
}
#[function]
pub fn btf(value: bool) -> f64 {
    return if value { 1.0 } else { 0.0 };
}
#[doc = "Sample from a discrete distribution using inverse transform sampling."]
pub fn sample(distribution: &Vec<f64>, k: f64) -> i32 {
    let mut cumulative = 0.0;
    for (i, p) in distribution
        .clone()
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
    {
        let i = i as i32;
        cumulative = cumulative + p;
        if k < cumulative {
            return i;
        }
    }
    return (distribution.len() as i32).saturating_sub(1) as i32;
}
#[doc = "Sample from an unnormalized distribution, scaling k by the sum."]
pub fn sample_scaled(distribution: &Vec<f64>, distribution_sum: f64, k: f64) -> i32 {
    let _cse_temp_0 = k * distribution_sum;
    let threshold = _cse_temp_0;
    let mut cumulative = 0.0;
    for (i, p) in distribution
        .clone()
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
    {
        let i = i as i32;
        cumulative = cumulative + p;
        if threshold < cumulative {
            return i;
        }
    }
    return (distribution.len() as i32).saturating_sub(1) as i32;
}
#[doc = "Apply softmax transformation, returning a new list."]
pub fn softmax(logits: Vec<f64>) -> Vec<f64> {
    let _cse_temp_0 = logits
        .clone()
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let max_logit = _cse_temp_0;
    let exps = logits
        .clone()
        .into_iter()
        .map(|x| (x - max_logit as f64).exp())
        .collect::<Vec<_>>();
    let _cse_temp_1 = exps.clone().iter().sum::<f64>();
    let total = _cse_temp_1;
    return exps
        .clone()
        .into_iter()
        .map(|e| (e as f64) / (total as f64))
        .collect::<Vec<_>>();
}
#[doc = "Normalize values to sum to 1, returning a new list."]
pub fn normalize(values: Vec<f64>) -> Vec<f64> {
    let _cse_temp_0 = values.clone().iter().sum::<f64>();
    let total = _cse_temp_0;
    return values
        .clone()
        .into_iter()
        .map(|v| (v as f64) / (total as f64))
        .collect::<Vec<_>>();
}
pub fn distribution(mut values: Vec<f64>) -> Vec<f64> {
    let _cse_temp_0 = values.clone().get(1usize).cloned().unwrap() * 0.9;
    values.clone().insert((1) as usize, _cse_temp_0);
    let _cse_temp_1 = 1.0 - values.get(1usize).cloned().unwrap();
    values.insert((0) as usize, _cse_temp_1);
    return values;
}
#[function]
pub fn add_penalty(state: &mut State) {
    let team = state.team_in_possession.clone();
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    let _cse_temp_1 = team.clone() == "Home".to_string();
    if _cse_temp_1 {
        let _cse_temp_2 = state.home_match_score + PENALTY_POINTS;
        state.home_match_score = _cse_temp_2;
    } else {
        let _cse_temp_3 = state.away_match_score + PENALTY_POINTS;
        state.away_match_score = _cse_temp_3;
    }
    let mut team_stats = if team.clone() == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let _cse_temp_4 = team_stats.total_statistics.scores.penalties + 1;
    team_stats.total_statistics.scores.penalties = _cse_temp_4;
    let _cse_temp_5 = team_stats.total_statistics.scores.total + PENALTY_POINTS;
    team_stats.total_statistics.scores.total = _cse_temp_5;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .penalties = _cse_temp_4;
    team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total = _cse_temp_5;
    if state.include_players {
        let mut player = if team == "Home".to_string() {
            state.home_player_selected_for_points_market.clone()
        } else {
            state.away_player_selected_for_points_market.clone()
        };
        player.total_statistics.scores.conversions = _cse_temp_4;
        player.total_statistics.scores.total = _cse_temp_5;
        player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .conversions = _cse_temp_4;
        player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total = _cse_temp_5;
    }
}
#[function]
pub fn calculate_player_of_match_distributions(state: &State) -> Vec<f64> {
    let players = state.all_players.clone();
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    let margin = home_score - away_score;
    let total_points = home_score + away_score;
    return players
        .clone()
        .into_iter()
        .map(|player| _calculate_percentage_chance(&player, margin, total_points))
        .collect::<Vec<_>>();
}
pub fn _calculate_percentage_chance(player: &Player, margin: i32, total_points: i32) -> f64 {
    let margin_factor = _calculate_margin_factor(player.delta_strength.clone(), margin);
    let match_stats = player.total_statistics.clone();
    let match_tries = match_stats.scores.tries;
    let match_score = match_stats.scores.total;
    let pom_percentage = player.player_of_the_match_percentage;
    let team_won = _team_won(player, margin);
    let pom_factor = _calculate_pom_chance(
        player.tries_strength.clone(),
        match_tries,
        match_score,
        pom_percentage,
        team_won,
    );
    let total_factor = _calculate_total_factor(player.total_strength.clone(), total_points);
    return margin_factor * pom_factor * total_factor + 0.5 * pom_percentage;
}
pub fn _calculate_margin_factor(strength: String, margin: i32) -> f64 {
    let _cse_temp_0 = margin.abs();
    let abs_margin = _cse_temp_0;
    let _cse_temp_1 = strength.clone() == "LOW".to_string();
    if _cse_temp_1 {
        return f64::max(2.5 - (abs_margin as f64) / 10.0, 0.5);
    }
    let _cse_temp_2 = strength == "HIGH".to_string();
    if _cse_temp_2 {
        return f64::min(0.5 * (abs_margin as f64) / 10.0, 2.5);
    }
    return 1.0;
}
pub fn _calculate_total_factor(strength: String, total_points: i32) -> f64 {
    let _cse_temp_0 = strength.clone() == "LOW".to_string();
    if _cse_temp_0 {
        return f64::max(
            3.0 - (0.175 * (total_points as f64) / 10.0 as f64).exp(),
            0.1,
        );
    }
    let _cse_temp_1 = strength == "HIGH".to_string();
    if _cse_temp_1 {
        return (0.175 * (total_points as f64) / 9.9 as f64).exp() - 1.0;
    }
    return 1.0;
}
pub fn _calculate_pom_chance(
    tries_strength: String,
    match_tries: i32,
    match_score: i32,
    pom_percentage: f64,
    team_won: bool,
) -> f64 {
    let try_factor = _calculate_try_factor(tries_strength, match_tries);
    let win_factor = if team_won { 500 } else { 0 };
    let _cse_temp_0 = match_tries * 4;
    let points_scored = match_score - _cse_temp_0;
    let _cse_temp_1 = 0.3357 * (points_scored as f64);
    let _cse_temp_2 = f64::max(1.0, _cse_temp_1 - 1.75);
    let points_factor = _cse_temp_2;
    let _cse_temp_3 = pom_percentage * (win_factor as f64);
    let _cse_temp_4 = pom_percentage + _cse_temp_3 + try_factor;
    let _cse_temp_5 = _cse_temp_4 * points_factor;
    let pom_weight = _cse_temp_5;
    return f64::max(pom_weight, MIN_POM_WEIGHT);
}
pub fn _calculate_try_factor(strength: String, tries: i32) -> f64 {
    let coefficient = {
        let mut map = HashMap::new();
        map.insert("LOW".to_string(), 1.0);
        map.insert("MID".to_string(), 3.0);
        map.insert("HIGH".to_string(), 10.0);
        map
    }
    .get(&strength)
    .cloned()
    .unwrap_or(3.0);
    let _cse_temp_0 = tries <= 0;
    if _cse_temp_0 {
        return 0.0;
    }
    return (coefficient * (tries as f64).powf(4.5 as f64) as f64) / 2.0;
}
pub fn _team_won(player: &Player, margin: i32) -> bool {
    let is_home = player.is_home_team;
    let _cse_temp_0 = margin == 0;
    if _cse_temp_0 {
        return false;
    }
    return if is_home { margin > 0 } else { margin < 0 };
}
#[function]
pub fn _goal_line_bucket(grid_coordinate: i32, k: f64) -> i32 {
    let _cse_temp_0 = grid_coordinate < GOAL_LINE_GRID_START_INDEX;
    if _cse_temp_0 {
        return -1;
    }
    let _cse_temp_1 = grid_coordinate % 6;
    let y_band = _cse_temp_1;
    let weights = GOAL_LINE_ZONE_WEIGHTS
        .clone()
        .get(y_band as usize)
        .cloned()
        .unwrap();
    return sample(&weights, k);
}
pub fn compute_conversion_location_from_grid(grid_coordinate: i32, k: f64) -> i32 {
    let bucket = _goal_line_bucket(grid_coordinate, k);
    let _cse_temp_0 = bucket < 0;
    if _cse_temp_0 {
        let _cse_temp_1 = CONVERSION_X_VALUES.clone().len() as i32;
        let _cse_temp_2 = {
            let a = _cse_temp_1;
            let b = 2;
            let q = a / b;
            let r = a % b;
            let r_negative = r < 0;
            let b_negative = b < 0;
            let r_nonzero = r != 0;
            let signs_differ = r_negative != b_negative;
            let needs_adjustment = r_nonzero && signs_differ;
            if needs_adjustment {
                q - 1
            } else {
                q
            }
        };
        let default_band = _cse_temp_2;
        return CONVERSION_X_VALUES
            .clone()
            .get(default_band as usize)
            .cloned()
            .unwrap();
    }
    let _cse_temp_3 = {
        let a = bucket;
        let b = 5;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0;
        let b_negative = b < 0;
        let r_nonzero = r != 0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1
        } else {
            q
        }
    };
    let bucket_row = _cse_temp_3;
    let _cse_temp_4 = (bucket_row as f64) * GOAL_LINE_Y_WIDTH;
    let _cse_temp_5 = {
        let a = GOAL_LINE_Y_WIDTH;
        let b = 2 as f64;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0.0;
        let b_negative = b < 0.0;
        let r_nonzero = r != 0.0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1.0
        } else {
            q
        }
    };
    let _cse_temp_6 = (_cse_temp_4 + (_cse_temp_5 as f64)) as i32;
    let base_y = _cse_temp_6;
    let _cse_temp_7 = grid_coordinate % 6;
    let _cse_temp_8 = _cse_temp_7 * GOAL_LINE_Y_HEIGHT_INCREMENT;
    let goal_line_field_y = base_y + _cse_temp_8;
    let _cse_temp_9 = {
        let a = goal_line_field_y;
        let b = 35;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0;
        let b_negative = b < 0;
        let r_nonzero = r != 0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1
        } else {
            q
        }
    };
    let mut conversion_band = _cse_temp_9;
    let _cse_temp_10 = CONVERSION_X_VALUES.clone().len() as i32;
    let _cse_temp_11 = std::cmp::min(conversion_band, _cse_temp_10 - 1);
    let _cse_temp_12 = std::cmp::max(0, _cse_temp_11);
    conversion_band = _cse_temp_12;
    let conversion_location = CONVERSION_X_VALUES
        .clone()
        .get(conversion_band as usize)
        .cloned()
        .unwrap();
    return conversion_location;
}
pub fn goal_line_position(goal_line_index: i32, grid_index: i32) -> FieldPosition {
    let _cse_temp_0 = goal_line_index % 5;
    let _cse_temp_1 = _cse_temp_0 * GOAL_LINE_X_WIDTH;
    let _cse_temp_2 = {
        let a = GOAL_LINE_X_WIDTH;
        let b = 2;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0;
        let b_negative = b < 0;
        let r_nonzero = r != 0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1
        } else {
            q
        }
    };
    let _cse_temp_3 = PLAYING_FIELD_WIDTH + _cse_temp_1 + _cse_temp_2;
    let x = _cse_temp_3;
    let _cse_temp_4 = {
        let a = goal_line_index;
        let b = 5;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0;
        let b_negative = b < 0;
        let r_nonzero = r != 0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1
        } else {
            q
        }
    };
    let _cse_temp_5 = (_cse_temp_4 as f64) * GOAL_LINE_Y_WIDTH;
    let _cse_temp_6 = GOAL_LINE_Y_WIDTH / (2 as f64);
    let _cse_temp_7 = (_cse_temp_5 + _cse_temp_6) as i32;
    let mut y = _cse_temp_7;
    let _cse_temp_8 = grid_index % 6;
    let _cse_temp_9 = _cse_temp_8 * GOAL_LINE_Y_HEIGHT_INCREMENT;
    y = y + _cse_temp_9;
    return FieldPosition::new(x, y);
}
pub fn convert_field_position(state: &State, grid_result: String, team: String) -> FieldPosition {
    let _cse_temp_0 = GRID_RESULT
        .clone()
        .iter()
        .position(|x| x == &grid_result.clone())
        .map(|i| i as i32)
        .expect("ValueError: value is not in list")
        % 6;
    let y_position = _cse_temp_0;
    let _cse_temp_1 = {
        let a = GRID_RESULT
            .clone()
            .iter()
            .position(|x| x == &grid_result)
            .map(|i| i as i32)
            .expect("ValueError: value is not in list");
        let b = 6;
        let q = a / b;
        let r = a % b;
        let r_negative = r < 0;
        let b_negative = b < 0;
        let r_nonzero = r != 0;
        let signs_differ = r_negative != b_negative;
        let needs_adjustment = r_nonzero && signs_differ;
        if needs_adjustment {
            q - 1
        } else {
            q
        }
    };
    let x_position = _cse_temp_1;
    let _cse_temp_2 = team == "Home".to_string();
    let y_range;
    let x_range;
    if _cse_temp_2 {
        x_range = X_VALUES.clone().get(x_position as usize).cloned().unwrap();
        y_range = Y_VALUES.clone().get(y_position as usize).cloned().unwrap();
    } else {
        x_range = X_VALUES_AWAY
            .clone()
            .get(x_position as usize)
            .cloned()
            .unwrap();
        y_range = Y_VALUES_AWAY
            .clone()
            .get(y_position as usize)
            .cloned()
            .unwrap();
    }
    return FieldPosition::new(
        rand::thread_rng().gen_range(x_range.0..=x_range.1 - 1),
        rand::thread_rng().gen_range(y_range.0..=y_range.1 - 1),
    );
}
pub fn derive_grid_coordinate(position: &FieldPosition, team: String) -> i32 {
    let _cse_temp_0 = team.contains(&"Away");
    let y_ranges;
    let x_ranges;
    if _cse_temp_0 {
        x_ranges = X_VALUES_AWAY.clone();
        y_ranges = Y_VALUES_AWAY.clone();
    } else {
        x_ranges = X_VALUES.clone();
        y_ranges = Y_VALUES.clone();
    }
    let x_index = _resolve_index(position.x, x_ranges);
    let y_index = _resolve_index(position.y, y_ranges);
    let _cse_temp_1 = x_index < 0;
    let _cse_temp_2 = y_index < 0;
    let _cse_temp_3 = (_cse_temp_1) || (_cse_temp_2);
    if _cse_temp_3 {
        return -1;
    }
    return x_index * 6 + y_index;
}
pub fn get_goal_line_coordinate(state: &State) -> i32 {
    let grid_coordinate = derive_grid_coordinate(
        &state.ball_location.clone(),
        state.team_in_possession.clone(),
    );
    let _cse_temp_0 = grid_coordinate < 0;
    if _cse_temp_0 {
        return 65;
    }
    return 65 + grid_coordinate % 6;
}
pub fn _resolve_index(coordinate: i32, ranges: Vec<(i32, i32)>) -> i32 {
    for (index, range_bounds) in ranges
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
    {
        let index = index as i32;
        let (lower, upper) = range_bounds;
        if (lower <= coordinate) && (coordinate < upper) {
            return index;
        }
    }
    return -1;
}
#[function]
pub fn record_player_sin_bin(state: &State, player_index: i32, team: String, sin_bin_type: String) {
    let players = if team.clone() == "Home".to_string() {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
    let mut player = _resolve_player(&players, player_index);
    if player.clone().is_some() {
        let mut sin_bin_collection = if team.clone() == "Home".to_string() {
            state.home_sin_bin.clone()
        } else {
            state.away_sin_bin.clone()
        };
        let _cse_temp_0 = !sin_bin_collection.contains(&player.clone().unwrap());
        if _cse_temp_0 {
            sin_bin_collection.push(player.clone().unwrap());
        }
        let seconds_elapsed = state.time_elapsed;
        let duration = if sin_bin_type.clone() == "YellowCard".to_string() {
            YELLOW_CARD_SECONDS
        } else {
            0
        };
        player.as_mut().unwrap().sin_bin_status = sin_bin_type;
        player.as_mut().unwrap().return_from_sin_bin_time = seconds_elapsed + duration;
        player.as_mut().unwrap().on_field = false;
        player.as_mut().unwrap().sin_bin_sent_off = seconds_elapsed;
        rebuild_sin_bin_players(state, team.to_string());
    }
}
pub fn _resolve_player(players: &Vec<Player>, candidate_index: i32) -> Option<Player> {
    for player in players.clone().iter().cloned() {
        if player.clone().player_index == candidate_index {
            return Some(player.clone());
        }
    }
    let target_position = PLAYER_MODEL_POSITIONS
        .clone()
        .get(candidate_index as usize)
        .cloned()
        .unwrap();
    for player in players.clone().iter().cloned() {
        if (player.position.position_type.clone() == target_position.clone()) && (player.on_field) {
            return Some(player);
        }
    }
    for player in players.iter().cloned() {
        if player.position.position_type.clone() == target_position {
            return Some(player);
        }
    }
    return None;
}
pub fn rebuild_sin_bin_players(state: &State, team: String) {
    let team_names = if !team.clone().is_empty() {
        vec![team]
    } else {
        vec!["Home".to_string(), "Away".to_string()]
    };
    for team_name in team_names.iter().cloned() {
        let sin_bin_collection = if team_name.clone() == "Home".to_string() {
            state.home_sin_bin.clone()
        } else {
            state.away_sin_bin.clone()
        };
        let sin_bin_players = sin_bin_collection
            .clone()
            .into_iter()
            .filter(|player| player.sin_bin_status.clone() != "NotSet")
            .map(|player| player)
            .collect::<Vec<_>>();
        let mut team_stats = if team_name == "Home".to_string() {
            state.home_statistics.clone()
        } else {
            state.away_statistics.clone()
        };
        team_stats.sin_bin_players = sin_bin_players;
    }
}
#[function]
pub fn record_tackle(state: &State) {
    println!("{:?}", state);
    let team = state.team_in_possession.clone();
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    let mut team_stats = if team == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let _cse_temp_1 = team_stats
        .period_statistics
        .clone()
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .tackles
        + 1;
    team_stats.period_statistics[period_idx as usize].tackles = _cse_temp_1;
    team_stats.total_statistics.tackles = _cse_temp_1;
}
#[function]
pub fn _create_scores() -> Scores {
    return Scores::new(0, 0, 0, 0, 0);
}
pub fn _create_statistics() -> Statistics {
    return Statistics::new(0, 0, _create_scores(), 0, 0);
}
pub fn _create_player_statistics(player: &Player) -> PlayerStatistics {
    let period_breakdown = (0..NUM_PERIODS)
        .map(|_| _create_statistics())
        .collect::<Vec<_>>();
    return PlayerStatistics::new(
        player.jersey_number,
        0,
        period_breakdown,
        player.player_index,
        _create_scores(),
        0,
        0.0,
        _create_statistics(),
    );
}
pub fn _create_team_statistics() -> TeamStatistics {
    return TeamStatistics::new(
        (0..NUM_PERIODS)
            .map(|_| _create_statistics())
            .collect::<Vec<_>>(),
        vec![],
        vec![],
        _create_statistics(),
    );
}
pub fn get_team_handicap(state: &State) -> f64 {
    let _cse_temp_0 = state.team_in_possession.clone() == "Home";
    if _cse_temp_0 {
        return state.simulation_invariants.home_handicap;
    }
    return state.simulation_invariants.away_handicap;
}
pub fn swap_possession(state: &mut State) {
    let _cse_temp_0 = state.set + 1;
    state.set = _cse_temp_0;
    state.team_in_possession = if state.team_in_possession.clone() == "Home" {
        "Away".to_string()
    } else {
        "Home".to_string()
    };
    state.tackles = STARTING_TACKLE;
}
pub fn setup_statistics(state: &mut State) {
    state.home_statistics = _create_team_statistics();
    state.away_statistics = _create_team_statistics();
    state.home_sin_bin = vec![];
    state.away_sin_bin = vec![];
    for team in ["Home", "Away"] {
        let players = _get_players(state, team.clone().to_string());
        let mut team_stats = _get_team_stats(state, team.clone().to_string());
        team_stats.player_statistics = vec![];
        for mut player in players.iter().cloned() {
            player.period_statistics = (0..NUM_PERIODS)
                .map(|_| _create_player_statistics(&player))
                .collect::<Vec<_>>();
            let total_statistics = _create_player_statistics(&player);
            player.total_statistics = total_statistics.clone();
            team_stats.player_statistics.push(total_statistics);
        }
        rebuild_sin_bin_players(state, team.to_string());
    }
}
pub fn log_state(state: &State) {
    println!("{:?}", state);
}
pub fn get_team_try_distributions(state: &State) -> Vec<f64> {
    let players = _get_players(state, state.team_in_possession.clone());
    return players
        .clone()
        .into_iter()
        .map(|p| p.tries_percentage)
        .collect::<Vec<_>>();
}
pub fn apply_time_on_ground(state: &State, team: String, seconds: f64) {
    let players = _get_players(state, team.clone().to_string());
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    for mut player in {
        let base = &players;
        let stop = (NUM_STARTING_PLAYERS_PER_TEAM).max(0) as usize;
        base[..stop.min(base.len())].to_vec()
    } {
        player.period_statistics[period_idx as usize].time_on_field = player
            .period_statistics
            .clone()
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .time_on_field
            + seconds;
        player.total_statistics.time_on_field = player.total_statistics.time_on_field + seconds;
    }
    for mut s in {
        let base = &_get_team_stats(state, team.to_string()).player_statistics;
        let stop = (NUM_STARTING_PLAYERS_PER_TEAM).max(0) as usize;
        base[..stop.min(base.len())].to_vec()
    } {
        s.time_on_field = s.time_on_field + seconds;
    }
}
pub fn _get_team_stats(state: &State, team: String) -> TeamStatistics {
    return if team == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
}
pub fn _get_players(state: &State, team: String) -> Vec<Player> {
    return if team == "Home".to_string() {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
}
#[function]
pub fn add_try(state: &mut State) {
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    let _cse_temp_1 = state.team_in_possession.clone() == "Home";
    if _cse_temp_1 {
        let _cse_temp_2 = state.home_match_score + TRY_POINTS;
        state.home_match_score = _cse_temp_2;
        let _cse_temp_3 = state
            .home_statistics
            .period_statistics
            .clone()
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries
            + 1;
        state
            .home_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries = _cse_temp_3;
        state
            .home_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total = _cse_temp_2;
        state.home_statistics.total_statistics.scores.tries = _cse_temp_3;
        state.home_statistics.total_statistics.scores.total = _cse_temp_2;
    } else {
        let _cse_temp_4 = state.away_match_score + TRY_POINTS;
        state.away_match_score = _cse_temp_4;
        let _cse_temp_5 = state
            .away_statistics
            .period_statistics
            .clone()
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries
            + 1;
        state
            .away_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries = _cse_temp_5;
        state
            .away_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total = _cse_temp_4;
        state.away_statistics.total_statistics.scores.tries = _cse_temp_5;
        state.away_statistics.total_statistics.scores.total = _cse_temp_4;
    }
}
pub fn assign_try(state: &mut State, player_index: i32, team: String) {
    let _cse_temp_0 = state.period.number - 1;
    let period_idx = _cse_temp_0;
    let players = if team.clone() == "Home".to_string() {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
    let (player_list_idx, mut player) = players
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
        .filter(|(i, p)| p.player_index == player_index)
        .map(|(i, p)| (i, p))
        .next()
        .expect("StopIteration: iterator is empty");
    state
        .period_try_scorers
        .clone()
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .push(player_list_idx);
    state.total_try_scorers.push(player_list_idx);
    let _cse_temp_1 = team.clone() == "Home".to_string();
    if _cse_temp_1 {
        state
            .home_period_try_scorers
            .clone()
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .push(player_list_idx);
        state.home_total_try_scorers.push(player_list_idx);
    } else {
        state
            .away_period_try_scorers
            .clone()
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .push(player_list_idx);
        state.away_total_try_scorers.push(player_list_idx);
    }
    let _cse_temp_2 = player
        .period_statistics
        .clone()
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .tries
        + 1;
    player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .tries = _cse_temp_2;
    let _cse_temp_3 = player
        .period_statistics
        .clone()
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + TRY_POINTS;
    player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total = _cse_temp_3;
    player.total_statistics.scores.tries = _cse_temp_2;
    player.total_statistics.scores.total = _cse_temp_3;
    let mut team_stats = if team == "Home".to_string() {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    team_stats
        .player_statistics
        .get(player_list_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .tries = _cse_temp_2;
    team_stats
        .player_statistics
        .get(player_list_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total = _cse_temp_3;
}
pub fn nrl(state: &mut State) {
    let mut current_minute: i32 = 0;
    let mut field_goal_attempt_result: i32 = 0;
    let mut minute_delta: i32 = 0;
    let mut previous_minute: i32 = 0;
    setup_statistics(state);
    execute_events(state);
    state.period.number = 1;
    state.include_players = false;
    state.period.name = "FirstHalf".to_string();
    execute_events(state);
    log_state(state);
    execute_events(state);
    while true {
        state.previous_ball_location = state.clone().ball_location.clone();
        state.previous_play_type = state.clone().current_play_type.clone();
        execute_events(state);
        let field_goal_decision: FieldGoalDecisionOutputs = field_goal_decision(state);
        execute_events(state);
        if field_goal_decision.attempt {
            let field_goal_attempt_outcome: FieldGoalAttemptOutputs = field_goal_attempt(state);
            execute_events(state);
            field_goal_attempt_result = (field_goal_attempt_outcome.attempt) as i32;
            execute_events(state);
        } else {
            field_goal_attempt_result = 0;
            execute_events(state);
        }
        execute_events(state);
        if field_goal_attempt_result == 1 {
            field_goal(state);
            execute_events(state);
        } else {
            let next_play_result: NextPlayOutputs = next_play(state);
            execute_events(state);
            state.current_play_type = next_play_result.play_type.clone().to_string();
            execute_events(state);
            if state.clone().current_play_type.clone() == "LineDropout" {
                swap_possession(state);
                execute_events(state);
                kickoff(state, true, true);
                execute_events(state);
            } else {
                let xy_result: XyOutputs = xy(state);
                execute_events(state);
                state.ball_location = xy_result.field_position.clone();
                execute_events(state);
                if state.clone().current_play_type.clone() == "WonPenalty" {
                    process_penalty(state);
                    execute_events(state);
                } else {
                    if state.clone().current_play_type.clone() == "KickRetain" {
                    } else {
                        if state.clone().current_play_type.clone() == "KickRetainTry" {
                            process_try(state);
                            execute_events(state);
                        } else {
                            if state.clone().current_play_type.clone() == "KickTurnover" {
                                swap_possession(state);
                                execute_events(state);
                            } else {
                                if state.clone().current_play_type.clone() == "ErrorDefence" {
                                    state.tackles = STARTING_TACKLE;
                                    execute_events(state);
                                } else {
                                    if state.clone().current_play_type.clone() == "RunTry" {
                                        process_try(state);
                                        execute_events(state);
                                    } else {
                                        if state.clone().current_play_type.clone()
                                            == "KickRetainTackle"
                                        {
                                            process_tackle(state);
                                            execute_events(state);
                                        } else {
                                            if state.clone().current_play_type.clone()
                                                == "ConcededPenalty"
                                            {
                                                process_penalty(state);
                                                execute_events(state);
                                            } else {
                                                if state.clone().current_play_type.clone() == "Pass"
                                                {
                                                } else {
                                                    if state.clone().current_play_type.clone()
                                                        == "ErrorAttack"
                                                    {
                                                        swap_possession(state);
                                                        execute_events(state);
                                                    } else {
                                                        if state.clone().current_play_type.clone()
                                                            == "Run"
                                                        {
                                                        } else {
                                                            if state
                                                                .clone()
                                                                .current_play_type
                                                                .clone()
                                                                == "RunTackle"
                                                            {
                                                                process_tackle(state);
                                                                execute_events(state);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                execute_events(state);
            }
            execute_events(state);
        }
        execute_events(state);
        previous_minute = (state.time_elapsed / 60) as i32;
        execute_events(state);
        clock(state);
        execute_events(state);
        current_minute = (state.time_elapsed / 60) as i32;
        execute_events(state);
        minute_delta = current_minute - previous_minute;
        execute_events(state);
        if (state.include_players) && (minute_delta > 0) {
            apply_time_on_ground(state, "Home".to_string(), (minute_delta * 60) as f64);
            execute_events(state);
            apply_time_on_ground(state, "Away".to_string(), (minute_delta * 60) as f64);
            execute_events(state);
            interchange(state);
            execute_events(state);
        }
        execute_events(state);
        check_game_status(state);
        execute_events(state);
        if state.is_over {
            break;
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    return;
}
pub fn unnamed(state: &State) {
    return;
}
pub fn check_game_status(state: &mut State) {
    let _cse_temp_0 = state.clone().time_elapsed > HALF_LENGTH_IN_SECONDS;
    let _cse_temp_1 = state.clone().period.name.clone() != "ExtraTime";
    let _cse_temp_2 = (_cse_temp_0) && (_cse_temp_1);
    if _cse_temp_2 {
        let _cse_temp_3 = state.clone().period.name.clone() == "FirstHalf";
        if _cse_temp_3 {
            state.period.name = "SecondHalf".to_string();
            state.game_status = "Period2".to_string();
            state.period.number = 2;
            execute_events(state);
        } else {
            let _cse_temp_4 = state.clone().time_elapsed >= GAME_LENGTH_IN_SECONDS;
            if _cse_temp_4 {
                let _cse_temp_5 = state.clone().home_match_score == state.clone().away_match_score;
                if _cse_temp_5 {
                    state.is_extratime = true;
                    state.period.name = "ExtraTime".to_string();
                    state.game_status = "ExtraTime".to_string();
                    state.period.number = 3;
                    execute_events(state);
                } else {
                    state.game_status = "Ended".to_string();
                    state.is_over = true;
                    execute_events(state);
                }
                execute_events(state);
            }
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    let _cse_temp_6 = state.period.name.clone() == "ExtraTime";
    if _cse_temp_6 {
        let _cse_temp_7 =
            state.time_elapsed >= GAME_LENGTH_IN_SECONDS + EXTRA_TIME_LENGTH_IN_SECONDS;
        let _cse_temp_8 = state.home_match_score != state.away_match_score;
        let _cse_temp_9 = (_cse_temp_7) || (_cse_temp_8);
        if _cse_temp_9 {
            state.is_over = true;
            state.game_status = "Ended".to_string();
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    return;
}
pub fn clock(state: &mut State) -> ClockOutputs {
    let mut seconds_to_add: f64 = 0.0;
    let mut seconds_to_add_initial: f64 = 0.0;
    let mut time_adjustment: f64 = 0.0;
    let model: ClockInferOutputs0 = infer::<ClockInferOutputs0>(
        "clock".to_string(),
        vec![
            btf(state.clone().current_play_type.clone() == "Pass"),
            btf(state.clone().current_play_type.clone() == "RunTackle"),
            btf(state.clone().current_play_type.clone() == "KickTurnover"),
            btf(state.clone().current_play_type.clone() == "ErrorAttack"),
            btf(state.clone().current_play_type.clone() == "ErrorDefence"),
            btf(state.clone().current_play_type.clone() == "RunTry"),
            btf(state.clone().current_play_type.clone() == "ConcededPenalty"),
            btf(state.clone().current_play_type.clone() == "WonPenalty"),
            btf(state.current_play_type.clone() == "LineDropout"),
            btf(state.current_play_type.clone() == "KickRetainTackle"),
            btf(state.current_play_type.clone() == "KickRetainTry"),
            btf(state.current_play_type.clone() == "KickRetain"),
            btf(state.previous_play_type.clone() == "Pass"),
            btf(state.previous_play_type.clone() == "RunTackle"),
            (state.tackles) as f64,
            (if (state.time_elapsed) as i32 > FULL_TIME_SECONDS {
                NEAR_END_SECONDS
            } else {
                (state.time_elapsed) as i32
            }) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.ball_location.x
            } else {
                (PLAYING_FIELD_WIDTH - state.ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.ball_location.y
            } else {
                (PLAYING_FIELD_HEIGHT - state.ball_location.y).abs()
            }) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.previous_ball_location.x
            } else {
                (PLAYING_FIELD_WIDTH - state.previous_ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.previous_ball_location.y
            } else {
                (PLAYING_FIELD_HEIGHT - state.previous_ball_location.y).abs()
            }) as f64,
        ],
    );
    execute_events(state);
    let _cse_temp_0 = (model.variable.get(0usize).cloned().unwrap()) as f64;
    let _cse_temp_1 = {
        let multiplier = (10.0_f64).powi(1 as i32);
        (_cse_temp_0 * multiplier).round() / multiplier
    };
    seconds_to_add_initial = _cse_temp_1;
    execute_events(state);
    let _cse_temp_2 = !["RunTry", "KickRetainTry"].contains(&state.current_play_type.as_str());
    if _cse_temp_2 {
        let _cse_temp_3 = f64::max(seconds_to_add_initial, 20.0);
        seconds_to_add_initial = _cse_temp_3;
    }
    execute_events(state);
    time_adjustment = 1.0;
    execute_events(state);
    let _cse_temp_4 = state.period.name.clone() == "FIRST_HALF";
    if _cse_temp_4 {
        time_adjustment = 0.86;
        execute_events(state);
    }
    execute_events(state);
    let _cse_temp_5 = seconds_to_add_initial * time_adjustment;
    seconds_to_add = _cse_temp_5;
    execute_events(state);
    let _cse_temp_6 = (seconds_to_add) as i32;
    let _cse_temp_7 = state.time_elapsed + _cse_temp_6;
    state.time_elapsed = _cse_temp_7;
    execute_events(state);
    execute_events(state);
    return ClockOutputs::new(seconds_to_add);
}
pub fn conversion(state: &mut State) {
    let conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state);
    execute_events(state);
    if conversion_result.scored {
        add_conversion(state);
        execute_events(state);
    }
    execute_events(state);
    swap_possession(state);
    execute_events(state);
    kickoff(state, true, false);
    execute_events(state);
    return;
}
pub fn end_zone(state: &mut State) {
    let mut grid_coordinate: i32 = 0;
    let mut is_end_zone: bool = false;
    let mut is_za_zone: bool = false;
    let mut is_zj_zone: bool = false;
    let mut zone_division: i32 = 0;
    let zone_type: String = "".to_string();
    grid_coordinate = derive_grid_coordinate(
        &state.clone().ball_location.clone(),
        state.clone().team_in_possession.clone().clone(),
    );
    execute_events(state);
    let _cse_temp_0 = grid_coordinate / 6;
    zone_division = _cse_temp_0;
    execute_events(state);
    let _cse_temp_1 = zone_division == 10;
    let _cse_temp_2 = zone_division == 11;
    let _cse_temp_3 = (_cse_temp_1) || (_cse_temp_2);
    is_end_zone = _cse_temp_3;
    is_za_zone = _cse_temp_1;
    is_zj_zone = _cse_temp_2;
    execute_events(state);
    state.is_in_end_zone = is_end_zone;
    state.end_zone_type = if is_za_zone {
        "za".to_string()
    } else {
        if is_zj_zone {
            "zj".to_string()
        } else {
            "none".to_string()
        }
    };
    execute_events(state);
    return;
}
pub fn field_goal(state: &mut State) {
    let mut adjusted_x: i32 = 0;
    let mut is_two_pointer: bool = false;
    let field_goal_result: GetFieldGoalModelResultOutputs = get_field_goal_model_result(state);
    execute_events(state);
    if field_goal_result.scored {
        let _cse_temp_0 = state.clone().team_in_possession.clone() == "Home";
        if _cse_temp_0 {
            adjusted_x = state.clone().ball_location.x;
            execute_events(state);
        } else {
            let _cse_temp_1 = PLAYING_FIELD_WIDTH - state.clone().ball_location.x;
            adjusted_x = _cse_temp_1;
            execute_events(state);
        }
        execute_events(state);
        let _cse_temp_2 = adjusted_x < PLAYING_FIELD_WIDTH - TWO_POINT_FIELD_GOAL_DISTANCE;
        is_two_pointer = _cse_temp_2;
        execute_events(state);
        add_field_goal(state, is_two_pointer);
        execute_events(state);
        kickoff(state, false, false);
        execute_events(state);
    }
    execute_events(state);
    if !field_goal_result.scored {
        swap_possession(state);
        execute_events(state);
        let _cse_temp_3 = state.team_in_possession.clone() == "Home";
        if _cse_temp_3 {
            state.ball_location.y = CENTRE_OF_THE_FIELD_Y;
            state.ball_location.x = MISSED_FIELD_GOAL_RESTART_X;
            execute_events(state);
        } else {
            state.ball_location.x = PLAYING_FIELD_WIDTH - MISSED_FIELD_GOAL_RESTART_X;
            state.ball_location.y = CENTRE_OF_THE_FIELD_Y;
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    return;
}
pub fn field_goal_attempt(state: &State) -> FieldGoalAttemptOutputs {
    let model: FieldGoalAttemptInferOutputs0 = infer::<FieldGoalAttemptInferOutputs0>(
        "field_goal_attempt".to_string(),
        vec![
            (if state.clone().time_elapsed < HALF_LENGTH_IN_SECONDS {
                HALF_LENGTH_IN_SECONDS - state.clone().time_elapsed
            } else {
                if state.clone().time_elapsed < GAME_LENGTH_IN_SECONDS {
                    GAME_LENGTH_IN_SECONDS - state.time_elapsed
                } else {
                    FIELD_GOAL_ATTEMPT_EXTRA_TIME_SECONDS
                }
            }) as f64,
            (if (calculate_margin(state)).abs() < 2 {
                1.0
            } else {
                0.0
            }) as f64,
            (if calculate_margin(state) > 10 {
                1.0
            } else {
                0.0
            }) as f64,
            (std::cmp::max(calculate_dist_to_try_line(state), 0)) as f64,
            ((state.ball_location.y - CENTRE_OF_THE_FIELD_Y).abs()) as f64,
        ],
    );
    execute_events(state);
    return FieldGoalAttemptOutputs::new(sample(&model.probabilities, rand::random::<f64>()));
}
pub fn field_goal_decision(state: &State) -> FieldGoalDecisionOutputs {
    const FIELD_GOAL_MAX_ABSOLUTE: i32 = 900;
    const FIELD_GOAL_MAX_TIME_FIRST: i32 = 2400;
    const FIELD_GOAL_MIN_ABSOLUTE: i32 = 100;
    const FIELD_GOAL_MIN_AWAY: i32 = 400;
    const FIELD_GOAL_MIN_HOME: i32 = 600;
    const FIELD_GOAL_MIN_TIME_FIRST: i32 = 2340;
    const FIELD_GOAL_MIN_TIME_SECOND: i32 = 4200;
    let mut in_bounds: bool = false;
    let mut in_direction: bool = false;
    let mut in_time: bool = false;
    let _cse_temp_0 = state.time_elapsed >= FIELD_GOAL_MIN_TIME_FIRST;
    let _cse_temp_1 = state.time_elapsed <= FIELD_GOAL_MAX_TIME_FIRST;
    let _cse_temp_2 = (_cse_temp_0) && (_cse_temp_1);
    let _cse_temp_3 = state.time_elapsed >= FIELD_GOAL_MIN_TIME_SECOND;
    let _cse_temp_4 = (_cse_temp_2) || (_cse_temp_3);
    in_time = _cse_temp_4;
    let _cse_temp_5 = state.ball_location.x > FIELD_GOAL_MIN_ABSOLUTE;
    let _cse_temp_6 = state.ball_location.x < FIELD_GOAL_MAX_ABSOLUTE;
    let _cse_temp_7 = (_cse_temp_5) && (_cse_temp_6);
    in_bounds = _cse_temp_7;
    let _cse_temp_8 = state.team_in_possession.clone() == "Away";
    let _cse_temp_9 = state.ball_location.x < FIELD_GOAL_MIN_AWAY;
    let _cse_temp_10 = (_cse_temp_8) && (_cse_temp_9);
    let _cse_temp_11 = state.team_in_possession.clone() == "Home";
    let _cse_temp_12 = state.ball_location.x > FIELD_GOAL_MIN_HOME;
    let _cse_temp_13 = (_cse_temp_11) && (_cse_temp_12);
    let _cse_temp_14 = (_cse_temp_10) || (_cse_temp_13);
    in_direction = _cse_temp_14;
    execute_events(state);
    return FieldGoalDecisionOutputs::new(((in_bounds) && (in_time)) && (in_direction));
}
pub fn get_conversion_model_result(state: &State) -> GetConversionModelResultOutputs {
    let mut conversion_k: f64 = 0.0;
    let mut is_extra_time: bool = false;
    let mut player_advantage: i32 = 0;
    let _cse_temp_0 = state.clone().period.name.clone() == "ExtraTime";
    is_extra_time = _cse_temp_0;
    conversion_k = rand::random::<f64>();
    let _cse_temp_1 = (if state.clone().team_in_possession.clone() == "Home" {
        (state.clone().away_sin_bin.clone().len() as i32)
            .saturating_sub(state.clone().home_sin_bin.clone().len() as i32)
    } else {
        (state.home_sin_bin.clone().len() as i32)
            .saturating_sub(state.away_sin_bin.clone().len() as i32)
    }) as i32;
    player_advantage = _cse_temp_1;
    execute_events(state);
    let xy_model: GetConversionModelResultInferOutputs0 =
        infer::<GetConversionModelResultInferOutputs0>(
            "xy".to_string(),
            vec![
                (state.tackles) as f64,
                (if state.team_in_possession.clone() == "Home" {
                    state.ball_location.x
                } else {
                    (PLAYING_FIELD_WIDTH - state.ball_location.x).abs()
                }) as f64,
                (if state.team_in_possession.clone() == "Home" {
                    state.ball_location.y
                } else {
                    (700 - state.ball_location.y).abs()
                }) as f64,
                (state.simulation_invariants.total_points) as f64,
                (get_team_handicap(state)) as f64,
                btf(player_advantage == 1),
                btf(player_advantage > 1),
                btf(player_advantage == -1),
                btf(player_advantage < -1),
                (calculate_margin(state)) as f64,
                btf((state.current_play_type.clone() == "Pass") || (is_extra_time)),
                btf(
                    (["Run", "RunTackle"].contains(&state.current_play_type.as_str()))
                        || ((state.current_play_type.clone() == "RunTry") && (!is_extra_time)),
                ),
                btf((["KickRetain", "KickRetainTackle", "KickTurnover"]
                    .contains(&state.current_play_type.as_str()))
                    || ((state.current_play_type.clone() == "KickRetainTry") && (!is_extra_time))),
                btf((state.current_play_type.clone() == "RunTry")
                    || ((state.current_play_type.clone() == "KickRetainTry") && (!is_extra_time))),
            ],
        );
    execute_events(state);
    let conversion_model: GetConversionModelResultInferOutputs1 =
        infer::<GetConversionModelResultInferOutputs1>(
            "conversion".to_string(),
            vec![
                (compute_conversion_location_from_grid(
                    sample(&xy_model.probabilities, rand::random::<f64>()),
                    conversion_k,
                )) as f64,
                ((CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs()) as f64,
            ],
        );
    execute_events(state);
    return GetConversionModelResultOutputs::new(
        sample(&distribution(conversion_model.probabilities), conversion_k) == 1,
    );
}
pub fn get_field_goal_model_result(state: &State) -> GetFieldGoalModelResultOutputs {
    let model: GetFieldGoalModelResultInferOutputs0 = infer::<GetFieldGoalModelResultInferOutputs0>(
        "field_goal".to_string(),
        vec![
            (calculate_dist_to_try_line(state)) as f64,
            ((CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs()) as f64,
        ],
    );
    execute_events(state);
    return GetFieldGoalModelResultOutputs::new(
        sample(&model.probabilities, rand::random::<f64>()) == 1,
    );
}
pub fn get_kickoff_model_result(state: &State) -> GetKickoffModelResultOutputs {
    let model: GetKickoffModelResultInferOutputs0 = infer::<GetKickoffModelResultInferOutputs0>(
        "kickoff".to_string(),
        vec![
            btf(state.clone().team_in_possession.clone() == "Home"),
            (state.time_elapsed) as f64,
            0.0,
            (if state.team_in_possession.clone() == "Home" {
                (state.home_players.clone().len() as i32)
                    .saturating_sub(state.away_players.clone().len() as i32)
            } else {
                (state.away_players.clone().len() as i32)
                    .saturating_sub(state.home_players.clone().len() as i32)
            }) as f64,
            btf(state.current_play_type.clone() == "LineDropout"),
        ],
    );
    execute_events(state);
    let kickoff_result = {
        let base = &KICKOFF_RESULTS.clone();
        let idx: i32 = (sample(&model.probabilities, rand::random::<f64>())) as i32;
        let actual_idx = if idx < 0 {
            base.len().saturating_sub(idx.abs() as usize)
        } else {
            idx as usize
        };
        base.get(actual_idx).cloned().unwrap()
    };
    execute_events(state);
    return GetKickoffModelResultOutputs::new(
        !KICKOFF_RETAIN_RESULTS
            .clone()
            .contains(&kickoff_result.clone()),
        KICKOFF_TO_GRID_RESULT
            .clone()
            .get(&kickoff_result.to_string())
            .cloned()
            .unwrap(),
    );
}
pub fn interchange(state: &mut State) {
    let mut away_interchange_executed: bool = false;
    let mut away_off_player_index: Option<i32> = None;
    let mut away_on_player_index: Option<i32> = None;
    let mut away_should_interchange: bool = false;
    let mut home_interchange_executed: bool = false;
    let mut home_off_player_index: Option<i32> = None;
    let mut home_on_player_index: Option<i32> = None;
    let mut home_should_interchange: bool = false;
    home_on_player_index = None;
    home_off_player_index = None;
    away_should_interchange = false;
    home_should_interchange = false;
    home_interchange_executed = false;
    away_off_player_index = None;
    away_interchange_executed = false;
    away_on_player_index = None;
    execute_events(state);
    let _cse_temp_0 = state.clone().home_remaining_interchanges > 0;
    if _cse_temp_0 {
        let home_interchange_model: InterchangeInferOutputs0 = infer::<InterchangeInferOutputs0>(
            "interchange".to_string(),
            vec![
                ((state.clone().time_elapsed / 60) as i32) as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "FullBack".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "WingerOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "CentreOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "CentreTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "WingerTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(
                    state,
                    "Home".to_string(),
                    "FiveEighth".to_string(),
                )) as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "HalfBack".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "PropOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "Hooker".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "PropTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(
                    state,
                    "Home".to_string(),
                    "SecondRowOne".to_string(),
                )) as f64,
                (get_time_on_field_for_position(
                    state,
                    "Home".to_string(),
                    "SecondRowTwo".to_string(),
                )) as f64,
                (get_time_on_field_for_position(state, "Home".to_string(), "Lock".to_string()))
                    as f64,
                (get_interchanges_used(state, "Home".to_string())) as f64,
            ],
        );
        execute_events(state);
        let _cse_temp_1 = (home_interchange_model
            .probabilities
            .get(0usize)
            .cloned()
            .unwrap()) as i32;
        let _cse_temp_2 = _cse_temp_1 == 1;
        home_should_interchange = _cse_temp_2;
        execute_events(state);
    }
    execute_events(state);
    if home_should_interchange {
        let home_execution: PlayerInterchangeOutputs =
            player_interchange(state, "Home".to_string());
        execute_events(state);
        if home_execution.executed {
            home_interchange_executed = true;
            home_on_player_index = Some(home_execution.on_player_index);
            home_off_player_index = Some(home_execution.off_player_index);
        }
        execute_events(state);
    }
    execute_events(state);
    if home_interchange_executed {
        decrement_remaining_interchanges(state, "Home".to_string());
    }
    execute_events(state);
    if _cse_temp_0 {
        let away_interchange_model: InterchangeInferOutputs1 = infer::<InterchangeInferOutputs1>(
            "Interchange".to_string(),
            vec![
                ((state.clone().time_elapsed / 60) as i32) as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "FullBack".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "WingerOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "CentreOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "CentreTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "WingerTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(
                    state,
                    "Away".to_string(),
                    "FiveEighth".to_string(),
                )) as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "HalfBack".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "PropOne".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "Hooker".to_string()))
                    as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "PropTwo".to_string()))
                    as f64,
                (get_time_on_field_for_position(
                    state,
                    "Away".to_string(),
                    "SecondRowOne".to_string(),
                )) as f64,
                (get_time_on_field_for_position(
                    state,
                    "Away".to_string(),
                    "SecondRowTwo".to_string(),
                )) as f64,
                (get_time_on_field_for_position(state, "Away".to_string(), "Lock".to_string()))
                    as f64,
                (get_interchanges_used(state, "Away".to_string())) as f64,
            ],
        );
        execute_events(state);
        let _cse_temp_3 = (away_interchange_model
            .probabilities
            .get(0usize)
            .cloned()
            .unwrap()) as i32;
        let _cse_temp_4 = _cse_temp_3 == 1;
        away_should_interchange = _cse_temp_4;
        execute_events(state);
    }
    execute_events(state);
    if away_should_interchange {
        let away_execution: PlayerInterchangeOutputs =
            player_interchange(state, "Away".to_string());
        execute_events(state);
        if away_execution.executed {
            away_off_player_index = Some(away_execution.off_player_index);
            away_on_player_index = Some(away_execution.on_player_index);
            away_interchange_executed = true;
        }
        execute_events(state);
    }
    execute_events(state);
    if away_interchange_executed {
        decrement_remaining_interchanges(state, "Away".to_string());
    }
    execute_events(state);
    let _cse_temp_5 = (home_interchange_executed) || (away_interchange_executed);
    if _cse_temp_5 {
        let last_event = "INTERCHANGE".to_string();
        let away_off_index = away_off_player_index;
        let home_executed = home_interchange_executed;
        let home_off_index = home_off_player_index;
        let away_executed = away_interchange_executed;
        let home_on_index = home_on_player_index;
        let away_on_index = away_on_player_index;
    }
    execute_events(state);
    return;
}
pub fn kickoff(state: &mut State, force_possession_change: bool, is_line_dropout: bool) {
    let mut field_position: FieldPosition = FieldPosition::new(0, 0);
    let mut new_team_in_possession: String = "".to_string();
    let mut valid: bool = false;
    let _cse_temp_0 = state.clone().team_in_possession.clone() == "NotSet";
    if _cse_temp_0 {
        let _cse_temp_1 = rand::random::<f64>() > 0.5;
        if _cse_temp_1 {
            state.team_in_possession = "Away".to_string();
            execute_events(state);
        } else {
            state.team_in_possession = "Home".to_string();
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    new_team_in_possession = state.clone().team_in_possession.clone();
    execute_events(state);
    while true {
        let kickoff: GetKickoffModelResultOutputs = get_kickoff_model_result(state);
        execute_events(state);
        if kickoff.change_possession {
            if new_team_in_possession.clone() == "Home".to_string() {
                new_team_in_possession = "Away".to_string();
                execute_events(state);
            } else {
                new_team_in_possession = "Home".to_string();
                execute_events(state);
            }
            execute_events(state);
        }
        execute_events(state);
        if !is_line_dropout {
            if new_team_in_possession.clone() == "Home".to_string() {
                field_position =
                    convert_field_position(state, kickoff.grid_result.clone(), "Away".to_string());
                execute_events(state);
            } else {
                field_position =
                    convert_field_position(state, kickoff.grid_result.clone(), "Home".to_string());
                execute_events(state);
            }
            execute_events(state);
        } else {
            field_position = convert_field_position(
                state,
                kickoff.grid_result.clone(),
                new_team_in_possession.clone(),
            );
            execute_events(state);
        }
        execute_events(state);
        valid = ((((is_line_dropout)
            && ((field_position.x >= DROPOUT_X) && (field_position.x <= PLAYING_FIELD_WIDTH)))
            || (((!is_line_dropout) && (new_team_in_possession.clone() == "Home".to_string()))
                && (field_position.x <= KICKOFF_X)))
            || (((!is_line_dropout) && (new_team_in_possession.clone() == "Away".to_string()))
                && (field_position.x >= KICKOFF_X)))
            && ((!force_possession_change) || (kickoff.change_possession));
        execute_events(state);
        if valid {
            break;
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    state.ball_location = field_position;
    state.team_in_possession = new_team_in_possession;
    execute_events(state);
    return;
}
pub fn next_play(state: &State) -> NextPlayOutputs {
    let mut distance_to_try_line: i32 = 0;
    let mut player_advantage: i32 = 0;
    let mut possessing_team_margin: f64 = 0.0;
    let _cse_temp_0 = (calculate_margin(state)) as f64;
    possessing_team_margin = _cse_temp_0;
    distance_to_try_line = calculate_dist_to_try_line(state);
    let _cse_temp_1 = (if state.clone().team_in_possession.clone() == "Home" {
        (state.clone().away_sin_bin.clone().len() as i32)
            .saturating_sub(state.clone().home_sin_bin.clone().len() as i32)
    } else {
        (state.home_sin_bin.clone().len() as i32)
            .saturating_sub(state.away_sin_bin.clone().len() as i32)
    }) as i32;
    player_advantage = _cse_temp_1;
    execute_events(state);
    let model: NextPlayInferOutputs0 = infer::<NextPlayInferOutputs0>(
        "next_play".to_string(),
        vec![
            (state.tackles) as f64,
            (distance_to_try_line) as f64,
            (calculate_dist_from_centre(state)) as f64,
            (get_team_handicap(state)) as f64,
            (state.simulation_invariants.total_points) as f64,
            (state.home_match_score + state.away_match_score) as f64,
            (f64::max(f64::min(possessing_team_margin / 2.0, 10.0), -10.0)) as f64,
            btf(player_advantage == 1),
            btf(player_advantage > 1),
            btf(player_advantage == -1),
            btf(player_advantage < -1),
            (if (0 < distance_to_try_line) && (distance_to_try_line <= 10) {
                0.8
            } else {
                if (10 < distance_to_try_line) && (distance_to_try_line <= 20) {
                    0.06
                } else {
                    0.0
                }
            }) as f64,
        ],
    );
    execute_events(state);
    return NextPlayOutputs::new({
        let base = &NEXT_PLAY_TYPES.clone();
        let idx: i32 = (sample(&model.probabilities, rand::random::<f64>())) as i32;
        let actual_idx = if idx < 0 {
            base.len().saturating_sub(idx.abs() as usize)
        } else {
            idx as usize
        };
        base.get(actual_idx).cloned().unwrap()
    });
}
pub fn penalty(state: &mut State) {
    process_penalty(state);
    execute_events(state);
    return;
}
pub fn penalty_type(state: &State) -> PenaltyTypeOutputs {
    let model: PenaltyTypeInferOutputs0 = infer::<PenaltyTypeInferOutputs0>(
        "penalty_type".to_string(),
        vec![
            btf(state.clone().current_play_type.clone() == "WonPenalty"),
            if state.clone().team_in_possession.clone() == "Home" {
                (state.ball_location.x) as f64
            } else {
                ((1000 - state.ball_location.x).abs()) as f64
            },
            if state.team_in_possession.clone() == "Home" {
                (state.ball_location.y) as f64
            } else {
                ((700 - state.ball_location.y).abs()) as f64
            },
            ((700 - state.ball_location.y).abs()) as f64,
            (if state.time_elapsed < 2400 {
                2400 - state.time_elapsed
            } else {
                2 * 2400 - state.time_elapsed
            }) as f64,
            (get_team_handicap(state)) as f64,
            (calculate_margin(state)) as f64,
        ],
    );
    execute_events(state);
    return PenaltyTypeOutputs::new(
        sample(&model.probabilities, rand::random::<f64>()),
        sample(&model.probabilities, rand::random::<f64>()),
    );
}
pub fn player_interchange(state: &State, team: String) -> PlayerInterchangeOutputs {
    let model: PlayerInterchangeInferOutputs0 = infer::<PlayerInterchangeInferOutputs0>(
        "player_interchange".to_string(),
        vec![
            (state.clone().time_elapsed) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "FullBack".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "WingerOne".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "CentreOne".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "CentreTwo".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "WingerTwo".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "FiveEighth".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "HalfBack".to_string(),
            )) as f64,
            (get_time_on_field_for_position(state, team.clone().to_string(), "PropOne".to_string()))
                as f64,
            (get_time_on_field_for_position(state, team.clone().to_string(), "Hooker".to_string()))
                as f64,
            (get_time_on_field_for_position(state, team.clone().to_string(), "PropTwo".to_string()))
                as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "SecondRowOne".to_string(),
            )) as f64,
            (get_time_on_field_for_position(
                state,
                team.clone().to_string(),
                "SecondRowTwo".to_string(),
            )) as f64,
            (get_time_on_field_for_position(state, team.clone().to_string(), "Lock".to_string()))
                as f64,
            (state.set) as f64,
            (get_team_margin(state, team.clone().to_string())) as f64,
        ],
    );
    execute_events(state);
    return PlayerInterchangeOutputs::new(true, -1, -1, team);
}
pub fn player_meters(state: &State) -> PlayerMetersOutputs {
    let model: PlayerMetersInferOutputs0 =
        infer::<PlayerMetersInferOutputs0>("player_meters".to_string(), vec![]);
    execute_events(state);
    return PlayerMetersOutputs::new(sample(&model.probabilities, rand::random::<f64>()));
}
pub fn player_of_the_match(state: &mut State, enabled: bool) -> PlayerOfTheMatchOutputs {
    let mut distribution_sum: f64 = 0.0;
    let mut player_distributions: Vec<f64> = Vec::new();
    let mut pom_player_index: i32 = 0;
    let mut sample_index: i32 = 0;
    if !enabled {
        return PlayerOfTheMatchOutputs::new(state.clone().player_of_the_match);
        execute_events(state);
    }
    execute_events(state);
    player_distributions = calculate_player_of_match_distributions(state);
    execute_events(state);
    let _cse_temp_0 = player_distributions.clone().iter().sum::<f64>();
    let _cse_temp_1 = (_cse_temp_0) as f64;
    distribution_sum = _cse_temp_1;
    execute_events(state);
    let _cse_temp_2 = distribution_sum > 0.0;
    if _cse_temp_2 {
        sample_index = sample_scaled(
            &player_distributions,
            distribution_sum,
            rand::random::<f64>(),
        );
        execute_events(state);
    } else {
        let _cse_temp_3 = player_distributions.clone().len() as i32;
        let _cse_temp_4 = (_cse_temp_3) as f64;
        let _cse_temp_5 = rand::random::<f64>() * _cse_temp_4;
        let _cse_temp_6 = (_cse_temp_5) as i32;
        sample_index = _cse_temp_6;
        execute_events(state);
    }
    execute_events(state);
    let _cse_temp_7 = (sample_index) as i32;
    let _cse_temp_8 = std::cmp::max(_cse_temp_7, 0);
    let _cse_temp_9 = player_distributions.len() as i32;
    let _cse_temp_10 = std::cmp::min(_cse_temp_8, _cse_temp_9 - 1);
    sample_index = _cse_temp_10;
    execute_events(state);
    pom_player_index = state
        .all_players
        .clone()
        .get(sample_index as usize)
        .cloned()
        .unwrap()
        .player_index;
    execute_events(state);
    state.player_of_the_match = pom_player_index;
    execute_events(state);
    return PlayerOfTheMatchOutputs::new(pom_player_index);
}
pub fn player_sin_bin(state: &State, sin_bin_team_is_home: bool) {
    let mut is_home: f64 = 0.0;
    let mut penalty_team_is_home: bool = false;
    let mut sin_bin_player_index: i32 = 0;
    let mut sin_bin_team: String = "".to_string();
    penalty_team_is_home = sin_bin_team_is_home;
    execute_events(state);
    is_home = if penalty_team_is_home { 1.0 } else { 0.0 };
    execute_events(state);
    let model: PlayerSinBinInferOutputs0 = infer::<PlayerSinBinInferOutputs0>(
        "player_sin_bin".to_string(),
        vec![
            is_home,
            (state.clone().time_elapsed) as f64,
            (state.clone().set) as f64,
            (state.clone().tackles) as f64,
            (if penalty_team_is_home {
                state.clone().simulation_invariants.home_price
            } else {
                state.clone().simulation_invariants.away_price
            }) as f64,
            (state.simulation_invariants.total_points) as f64,
            (state.home_match_score + state.away_match_score) as f64,
            (calculate_foul_team_margin(state)) as f64,
        ],
    );
    execute_events(state);
    sin_bin_player_index = sample(&model.probabilities, rand::random::<f64>());
    sin_bin_team = if penalty_team_is_home {
        "Home".to_string()
    } else {
        "Away".to_string()
    };
    execute_events(state);
    let _cse_temp_0 = sin_bin_player_index >= 0;
    if _cse_temp_0 {
        record_player_sin_bin(
            state,
            sin_bin_player_index,
            sin_bin_team.clone(),
            "YellowCard".to_string(),
        );
    }
    execute_events(state);
    if _cse_temp_0 {
        let last_sin_bin_team = sin_bin_team;
        let last_event = "PLAYER_SIN_BIN".to_string();
        let last_sin_bin_player_index = sin_bin_player_index;
    }
    execute_events(state);
    return;
}
pub fn player_tries(state: &State, team: String) -> PlayerTriesOutputs {
    let player_index: i32 = 0;
    let percentage: Vec<f64> = get_team_try_distributions(state);
    execute_events(state);
    let model: PlayerTriesInferOutputs0 = infer::<PlayerTriesInferOutputs0>(
        "player_tries".to_string(),
        vec![
            percentage.clone().get(0usize).cloned().unwrap(),
            percentage.clone().get(1usize).cloned().unwrap(),
            percentage.clone().get(2usize).cloned().unwrap(),
            percentage.clone().get(3usize).cloned().unwrap(),
            percentage.clone().get(4usize).cloned().unwrap(),
            percentage.clone().get(5usize).cloned().unwrap(),
            percentage.clone().get(6usize).cloned().unwrap(),
            percentage.clone().get(7usize).cloned().unwrap(),
            percentage.clone().get(8usize).cloned().unwrap(),
            percentage.clone().get(9usize).cloned().unwrap(),
            percentage.clone().get(10usize).cloned().unwrap(),
            percentage.clone().get(11usize).cloned().unwrap(),
            percentage.get(12usize).cloned().unwrap(),
        ],
    );
    execute_events(state);
    return PlayerTriesOutputs::new(sample(&model.probabilities, rand::random::<f64>()), team);
}
pub fn process_penalty(state: &mut State) {
    let _cse_temp_0 = state.clone().current_play_type.clone() == "ConcededPenalty";
    if _cse_temp_0 {
        swap_possession(state);
    }
    execute_events(state);
    let penalty_outcome_result: PenaltyTypeOutputs = penalty_type(state);
    execute_events(state);
    let _cse_temp_1 = penalty_outcome_result.result_index == 0;
    if _cse_temp_1 {
        let conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state);
        execute_events(state);
        if conversion_result.scored {
            add_penalty(state);
            execute_events(state);
        }
        execute_events(state);
        kickoff(state, false, false);
        execute_events(state);
    }
    execute_events(state);
    let _cse_temp_2 = penalty_outcome_result.result_index == 2;
    if _cse_temp_2 {
        swap_possession(state);
    }
    execute_events(state);
    process_sin_bin_check(state);
    execute_events(state);
    return;
}
pub fn process_sin_bin_check(state: &State) {
    let send_off_result: SinBinOutputs = sin_bin(state);
    execute_events(state);
    if send_off_result.sent_off {
        player_sin_bin(
            state,
            ((state.current_play_type.clone() == "WonPenalty")
                && (state.team_in_possession.clone() == "Home"))
                || ((state.current_play_type.clone() != "WonPenalty")
                    && (state.team_in_possession.clone() == "Away")),
        );
    }
    execute_events(state);
    return;
}
pub fn process_tackle(state: &mut State) {
    const TACKLES_PER_SET: i32 = 6;
    record_tackle(state);
    execute_events(state);
    let _cse_temp_0 = state.clone().tackles + 1;
    state.tackles = _cse_temp_0;
    execute_events(state);
    let _cse_temp_1 = state.tackles > TACKLES_PER_SET;
    if _cse_temp_1 {
        swap_possession(state);
    }
    execute_events(state);
    return;
}
pub fn process_try(state: &mut State) {
    add_try(state);
    execute_events(state);
    if state.include_players {
        let try_selection: PlayerTriesOutputs =
            player_tries(state, state.clone().team_in_possession.clone().clone());
        execute_events(state);
        assign_try(
            state,
            try_selection.player_index,
            try_selection.team.clone(),
        );
        execute_events(state);
    }
    execute_events(state);
    conversion(state);
    execute_events(state);
    return;
}
pub fn sin_bin(state: &State) -> SinBinOutputs {
    let model: SinBinInferOutputs0 = infer::<SinBinInferOutputs0>(
        "sin_bin".to_string(),
        vec![
            (state.clone().ball_location.x) as f64,
            (state.ball_location.y) as f64,
            (calculate_foul_team_margin(state)) as f64,
        ],
    );
    execute_events(state);
    return SinBinOutputs::new(sample(&model.probabilities, rand::random::<f64>()) == 1);
}
pub fn xy(state: &State) -> XyOutputs {
    let mut is_extra_time: bool = false;
    let mut player_advantage: i32 = 0;
    let _cse_temp_0 = (if state.clone().team_in_possession.clone() == "Home" {
        (state.clone().away_sin_bin.clone().len() as i32)
            .saturating_sub(state.clone().home_sin_bin.clone().len() as i32)
    } else {
        (state.clone().home_sin_bin.clone().len() as i32)
            .saturating_sub(state.clone().away_sin_bin.clone().len() as i32)
    }) as i32;
    player_advantage = _cse_temp_0;
    let _cse_temp_1 = state.period.name.clone() == "ExtraTime";
    is_extra_time = _cse_temp_1;
    execute_events(state);
    let model: XyInferOutputs0 = infer::<XyInferOutputs0>(
        "xy".to_string(),
        vec![
            (state.tackles) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.ball_location.x
            } else {
                (1000 - state.ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession.clone() == "Home" {
                state.ball_location.y
            } else {
                (700 - state.ball_location.y).abs()
            }) as f64,
            (state.simulation_invariants.total_points) as f64,
            (get_team_handicap(state)) as f64,
            btf(player_advantage == 1),
            btf(player_advantage > 1),
            btf(player_advantage == -1),
            btf(player_advantage < -1),
            (calculate_margin(state)) as f64,
            btf((state.current_play_type.clone() == "Pass") || (is_extra_time)),
            btf(
                (["Run", "RunTackle"].contains(&state.current_play_type.as_str()))
                    || ((state.current_play_type.clone() == "RunTry") && (!is_extra_time)),
            ),
            btf((["KickRetain", "KickRetainTackle", "KickTurnover"]
                .contains(&state.current_play_type.as_str()))
                || ((state.current_play_type.clone() == "KickRetainTry") && (!is_extra_time))),
            btf((state.current_play_type.clone() == "RunTry")
                || ((state.current_play_type.clone() == "KickRetainTry") && (!is_extra_time))),
        ],
    );
    execute_events(state);
    let grid_result = {
        let base = &GRID_RESULT.clone();
        let idx: i32 = (sample(&model.probabilities, rand::random::<f64>())) as i32;
        let actual_idx = if idx < 0 {
            base.len().saturating_sub(idx.abs() as usize)
        } else {
            idx as usize
        };
        base.get(actual_idx).cloned().unwrap()
    };
    execute_events(state);
    return XyOutputs::new(convert_field_position(
        state,
        grid_result,
        state.team_in_possession.clone(),
    ));
}
#[doc = "Execute all registered event functions."]
pub fn execute_events(state: &State) {
    period_first_last_score_helper(state);
    first_half_output_event(state);
    normal_time_output_event(state);
    player_score_tracking_event(state);
    minute_winner_tracking_event(state);
    debug(state);
    player_void_output_event(state);
    end_of_game_output_event(state);
    first_to_score_tracking_event(state);
    race_to_points_tracking_event(state);
    anytime_output_event(state);
}
#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};
    #[test]
    fn test_softmax_examples() {
        assert_eq!(softmax(vec![]), vec![]);
        assert_eq!(softmax(vec![1]), vec![1]);
    }
    #[test]
    fn quickcheck_normalize() {
        fn prop(values: Vec<f64>) -> TestResult {
            let once = normalize(&values);
            let twice = normalize(once.clone());
            if once != twice {
                return TestResult::failed();
            }
            TestResult::passed()
        }
        quickcheck(prop as fn(Vec<f64>) -> TestResult);
    }
    #[test]
    fn test_normalize_examples() {
        assert_eq!(normalize(vec![]), vec![]);
        assert_eq!(normalize(vec![1]), vec![1]);
    }
    #[test]
    fn test_distribution_examples() {
        assert_eq!(distribution(vec![]), vec![]);
        assert_eq!(distribution(vec![1]), vec![1]);
    }
}
