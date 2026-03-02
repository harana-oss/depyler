use rand as random;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;
use std::f64 as math;
thread_local! {
    static DEPYLER_RNG: std::cell::RefCell<SmallRng>= std::cell::RefCell::new(SmallRng::from_os_rng());

}
use rand::seq::IndexedRandom;
pub const CENTRE_OF_THE_FIELD_X: i32 = 500;
pub const CENTRE_OF_THE_FIELD_Y: i32 = 350;
pub const CONVERSION_POINTS: i32 = 2;
pub static CONVERSION_X_VALUES: [i32; 20] = [
    769, 773, 779, 788, 801, 823, 845, 866, 878, 890, 890, 882, 868, 847, 825, 801, 786, 779, 771,
    769,
];
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
pub static GOAL_LINE_Y_VALUES: [(i32, i32); 7] = [
    (0, 113),
    (113, 226),
    (226, 339),
    (339, 452),
    (452, 565),
    (565, 678),
    (678, 700),
];
pub const GOAL_LINE_Y_WIDTH: f64 = 23.34;
lazy_static::lazy_static! {
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
            0.0017,
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
            0.003044301,
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
            0.0035183,
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
            0.00492562,
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
            0.005074616,
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
            0.008949708,
        ],
    ];
}
lazy_static::lazy_static! {
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
        "ZJ6".to_string(),
    ];
}
pub const HALF_LENGTH_IN_SECONDS: i32 = 2400;
pub const HOOKER_METRES_PER_MINUTE: f64 = 0.75057377;
pub const HOOKER_TACKLES_PER_MINUTE: f64 = 0.547090164;
pub const HOOKER_TRIES_PER_MINUTE: f64 = 0.00232582;
pub const INITIAL_SOLVER_HANDICAP: i32 = -3;
pub const INITIAL_SOLVER_TOTAL_POINTS: i32 = 45;
lazy_static::lazy_static! {
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
        "J6Retain".to_string(),
    ];
}
lazy_static::lazy_static! {
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
        "J6Retain".to_string(),
    ];
}
lazy_static::lazy_static! {
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
}
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
lazy_static::lazy_static! {
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
        "FieldGoalAttempt".to_string(),
    ];
}
pub const NUDGES_PERCENT: f64 = 1.0;
pub const NUMBER_OF_INTERCHANGE_PLAYERS: i32 = 4;
pub const NUMBER_OF_SIMULATIONS_DEFAULT: i32 = 40000;
pub const NUM_PERIODS: i32 = 4;
pub const NUM_PLAYERS_PER_TEAM: i32 = 17;
pub const NUM_STARTING_PLAYERS_PER_TEAM: i32 = 13;
pub const PENALTY_POINTS: i32 = 2;
lazy_static::lazy_static! {
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
        "Lock".to_string(),
    ];
}
lazy_static::lazy_static! {
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
        "Lock".to_string(),
    ];
}
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
pub static X_VALUES: [(i32, i32); 12] = [
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
    (1000, 1100),
];
pub static X_VALUES_AWAY: [(i32, i32); 12] = [
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
    (-100, 1),
];
pub const YELLOW_CARD_SECONDS: i32 = 600;
pub static Y_VALUES: [(i32, i32); 6] = [
    (0, 118),
    (118, 234),
    (234, 351),
    (351, 468),
    (468, 584),
    (584, 700),
];
pub static Y_VALUES_AWAY: [(i32, i32); 6] = [
    (584, 700),
    (468, 584),
    (351, 468),
    (234, 351),
    (118, 234),
    (0, 118),
];
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct FieldPosition {
    pub x: i32,
    pub y: i32,
}
impl FieldPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
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
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerPosition {
    pub position_type: String,
}
impl PlayerPosition {
    pub fn new(position_type: String) -> Self {
        Self { position_type }
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
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
}
#[derive(Debug, Clone, PartialEq, Default, Reflect)]
pub struct ClockInferOutputs0 {
    pub variable: Vec<f64>,
}
impl ClockInferOutputs0 {
    pub fn new(variable: Vec<f64>) -> Self {
        Self { variable }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct ClockOutputs {
    pub seconds_to_add: f64,
}
impl ClockOutputs {
    pub fn new(seconds_to_add: f64) -> Self {
        Self { seconds_to_add }
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct FieldGoalAttemptOutputs {
    pub attempt: i32,
}
impl FieldGoalAttemptOutputs {
    pub fn new(attempt: i32) -> Self {
        Self { attempt }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct FieldGoalDecisionOutputs {
    pub attempt: bool,
}
impl FieldGoalDecisionOutputs {
    pub fn new(attempt: bool) -> Self {
        Self { attempt }
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct GetConversionModelResultOutputs {
    pub scored: bool,
}
impl GetConversionModelResultOutputs {
    pub fn new(scored: bool) -> Self {
        Self { scored }
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct GetFieldGoalModelResultOutputs {
    pub scored: bool,
}
impl GetFieldGoalModelResultOutputs {
    pub fn new(scored: bool) -> Self {
        Self { scored }
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
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NextPlayOutputs {
    pub play_type: String,
}
impl NextPlayOutputs {
    pub fn new(play_type: String) -> Self {
        Self { play_type }
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct PlayerMetersOutputs {
    pub player_index: i32,
}
impl PlayerMetersOutputs {
    pub fn new(player_index: i32) -> Self {
        Self { player_index }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct PlayerOfTheMatchOutputs {
    pub player_of_the_match: i32,
}
impl PlayerOfTheMatchOutputs {
    pub fn new(player_of_the_match: i32) -> Self {
        Self {
            player_of_the_match,
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
}
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct SinBinOutputs {
    pub sent_off: bool,
}
impl SinBinOutputs {
    pub fn new(sent_off: bool) -> Self {
        Self { sent_off }
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
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XyOutputs {
    pub field_position: FieldPosition,
}
impl XyOutputs {
    pub fn new(field_position: FieldPosition) -> Self {
        Self { field_position }
    }
}
pub fn debug(state: &State) {
    let away_running_score: i32 = 0;
    let home_running_score: i32 = 0;
    let race_to_targets: Vec<i32> = Vec::new();
    let remaining_targets: Vec<i32> = Vec::new();
    let target_index: i32 = 0;
    if state.is_over {
        log::info!("{}", "[Debug] Log");
        log::info!(
            "{}",
            format!(
                "State is over, home score = {}, away score = {}",
                state.home_match_score, state.away_match_score
            )
        );
    }
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
    if false {
        log::info!(
            "{}",
            "[Anytime Output Event] Get period and match statistics"
        );
        away_extra_time_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_extra_time_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        log::info!("{}", "[Anytime Output Event] Calculate status flags");
        ended_half_1 = state.period.number > 1;
        ended_match = state.is_over;
        team_a_second_half_points = home_second_half_stats.as_ref().unwrap().scores.total;
        team_b_second_half_points = away_second_half_stats.as_ref().unwrap().scores.total;
        log::info!("{}", "[Anytime Output Event] Output match status");
        record_bool("EndedHalf1".to_string(), ended_half_1);
        record_bool("EndedMatch".to_string(), ended_match);
        log::info!("{}", "[Anytime Output Event] Output second half points");
        record_int("PointsHalf2A".to_string(), team_a_second_half_points);
        record_int("PointsHalf2B".to_string(), team_b_second_half_points);
        log::info!("{}", "[Anytime Output Event] Output match tries");
        record_int(
            "TriesMatchA".to_string(),
            state.home_statistics.total_statistics.scores.tries,
        );
        record_int(
            "TriesMatchB".to_string(),
            state.away_statistics.total_statistics.scores.tries,
        );
        log::info!("{}", "[Anytime Output Event] Output extra time tries");
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
    if state.game_status == "ENDED" {
        log::info!("{}", "[End Of Game Output Event] Get period statistics");
        away_extra_time_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_extra_time_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(EXTRA_TIME_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        log::info!(
            "{}",
            "[End Of Game Output Event] Calculate extra time and combined scores"
        );
        home_extra_time_points = home_extra_time_stats.as_ref().unwrap().scores.total;
        home_second_half_with_et = home_second_half_stats.as_ref().unwrap().scores.total
            + home_extra_time_stats.as_ref().unwrap().scores.total;
        away_second_half_with_et = away_second_half_stats.as_ref().unwrap().scores.total
            + away_extra_time_stats.as_ref().unwrap().scores.total;
        away_extra_time_points = away_extra_time_stats.as_ref().unwrap().scores.total;
        log::info!(
            "{}",
            "[End Of Game Output Event] Determine second half and extra time winner"
        );
        team_a_won_second_half_and_et = home_second_half_with_et > away_second_half_with_et;
        team_b_won_second_half_and_et = home_second_half_with_et < away_second_half_with_et;
        log::info!("{}", "[End Of Game Output Event] Determine match results");
        team_a_won_match = state.home_match_score > state.away_match_score;
        draw_match = state.home_match_score == state.away_match_score;
        let team_b_won_match = state.away_match_score > state.home_match_score;
        log::info!("{}", "[End Of Game Output Event] Output match totals");
        record_int("PointsMatchA".to_string(), state.home_match_score);
        record_int("PointsMatchB".to_string(), state.away_match_score);
        log::info!("{}", "[End Of Game Output Event] Output extra time points");
        record_int("PointsExtraTimeA".to_string(), home_extra_time_points);
        record_int("PointsExtraTimeB".to_string(), away_extra_time_points);
        log::info!(
            "{}",
            "[End Of Game Output Event] Output second half and extra time results"
        );
        record_bool(
            "WinHalf2AndExtraTimeA".to_string(),
            team_a_won_second_half_and_et,
        );
        record_bool(
            "WinHalf2AndExtraTimeB".to_string(),
            team_b_won_second_half_and_et,
        );
        log::info!("{}", "[End Of Game Output Event] Output match results");
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
    if false {
        log::info!(
            "{}",
            "[First Half Output Event] Get first half period statistics"
        );
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        log::info!(
            "{}",
            "[First Half Output Event] Calculate first half scores"
        );
        home_first_half_tries = home_first_half_stats.as_ref().unwrap().scores.tries;
        away_first_half_tries = away_first_half_stats.as_ref().unwrap().scores.tries;
        away_first_half_total = away_first_half_stats.as_ref().unwrap().scores.total;
        home_first_half_total = home_first_half_stats.as_ref().unwrap().scores.total;
        log::info!(
            "{}",
            "[First Half Output Event] Determine first half results"
        );
        team_a_won_first_half = home_first_half_total > away_first_half_total;
        team_b_won_first_half = away_first_half_total > home_first_half_total;
        draw_first_half = home_first_half_total == away_first_half_total;
        log::info!("{}", "[First Half Output Event] Output first half points");
        record_int("PointsHalf1A".to_string(), home_first_half_total);
        record_int("PointsHalf1B".to_string(), away_first_half_total);
        log::info!(
            "{}",
            "[First Half Output Event] Output first half win/draw results"
        );
        record_bool("WinHalf1A".to_string(), team_a_won_first_half);
        record_bool("WinHalf1B".to_string(), team_b_won_first_half);
        record_bool("DrawHalf1".to_string(), draw_first_half);
        log::info!("{}", "[First Half Output Event] Output first half tries");
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
    if false {
        log::info!("{}", "[Normal Time Output Event] Get period statistics");
        away_first_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        away_normal_time_score = 0;
        away_second_half_stats = Some(
            state
                .away_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_normal_time_score = 0;
        home_first_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(FIRST_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        home_second_half_stats = Some(
            state
                .home_statistics
                .period_statistics
                .get(SECOND_HALF_INDEX as usize)
                .cloned()
                .unwrap(),
        );
        log::info!(
            "{}",
            "[Normal Time Output Event] Calculate normal time scores"
        );
        away_normal_time_score = away_first_half_stats.as_ref().unwrap().scores.total
            + away_second_half_stats.as_ref().unwrap().scores.total;
        home_normal_time_score = home_first_half_stats.as_ref().unwrap().scores.total
            + home_second_half_stats.as_ref().unwrap().scores.total;
        log::info!(
            "{}",
            "[Normal Time Output Event] Determine extra time occurrence"
        );
        extra_time_occurred = home_normal_time_score == away_normal_time_score;
        log::info!(
            "{}",
            "[Normal Time Output Event] Output normal time results"
        );
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
        log::info!("{}", "[Normal Time Output Event] Output second half tries");
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
    if true {
        log::info!("{}", "[Player Void Output Event] Bind trader state players");
        home_trader_players = state.home_players.clone();
        away_trader_players = state.away_players.clone();
        log::info!(
            "{}",
            "[Player Void Output Event] Output home player void status"
        );
        for player in home_trader_players.iter().cloned() {
            log::info!("{}", "[Player Void Output Event] record");
            record_bool(
                format!("Player_Voided_{}", player.player_index),
                player.is_voided,
            );
        }
        log::info!(
            "{}",
            "[Player Void Output Event] Output away player void status"
        );
        for player in away_trader_players.iter().cloned() {
            log::info!("{}", "[Player Void Output Event] record");
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
        log::info!(
            "{}",
            "[Period First Last Score Helper] Find first try in period"
        );
        first_try_team = state
            .incidents
            .iter()
            .find(|incident| {
                ((period_filter) && (incident.has_points_confirmed))
                    && (incident.points_confirmed_score_type == "Try")
            })
            .cloned()
            .unwrap()
            .points_scored_team;
        log::info!(
            "{}",
            "[Period First Last Score Helper] Find first score in period"
        );
        first_score_team = state
            .incidents
            .iter()
            .find(|incident| (period_filter) && (incident.has_points_confirmed))
            .cloned()
            .unwrap()
            .points_scored_team;
        log::info!(
            "{}",
            "[Period First Last Score Helper] Calculate first try times"
        );
        away_first_try_time = {
            let a = state
                .incidents
                .iter()
                .find(|incident| {
                    (((period_filter) && (incident.has_points_confirmed))
                        && (incident.points_confirmed_score_type == "Try"))
                        && (incident.points_scored_team == "Away")
                })
                .cloned()
                .unwrap()
                .time_elapsed;
            let b = 60;
            let d = a / b;
            let r = a % b;
            if r != 0 && (a ^ b) < 0 {
                d - 1
            } else {
                d
            }
        } + 1;
        first_try_time = away_first_try_time;
        home_first_try_time = away_first_try_time;
        log::info!(
            "{}",
            "[Period First Last Score Helper] Output first try results"
        );
        record_bool(
            format!("FirstTry{}A", period_name),
            first_try_team == "Home",
        );
        record_bool(
            format!("FirstTry{}B", period_name),
            first_try_team == "Away",
        );
        log::info!(
            "{}",
            "[Period First Last Score Helper] Output first to score results"
        );
        record_bool(
            format!("FirstToScore{}A", period_name),
            first_score_team == "Home",
        );
        record_bool(
            format!("FirstToScore{}B", period_name),
            first_score_team == "Away",
        );
        log::info!(
            "{}",
            "[Period First Last Score Helper] Output first try time results"
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
    let _cse_temp_0 = (state.incidents.len() as i32 > 0) || (state.is_over);
    if _cse_temp_0 {
        log::info!(
            "{}",
            "[First To Score Tracking Event] Collect confirmed scoring incidents"
        );
        points_confirmed_incidents = state
            .incidents
            .iter()
            .cloned()
            .filter(|incident| {
                (incident.has_points_confirmed)
                    && (["Home", "Away"].contains(&*incident.points_scored_team.as_str()))
            })
            .collect::<Vec<_>>();
        log::info!(
            "{}",
            "[First To Score Tracking Event] Collect confirmed try incidents"
        );
        try_incidents = points_confirmed_incidents
            .iter()
            .cloned()
            .filter(|incident| incident.points_confirmed_score_type == "Try")
            .collect::<Vec<_>>();
        log::info!(
            "{}",
            "[First To Score Tracking Event] Resolve key incidents across periods"
        );
        match_first_score_incident = if !points_confirmed_incidents.is_empty() {
            Some(points_confirmed_incidents.get(0usize).cloned().unwrap())
        } else {
            None
        };
        match_first_try_incident = if !try_incidents.is_empty() {
            Some(try_incidents.get(0usize).cloned().unwrap())
        } else {
            None
        };
        half2_et_first_score_incident = points_confirmed_incidents
            .iter()
            .find(|incident| incident.period.number as i32 - 1 >= SECOND_HALF_INDEX)
            .cloned();
        away_first_try_incident = try_incidents
            .iter()
            .find(|incident| incident.points_scored_team == "Away")
            .cloned();
        half2_et_first_try_incident = try_incidents
            .iter()
            .find(|incident| incident.period.number as i32 - 1 >= SECOND_HALF_INDEX)
            .cloned();
        home_first_try_incident = try_incidents
            .iter()
            .find(|incident| incident.points_scored_team == "Home")
            .cloned();
        last_score_incident = if !points_confirmed_incidents.is_empty() {
            Some(points_confirmed_incidents.last().cloned().unwrap())
        } else {
            None
        };
        extra_time_first_try_incident = try_incidents
            .iter()
            .find(|incident| incident.period.number as i32 - 1 == EXTRA_TIME_INDEX)
            .cloned();
        extra_time_first_score_incident = points_confirmed_incidents
            .iter()
            .find(|incident| incident.period.number as i32 - 1 == EXTRA_TIME_INDEX)
            .cloned();
        last_try_incident = if !try_incidents.is_empty() {
            Some(try_incidents.last().cloned().unwrap())
        } else {
            None
        };
        log::info!("{}", "[First To Score Tracking Event] Compute try timings");
        last_try_time = if last_try_incident.is_some() {
            Some(
                ((last_try_incident.as_ref().unwrap().time_elapsed as f64) / (60 as f64)) as i32
                    + 1,
            )
        } else {
            None
        };
        away_first_try_time = if away_first_try_incident.is_some() {
            Some(
                ((away_first_try_incident.as_ref().unwrap().time_elapsed as f64) / (60 as f64))
                    as i32
                    + 1,
            )
        } else {
            None
        };
        home_first_try_time = if home_first_try_incident.is_some() {
            Some(
                ((home_first_try_incident.as_ref().unwrap().time_elapsed as f64) / (60 as f64))
                    as i32
                    + 1,
            )
        } else {
            None
        };
        match_first_try_time = if match_first_try_incident.is_some() {
            Some(
                ((match_first_try_incident.as_ref().unwrap().time_elapsed as f64) / (60 as f64))
                    as i32
                    + 1,
            )
        } else {
            None
        };
    }
    return;
}
pub fn minute_winner_tracking_event(state: &State) {
    let mut final_minute: i32 = 0;
    let mut minute_intervals: Vec<i32> = Vec::new();
    let _cse_temp_0 = (state.incidents.len() as i32 > 0) || (state.is_over);
    if _cse_temp_0 {
        log::info!(
            "{}",
            "[Minute Winner Tracking Event] Initialize interval context"
        );
        minute_intervals = vec![10, 20, 30, 50, 60];
        final_minute = ((state.time_elapsed as f64) / (60 as f64)) as i32;
        log::info!(
            "{}",
            "[Minute Winner Tracking Event] Track winner at each minute interval"
        );
        for minute in minute_intervals.iter().cloned() {
            log::info!(
                "{}",
                "[Minute Winner Tracking Event] Record minute winners when interval elapsed"
            );
            if final_minute > minute {
                log::info!(
                    "{}",
                    "[Minute Winner Tracking Event] Calculate scores through interval"
                );
                let home_points_at_minute = state
                    .incidents
                    .iter()
                    .cloned()
                    .filter(|incident| {
                        ((incident.has_points_confirmed) && (incident.points_scored_team == "Home"))
                            && (incident.time_elapsed <= minute * 60)
                    })
                    .map(|incident| incident.points_scored_points)
                    .sum::<i32>();
                let away_points_at_minute = state
                    .incidents
                    .iter()
                    .cloned()
                    .filter(|incident| {
                        ((incident.has_points_confirmed) && (incident.points_scored_team == "Away"))
                            && (incident.time_elapsed <= minute * 60)
                    })
                    .map(|incident| incident.points_scored_points)
                    .sum::<i32>();
                log::info!(
                    "{}",
                    "[Minute Winner Tracking Event] Output minute winner results"
                );
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
    if state.incidents.len() as i32 > 0 {
        log::info!(
            "{}",
            "[Player Score Tracking Event] Collect try scorer information"
        );
        try_scorer_indices = state
            .incidents
            .iter()
            .cloned()
            .filter(|incident| {
                (incident.has_points_confirmed) && (incident.points_confirmed_score_type == "Try")
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        try_teams = state
            .incidents
            .iter()
            .cloned()
            .filter(|incident| {
                (incident.has_points_confirmed) && (incident.points_confirmed_score_type == "Try")
            })
            .map(|incident| incident.points_scored_team)
            .collect::<Vec<_>>();
        home_try_scorer_indices = state
            .incidents
            .iter()
            .cloned()
            .filter(|incident| {
                ((incident.has_points_confirmed) && (incident.points_confirmed_score_type == "Try"))
                    && (incident.points_scored_team == "Home")
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        away_try_scorer_indices = state
            .incidents
            .iter()
            .cloned()
            .filter(|incident| {
                ((incident.has_points_confirmed) && (incident.points_confirmed_score_type == "Try"))
                    && (incident.points_scored_team == "Away")
            })
            .map(|incident| incident.points_confirmed_player_index)
            .collect::<Vec<_>>();
        log::info!(
            "{}",
            "[Player Score Tracking Event] Find jersey numbers for first and last try scorers"
        );
        first_try_jersey = {
            let base = &state.all_players;
            let idx: i32 = try_scorer_indices.get(0usize).cloned().unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        home_first_try_jersey = {
            let base = &state.home_players;
            let idx: i32 = home_try_scorer_indices.get(0usize).cloned().unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        last_try_jersey = {
            let base = &state.all_players;
            let idx: i32 = {
                let base = &try_scorer_indices;
                let idx: i32 = (try_scorer_indices.len() as i32).saturating_sub(1);
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
        away_first_try_jersey = {
            let base = &state.away_players;
            let idx: i32 = away_try_scorer_indices.get(0usize).cloned().unwrap();
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        }
        .jersey_number;
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output try scorer sequence(up to 15 tries)"
        );
        for index in 1..std::cmp::min(15, try_scorer_indices.len() as i32) {
            log::info!("{}", "[Player Score Tracking Event] record");
            record_int(format!("{}thTryScorer_Match", index), {
                let base = &try_scorer_indices;
                let idx: i32 = index - 1;
                let actual_idx = if idx < 0 {
                    base.len().saturating_sub(idx.abs() as usize)
                } else {
                    idx as usize
                };
                base.get(actual_idx).cloned().unwrap()
            });
        }
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output try team sequence"
        );
        for index in 1..try_teams.len() as i32 {
            log::info!("{}", "[Player Score Tracking Event] record");
            record_bool(
                format!("{}thTry_Match_A", index),
                {
                    let base = &try_teams;
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
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output first try scorer information"
        );
        record_int("FirstTryScorerJerseyMatch".to_string(), first_try_jersey);
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output home team first try scorer"
        );
        record_int(
            "HomeFirstTryScorerMatch".to_string(),
            home_try_scorer_indices.get(0usize).cloned().unwrap(),
        );
        record_int(
            "HomeFirstTryScorerJerseyMatch".to_string(),
            home_first_try_jersey,
        );
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output away team first try scorer"
        );
        record_int(
            "AwayFirstTryScorerMatch".to_string(),
            away_try_scorer_indices.get(0usize).cloned().unwrap(),
        );
        record_int(
            "AwayFirstTryScorerJerseyMatch".to_string(),
            away_first_try_jersey,
        );
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output three unanswered tries"
        );
        record_bool("ThreeUnansweredTries".to_string(), true);
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output last try scorer(end of match only)"
        );
        record_int("LastTryScorerMatch".to_string(), {
            let base = &try_scorer_indices;
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
            let base = &home_try_scorer_indices;
            let idx: i32 = (home_try_scorer_indices.len() as i32).saturating_sub(1);
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        });
        record_int("AwayLastTryScorerMatch".to_string(), {
            let base = &away_try_scorer_indices;
            let idx: i32 = (away_try_scorer_indices.len() as i32).saturating_sub(1);
            let actual_idx = if idx < 0 {
                base.len().saturating_sub(idx.abs() as usize)
            } else {
                idx as usize
            };
            base.get(actual_idx).cloned().unwrap()
        });
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output player statistics from trader state"
        );
        for player in &state.home_players {
            let player = player.clone();
            log::info!("{}", "[Player Score Tracking Event] record");
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
        log::info!(
            "{}",
            "[Player Score Tracking Event] Output away player statistics from trader state"
        );
        for player in &state.away_players {
            let player = player.clone();
            log::info!("{}", "[Player Score Tracking Event] record");
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
    let _cse_temp_0 = (state.incidents.len() as i32 > 0) || (state.is_over);
    if _cse_temp_0 {
        log::info!(
            "{}",
            "[Race To Points Tracking Event] Initialize race to context"
        );
        target_index = 0;
        home_running_score = 0;
        away_running_score = 0;
        race_to_targets = vec![10, 15, 20, 25, 30, 35, 40];
        log::info!(
            "{}",
            "[Race To Points Tracking Event] Process scoring incidents for race targets"
        );
        for incident in &state.incidents {
            let incident = incident.clone();
            log::info!(
                "{}",
                "[Race To Points Tracking Event] Apply confirmed score to running totals"
            );
            if ((incident.has_points_confirmed)
                && (["Home", "Away"].contains(&incident.points_scored_team.as_str())))
                && (target_index < race_to_targets.len() as i32)
            {
                log::info!("{}", "[Race To Points Tracking Event] Update team totals");
                if incident.points_scored_team == "Home" {
                    log::info!("{}", "[Race To Points Tracking Event] Set variables");
                    home_running_score += incident.points_scored_points;
                } else {
                    log::info!("{}", "[Race To Points Tracking Event] Set variables");
                    away_running_score += incident.points_scored_points;
                }
                log::info!(
                    "{}",
                    "[Race To Points Tracking Event] Set current race target"
                );
                let current_target = race_to_targets.get(target_index as usize).cloned().unwrap();
                log::info!(
                    "{}",
                    "[Race To Points Tracking Event] Check target completion"
                );
                let home_reached_target = home_running_score >= current_target;
                let away_reached_target = away_running_score >= current_target;
                log::info!("{}", "[Race To Points Tracking Event] Record race winner when exactly one team reaches target");
                if home_reached_target != away_reached_target {
                    log::info!("{}", "[Race To Points Tracking Event] Record");
                    record_bool(
                        format!("FirstToPoints{}A", current_target),
                        home_reached_target,
                    );
                    record_bool(
                        format!("FirstToPoints{}B", current_target),
                        away_reached_target,
                    );
                    log::info!("{}", "[Race To Points Tracking Event] Set variables");
                    target_index += 1;
                }
            }
        }
        log::info!(
            "{}",
            "[Race To Points Tracking Event] Capture remaining targets as false"
        );
        remaining_targets = {
            let base = &race_to_targets;
            let start = (target_index).max(0) as usize;
            if start < base.len() {
                base[start..].to_vec()
            } else {
                Vec::new()
            }
        };
        log::info!(
            "{}",
            "[Race To Points Tracking Event] Record unresolved race targets"
        );
        for pending_target in remaining_targets.iter().cloned() {
            log::info!("{}", "[Race To Points Tracking Event] Record");
            record_bool(format!("FirstToPoints{}A", pending_target), false);
            record_bool(format!("FirstToPoints{}B", pending_target), false);
        }
    }
    return;
}
#[function]
pub fn add_conversion(state: &mut State) {
    let team = &state.team_in_possession;
    let period_idx = state.period.number - 1;
    log::info!(
        "{} {}",
        ">>>>>>>>>>>>>>>>Adding conversion for team:",
        state.team_in_possession.clone()
    );
    if team == "Home" {
        state.home_match_score += CONVERSION_POINTS;
    } else {
        state.away_match_score += CONVERSION_POINTS;
    }
    let team_stats = if team == "Home" {
        &mut state.home_statistics
    } else {
        &mut state.away_statistics
    };
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .conversions = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .conversions
        + 1;
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + CONVERSION_POINTS;
    team_stats.total_statistics.scores.conversions =
        team_stats.total_statistics.scores.conversions + 1;
    team_stats.total_statistics.scores.total =
        team_stats.total_statistics.scores.total + CONVERSION_POINTS;
    if state.include_players {
        let player = if team == "Home" {
            &mut state.home_player_selected_for_points_market
        } else {
            &mut state.away_player_selected_for_points_market
        };
        player.total_statistics.scores.conversions = player.total_statistics.scores.conversions + 1;
        player.total_statistics.scores.total =
            player.total_statistics.scores.total + CONVERSION_POINTS;
        player
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .conversions = player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .conversions
            + 1;
        player
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .total = player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total
            + CONVERSION_POINTS;
    }
}
#[doc = "Distance from centre of the field from the perspective of the team in possession."]
#[function]
pub fn calculate_dist_from_centre(state: &State) -> i32 {
    if state.team_in_possession == "Home" {
        return (CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs() as i32;
    } else {
        return (CENTRE_OF_THE_FIELD_Y - (PLAYING_FIELD_HEIGHT - state.ball_location.y)).abs()
            as i32;
    }
}
#[doc = "Distance to the try line from the perspective of the team in possession."]
pub fn calculate_dist_to_try_line(state: &State) -> i32 {
    if state.team_in_possession == "Home" {
        return PLAYING_FIELD_WIDTH - state.ball_location.x;
    } else {
        return state.ball_location.x;
    }
}
#[function]
pub fn add_field_goal(state: &mut State, is_two_pointer: bool) {
    let team = &state.team_in_possession;
    let period_idx = state.period.number - 1;
    let points = if is_two_pointer {
        TWO_POINT_FIELD_GOAL_POINTS
    } else {
        FIELD_GOAL_POINTS
    };
    log::info!(
        "{} {}",
        ">>>>>>>>>>>>>>>>Adding field goal for team:",
        state.team_in_possession.clone()
    );
    if team == "Home" {
        state.home_match_score += points;
    } else {
        state.away_match_score += points;
    }
    let team_stats = if team == "Home" {
        &mut state.home_statistics
    } else {
        &mut state.away_statistics
    };
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .field_goals = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .field_goals
        + 1;
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + points;
    team_stats.total_statistics.scores.field_goals =
        team_stats.total_statistics.scores.field_goals + 1;
    team_stats.total_statistics.scores.total = team_stats.total_statistics.scores.total + points;
    let player = if team == "Home" {
        &mut state.home_player_selected_for_points_market
    } else {
        &mut state.away_player_selected_for_points_market
    };
    player
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + points;
    player.total_statistics.scores.total = player.total_statistics.scores.total + points;
}
#[function]
pub fn get_time_on_field_for_position(state: &State, team: &str, position: String) -> f64 {
    let players = _team_players(state, team);
    let candidate = players
        .iter()
        .find(|p| p.position.position_type == position)
        .cloned()
        .expect("StopIteration: iterator is empty");
    return candidate.total_statistics.time_on_field as f64;
}
pub fn get_interchanges_used(state: &State, team: &str) -> f64 {
    let remaining = _get_remaining_interchanges(state, team);
    let used = MAX_INTERCHANGES - remaining;
    return std::cmp::max(0, used) as f64;
}
pub fn get_team_margin(state: &State, team: String) -> f64 {
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    return if team == "Home" {
        (home_score - away_score) as f64
    } else {
        (away_score - home_score) as f64
    };
}
pub fn decrement_remaining_interchanges(state: &mut State, team: &str) {
    let current = _get_remaining_interchanges(state, team);
    let remaining = std::cmp::max(0, current - 1);
    _set_remaining_interchanges(state, team, remaining);
}
pub fn _select_bench_player_index(players: &Vec<Player>, starters: i32) -> i32 {
    let available = (starters..players.len() as i32)
        .filter(|&idx| !players.get(idx as usize).cloned().unwrap().is_injured)
        .collect::<Vec<_>>();
    return DEPYLER_RNG.with(|rng| available.choose(&mut *rng.borrow_mut()).cloned().unwrap());
}
pub fn _swap_team_statistics(state: &State, team: &str, off_slot: i32, bench_slot: i32) {
    let stats = _team_statistic(state, team);
    {
        let (_unpack_tmp0, _unpack_tmp1) = (
            stats
                .player_statistics
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            stats
                .player_statistics
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        stats.player_statistics.clone()[off_slot as usize] = _unpack_tmp0;
        stats.player_statistics[bench_slot as usize] = _unpack_tmp1;
    }
    {
        let (_unpack_tmp0, _unpack_tmp1) = (
            stats
                .sin_bin_players
                .get(bench_slot as usize)
                .cloned()
                .unwrap(),
            stats
                .sin_bin_players
                .get(off_slot as usize)
                .cloned()
                .unwrap(),
        );
        stats.sin_bin_players.clone()[off_slot as usize] = _unpack_tmp0;
        stats.sin_bin_players[bench_slot as usize] = _unpack_tmp1;
    }
}
pub fn _swap_game_statistics(state: &State, team: &str, off_slot: i32, bench_slot: i32) {
    let mirrored = if team == "Home" {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
    let team_stats = _team_statistic(state, team);
    if mirrored == team_stats {
        return;
    }
    {
        let (_unpack_tmp0, _unpack_tmp1) = (
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
        mirrored.player_statistics[off_slot as usize] = _unpack_tmp0;
        mirrored.player_statistics[bench_slot as usize] = _unpack_tmp1;
    }
    {
        let (_unpack_tmp0, _unpack_tmp1) = (
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
        mirrored.sin_bin_players[off_slot as usize] = _unpack_tmp0;
        mirrored.sin_bin_players[bench_slot as usize] = _unpack_tmp1;
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
    return if team == "Home" {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
}
pub fn _team_statistic(state: &State, team: String) -> TeamStatistics {
    return if team == "Home" {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
}
pub fn _get_remaining_interchanges(state: &State, team: String) -> i32 {
    return if team == "Home" {
        state.home_remaining_interchanges
    } else {
        state.away_remaining_interchanges
    };
}
pub fn _set_remaining_interchanges(state: &mut State, team: String, value: i32) {
    if team == "Home" {
        state.home_remaining_interchanges = value;
    } else {
        state.away_remaining_interchanges = value;
    }
}
#[doc = "Margin from the perspective of the penalty-awarded team(matches C# processors)."]
#[function]
pub fn calculate_foul_team_margin(state: &State) -> i32 {
    let mut penalty_team;
    if state.current_play_type == "WonPenalty" {
        penalty_team = state.team_in_possession.clone();
    } else {
        penalty_team = if state.team_in_possession == "Home" {
            "Away".to_string()
        } else {
            "Home".to_string()
        };
    }
    if penalty_team == "Home" {
        return state.home_match_score - state.away_match_score;
    } else {
        return state.away_match_score - state.home_match_score;
    }
}
#[doc = "Margin from the perspective of the team in possession."]
pub fn calculate_margin(state: &State) -> i32 {
    if state.team_in_possession == "Home" {
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
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
    {
        let i = i as i32;
        cumulative += p;
        if k < cumulative {
            return i;
        }
    }
    return (distribution.len() as i32).saturating_sub(1) as i32;
}
#[doc = "Sample from an unnormalized distribution, scaling k by the sum."]
pub fn sample_scaled(distribution: &Vec<f64>, distribution_sum: f64, k: f64) -> i32 {
    let threshold = k * distribution_sum;
    let mut cumulative = 0.0;
    for (i, p) in distribution
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
    {
        let i = i as i32;
        cumulative += p;
        if threshold < cumulative {
            return i;
        }
    }
    return (distribution.len() as i32).saturating_sub(1) as i32;
}
#[doc = "Apply softmax transformation, returning a new list."]
pub fn softmax(logits: Vec<f64>) -> Vec<f64> {
    let max_logit = logits.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let exps = logits
        .iter()
        .cloned()
        .map(|x| (x - max_logit as f64).exp())
        .collect::<Vec<_>>();
    let total = exps.iter().sum::<f64>();
    return exps
        .iter()
        .cloned()
        .map(|e| (e as f64) / (total as f64))
        .collect::<Vec<_>>();
}
#[doc = "Normalize values to sum to 1, returning a new list."]
pub fn normalize(values: Vec<f64>) -> Vec<f64> {
    let total = values.iter().sum::<f64>();
    return values
        .iter()
        .cloned()
        .map(|v| (v as f64) / (total as f64))
        .collect::<Vec<_>>();
}
#[function]
pub fn add_penalty(state: &mut State) {
    let team = &state.team_in_possession;
    let period_idx = state.period.number - 1;
    log::info!(
        "{} {}",
        ">>>>>>>>>>>>>>>>Adding penalty for team:",
        state.team_in_possession.clone()
    );
    if team == "Home" {
        state.home_match_score += PENALTY_POINTS;
    } else {
        state.away_match_score += PENALTY_POINTS;
    }
    let team_stats = if team == "Home" {
        &mut state.home_statistics
    } else {
        &mut state.away_statistics
    };
    team_stats.total_statistics.scores.penalties = team_stats.total_statistics.scores.penalties + 1;
    team_stats.total_statistics.scores.total =
        team_stats.total_statistics.scores.total + PENALTY_POINTS;
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .penalties = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .penalties
        + 1;
    team_stats
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + PENALTY_POINTS;
    let player = if team == "Home" {
        &mut state.home_player_selected_for_points_market
    } else {
        &mut state.away_player_selected_for_points_market
    };
    player.total_statistics.scores.conversions = player.total_statistics.scores.conversions + 1;
    player.total_statistics.scores.total = player.total_statistics.scores.total + PENALTY_POINTS;
    player
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .conversions = player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .conversions
        + 1;
    player
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + PENALTY_POINTS;
}
#[function]
pub fn calculate_player_of_match_distributions(state: &State) -> Vec<f64> {
    let players = &state.all_players;
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    let margin = home_score - away_score;
    let total_points = home_score + away_score;
    return players
        .iter()
        .cloned()
        .map(|player| _calculate_percentage_chance(&player, margin, total_points))
        .collect::<Vec<_>>();
}
pub fn _calculate_percentage_chance(player: &Player, margin: i32, total_points: i32) -> f64 {
    let margin_factor = _calculate_margin_factor(player.delta_strength.clone(), margin);
    let match_stats = &player.total_statistics;
    let match_tries = match_stats.scores.tries;
    let match_score = match_stats.scores.total;
    let pom_percentage = player.player_of_the_match_percentage;
    let team_won = _team_won(player, margin);
    let pom_factor = _calculate_pom_chance(
        &player.tries_strength.clone(),
        match_tries,
        match_score,
        pom_percentage,
        team_won,
    );
    let total_factor = _calculate_total_factor(player.total_strength.clone(), total_points);
    return margin_factor * pom_factor * total_factor + 0.5 * pom_percentage;
}
pub fn _calculate_margin_factor(strength: String, margin: i32) -> f64 {
    let abs_margin = margin.abs();
    if strength == "LOW" {
        return f64::max(2.5 - (abs_margin as f64) / 10.0, 0.5);
    }
    if strength == "HIGH" {
        return f64::min(0.5 * ((abs_margin) as f64) / 10.0, 2.5);
    }
    return 1.0;
}
pub fn _calculate_total_factor(strength: String, total_points: i32) -> f64 {
    if strength == "LOW" {
        return f64::max(
            3.0 - (((0.175 * ((total_points) as f64) / 10.0 as f64).exp()) as f64),
            0.1,
        );
    }
    if strength == "HIGH" {
        return (((0.175 * ((total_points) as f64) / 9.9 as f64).exp()) as f64) - 1.0;
    }
    return 1.0;
}
pub fn _calculate_pom_chance(
    tries_strength: &str,
    match_tries: i32,
    match_score: i32,
    pom_percentage: f64,
    team_won: bool,
) -> f64 {
    let try_factor = _calculate_try_factor(tries_strength, match_tries);
    let win_factor = if team_won { 500 } else { 0 };
    let points_scored = match_score - match_tries * 4;
    let _cse_temp_1 = 0.3357 * ((points_scored) as f64) - 1.75;
    let points_factor = f64::max(1.0, _cse_temp_1);
    let _cse_temp_2 = pom_percentage + pom_percentage * ((win_factor) as f64);
    let pom_weight = (_cse_temp_2 + try_factor) * points_factor;
    return f64::max(pom_weight, MIN_POM_WEIGHT);
}
pub fn _calculate_try_factor(strength: String, tries: i32) -> f64 {
    let coefficient = *{
        let mut map = HashMap::new();
        map.insert("LOW".to_string(), 1.0);
        map.insert("MID".to_string(), 3.0);
        map.insert("HIGH".to_string(), 10.0);
        map
    }
    .get(&strength)
    .unwrap_or(&3.0);
    if tries <= 0 {
        return 0.0;
    }
    return (coefficient * (tries as f64).powf(4.5 as f64) as f64) / 2.0;
}
pub fn _team_won(player: &Player, margin: i32) -> bool {
    let is_home = player.is_home_team;
    if margin == 0 {
        return false;
    }
    return if is_home { margin > 0 } else { margin < 0 };
}
#[function]
pub fn _goal_line_bucket(grid_coordinate: i32, k: f64) -> i32 {
    if grid_coordinate < GOAL_LINE_GRID_START_INDEX {
        return -1;
    }
    let y_band = grid_coordinate % 6;
    let weights = GOAL_LINE_ZONE_WEIGHTS
        .get(y_band as usize)
        .cloned()
        .unwrap();
    return sample(&weights, k);
}
pub fn compute_conversion_location_from_grid(grid_coordinate: i32, k: f64) -> i32 {
    let bucket = _goal_line_bucket(grid_coordinate, k);
    if bucket < 0 {
        let default_band = {
            let a = CONVERSION_X_VALUES.len() as i32;
            let b = 2;
            let d = a / b;
            let r = a % b;
            if r != 0 && (a ^ b) < 0 {
                d - 1
            } else {
                d
            }
        };
        return CONVERSION_X_VALUES[default_band as usize];
    }
    let bucket_row = {
        let d = bucket / 5;
        let r = bucket % 5;
        if r != 0 && (bucket ^ 5) < 0 {
            d - 1
        } else {
            d
        }
    };
    let _cse_temp_0 =
        ((bucket_row) as f64) * GOAL_LINE_Y_WIDTH + (GOAL_LINE_Y_WIDTH / (2 as f64)).floor();
    let base_y = _cse_temp_0 as i32;
    let _cse_temp_1 = grid_coordinate % 6 * GOAL_LINE_Y_HEIGHT_INCREMENT;
    let goal_line_field_y = ((base_y) as f64) + _cse_temp_1;
    let mut conversion_band = (goal_line_field_y / (35 as f64)).floor();
    let _cse_temp_2 = (CONVERSION_X_VALUES.len() as i32).saturating_sub(1);
    conversion_band = f64::max((0) as f64, f64::min(conversion_band, _cse_temp_2));
    let conversion_location = CONVERSION_X_VALUES[conversion_band as usize];
    return conversion_location;
}
pub fn goal_line_position(goal_line_index: i32, grid_index: i32) -> FieldPosition {
    let _cse_temp_0 = goal_line_index % 5 * GOAL_LINE_X_WIDTH;
    let x = PLAYING_FIELD_WIDTH + _cse_temp_0 + {
        let d = GOAL_LINE_X_WIDTH / 2;
        let r = GOAL_LINE_X_WIDTH % 2;
        if r != 0 && (GOAL_LINE_X_WIDTH ^ 2) < 0 {
            d - 1
        } else {
            d
        }
    };
    let _cse_temp_2 = (({
        let d = goal_line_index / 5;
        let r = goal_line_index % 5;
        if r != 0 && (goal_line_index ^ 5) < 0 {
            d - 1
        } else {
            d
        }
    }) as f64)
        * GOAL_LINE_Y_WIDTH;
    let _cse_temp_3 = _cse_temp_2 + GOAL_LINE_Y_WIDTH / (2 as f64);
    let mut y = _cse_temp_3 as i32;
    let _cse_temp_4 = grid_index % 6 * GOAL_LINE_Y_HEIGHT_INCREMENT;
    y += _cse_temp_4;
    return FieldPosition::new(x, y);
}
pub fn convert_field_position(
    state: &State,
    kickoff_result: String,
    team: String,
) -> FieldPosition {
    let grid_result = GRID_RESULT
        .iter()
        .position(|x| {
            x == &KICKOFF_TO_GRID_RESULT
                .get(&kickoff_result)
                .cloned()
                .unwrap()
        })
        .map(|i| i as i32)
        .expect("ValueError: value is not in list");
    let y_position = grid_result % 6;
    let x_position = {
        let d = grid_result / 6;
        let r = grid_result % 6;
        if r != 0 && (grid_result ^ 6) < 0 {
            d - 1
        } else {
            d
        }
    };
    let mut y_range;
    let mut x_range;
    if team == "Home" {
        x_range = X_VALUES[x_position as usize];
        y_range = Y_VALUES[y_position as usize];
    } else {
        x_range = X_VALUES_AWAY[x_position as usize];
        y_range = Y_VALUES_AWAY[y_position as usize];
    }
    return FieldPosition::new(
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(x_range.0..=x_range.1 - 1)),
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(y_range.0..=y_range.1 - 1)),
    );
}
pub fn derive_grid_coordinate(position: &FieldPosition, team: String) -> i32 {
    let mut y_ranges;
    let mut x_ranges;
    if team.contains(&"Away") {
        x_ranges = X_VALUES_AWAY;
        y_ranges = Y_VALUES_AWAY;
    } else {
        x_ranges = X_VALUES;
        y_ranges = Y_VALUES;
    }
    let x_index = _resolve_index(position.x, x_ranges);
    let y_index = _resolve_index(position.y, y_ranges);
    let _cse_temp_0 = (x_index < 0) || (y_index < 0);
    if _cse_temp_0 {
        return -1;
    }
    return x_index * 6 + y_index;
}
pub fn get_goal_line_coordinate(state: &State) -> i32 {
    let grid_coordinate =
        derive_grid_coordinate(&state.ball_location, state.team_in_possession.clone());
    if grid_coordinate < 0 {
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
pub fn record_player_sin_bin(
    state: &mut State,
    player_index: i32,
    team: &str,
    sin_bin_type: String,
) {
    let players = if team == "Home" {
        &state.home_players
    } else {
        &state.away_players
    };
    let mut player = _resolve_player(&players, player_index);
    if let Some(mut player) = player {
        let sin_bin_collection = if team == "Home" {
            &mut state.home_sin_bin
        } else {
            &mut state.away_sin_bin
        };
        if !sin_bin_collection.contains(&player) {
            sin_bin_collection.push(player);
        }
        let seconds_elapsed = state.time_elapsed;
        let duration = if sin_bin_type == "YellowCard" {
            YELLOW_CARD_SECONDS
        } else {
            0
        };
        player.sin_bin_status = sin_bin_type;
        player.return_from_sin_bin_time = seconds_elapsed + duration;
        player.on_field = false;
        player.sin_bin_sent_off = seconds_elapsed;
        rebuild_sin_bin_players(state, team.to_string());
    }
}
pub fn _resolve_player(players: &Vec<Player>, candidate_index: i32) -> Option<Player> {
    for player in players.iter().cloned() {
        if player.player_index == candidate_index {
            return Some(player);
        }
    }
    let target_position = PLAYER_MODEL_POSITIONS
        .get(candidate_index as usize)
        .cloned()
        .unwrap();
    for player in players.iter().cloned() {
        if (player.position.position_type.clone() == target_position) && (player.on_field) {
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
pub fn rebuild_sin_bin_players(state: &mut State, team: String) {
    let team_names = if !team.is_empty() {
        vec![team]
    } else {
        vec!["Home".to_string(), "Away".to_string()]
    };
    for team_name in team_names.iter().cloned() {
        let sin_bin_collection = if team_name == "Home" {
            &state.home_sin_bin
        } else {
            &state.away_sin_bin
        };
        let sin_bin_players = sin_bin_collection
            .iter()
            .cloned()
            .filter(|player| *player.sin_bin_status != "NotSet")
            .collect::<Vec<_>>();
        let team_stats = if team_name == "Home" {
            &mut state.home_statistics
        } else {
            &mut state.away_statistics
        };
        team_stats.sin_bin_players = sin_bin_players;
    }
}
#[function]
pub fn record_tackle(state: &mut State) {
    log::info!("{:?}", state);
    let team = &state.team_in_possession;
    let period_idx = state.period.number - 1;
    let team_stats = if team == "Home" {
        &mut state.home_statistics
    } else {
        &mut state.away_statistics
    };
    team_stats.period_statistics[period_idx as usize].tackles = team_stats
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .tackles
        + 1;
    team_stats.total_statistics.tackles = team_stats.total_statistics.tackles + 1;
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
    if state.team_in_possession == "Home" {
        return state.simulation_invariants.home_handicap;
    }
    return state.simulation_invariants.away_handicap;
}
pub fn swap_possession(state: &mut State) {
    state.set += 1;
    state.team_in_possession = if state.team_in_possession == "Home" {
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
        let players = _get_players(state, team.to_string());
        let mut team_stats = _get_team_stats(state, team.to_string());
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
    log::info!("{:?}", state);
}
pub fn get_team_try_distributions(state: &State) -> Vec<f64> {
    let players = _get_players(state, state.team_in_possession.clone());
    return players
        .iter()
        .cloned()
        .map(|p| p.tries_percentage)
        .collect::<Vec<_>>();
}
pub fn apply_time_on_ground(state: &State, team: &str, seconds: f64) {
    let players = _get_players(state, team.to_string());
    let period_idx = state.period.number - 1;
    for mut player in {
        let base = &players;
        let stop = (NUM_STARTING_PLAYERS_PER_TEAM).max(0) as usize;
        base[..stop.min(base.len())].to_vec()
    } {
        player.period_statistics[period_idx as usize].time_on_field = ((player
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .time_on_field)
            as f64)
            + seconds;
        player.total_statistics.time_on_field =
            ((player.total_statistics.time_on_field) as f64) + seconds;
    }
    for mut s in {
        let base = &_get_team_stats(state, team.to_string()).player_statistics;
        let stop = (NUM_STARTING_PLAYERS_PER_TEAM).max(0) as usize;
        base[..stop.min(base.len())].to_vec()
    } {
        s.time_on_field += seconds;
    }
}
pub fn _get_team_stats(state: &State, team: String) -> TeamStatistics {
    return if team == "Home" {
        state.home_statistics.clone()
    } else {
        state.away_statistics.clone()
    };
}
pub fn _get_players(state: &State, team: String) -> Vec<Player> {
    return if team == "Home" {
        state.home_players.clone()
    } else {
        state.away_players.clone()
    };
}
#[function]
pub fn add_try(state: &mut State) {
    let period_idx = state.period.number - 1;
    log::info!(
        "{} {}",
        ">>>>>>>>>>>>>>>>Adding try for team:",
        state.team_in_possession.clone()
    );
    if state.team_in_possession == "Home" {
        state.home_match_score += TRY_POINTS;
        state
            .home_statistics
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .tries = state
            .home_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries
            + 1;
        state
            .home_statistics
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .total = state
            .home_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total
            + TRY_POINTS;
        state.home_statistics.total_statistics.scores.tries =
            state.home_statistics.total_statistics.scores.tries + 1;
        state.home_statistics.total_statistics.scores.total =
            state.home_statistics.total_statistics.scores.total + TRY_POINTS;
    } else {
        state.away_match_score += TRY_POINTS;
        state
            .away_statistics
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .tries = state
            .away_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .tries
            + 1;
        state
            .away_statistics
            .period_statistics
            .get_mut(period_idx as usize)
            .unwrap()
            .scores
            .total = state
            .away_statistics
            .period_statistics
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .scores
            .total
            + TRY_POINTS;
        state.away_statistics.total_statistics.scores.tries =
            state.away_statistics.total_statistics.scores.tries + 1;
        state.away_statistics.total_statistics.scores.total =
            state.away_statistics.total_statistics.scores.total + TRY_POINTS;
    }
    log::info!("{:?}", state);
}
pub fn assign_try(state: &mut State, player_index: i32, team: String) {
    let period_idx = state.period.number - 1;
    let players = if team == "Home" {
        &state.home_players
    } else {
        &state.away_players
    };
    let (player_list_idx, mut player) = players
        .iter()
        .enumerate()
        .map(|(i, x)| (i as i32, x.clone()))
        .filter(|(i, p)| p.player_index == player_index)
        .next()
        .expect("StopIteration: iterator is empty");
    state
        .period_try_scorers
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .push(player_list_idx);
    state.total_try_scorers.push(player_list_idx);
    if team == "Home" {
        state
            .home_period_try_scorers
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .push(player_list_idx);
        state.home_total_try_scorers.push(player_list_idx);
    } else {
        state
            .away_period_try_scorers
            .get(period_idx as usize)
            .cloned()
            .unwrap()
            .push(player_list_idx);
        state.away_total_try_scorers.push(player_list_idx);
    }
    player
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .tries = player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .tries
        + 1;
    player
        .period_statistics
        .get_mut(period_idx as usize)
        .unwrap()
        .scores
        .total = player
        .period_statistics
        .get(period_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + TRY_POINTS;
    player.total_statistics.scores.tries = player.total_statistics.scores.tries + 1;
    player.total_statistics.scores.total = player.total_statistics.scores.total + TRY_POINTS;
    let team_stats = if team == "Home" {
        &mut state.home_statistics
    } else {
        &mut state.away_statistics
    };
    team_stats
        .player_statistics
        .get_mut(player_list_idx as usize)
        .unwrap()
        .scores
        .tries = team_stats
        .player_statistics
        .get(player_list_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .tries
        + 1;
    team_stats
        .player_statistics
        .get_mut(player_list_idx as usize)
        .unwrap()
        .scores
        .total = team_stats
        .player_statistics
        .get(player_list_idx as usize)
        .cloned()
        .unwrap()
        .scores
        .total
        + TRY_POINTS;
}
pub fn nrl(state: &mut State) {
    let mut current_minute: i32 = 0;
    let mut field_goal_attempt_result: i32 = 0;
    let mut minute_delta: i32 = 0;
    let mut previous_minute: i32 = 0;
    log::info!("{}", "[NRL] Setup Statistics");
    setup_statistics(state);
    execute_events(state);
    log::info!("{}", "[NRL] Setup State");
    state.include_players = false;
    state.period.name = "FirstHalf".to_string();
    state.period.number = 1;
    execute_events(state);
    log::info!("{}", "[NRL] Log State");
    log_state(state);
    execute_events(state);
    log::info!("{}", "[NRL] Game Loop");
    while true {
        log::info!("{}", "[NRL] Store previous state");
        state.previous_ball_location = state.ball_location;
        state.previous_play_type = state.current_play_type.clone();
        execute_events(state);
        log::info!("{}", "[NRL] Evaluate field goal");
        let field_goal_decision: FieldGoalDecisionOutputs = field_goal_decision(state);
        execute_events(state);
        log::info!(
            "{}",
            "[NRL] Run field goal attempt model only if gating passes"
        );
        if field_goal_decision.attempt {
            log::info!("{}", "[NRL] Run field_goal_attempt");
            let field_goal_attempt_outcome: FieldGoalAttemptOutputs = field_goal_attempt(state);
            execute_events(state);
            log::info!("{}", "[NRL] Set variables");
            field_goal_attempt_result = field_goal_attempt_outcome.attempt as i32;
            execute_events(state);
        } else {
            log::info!("{}", "[NRL] Set variables");
            field_goal_attempt_result = 0;
            execute_events(state);
        }
        execute_events(state);
        log::info!("{}", "[NRL] Branch on field goal attempt vs normal play");
        if field_goal_attempt_result == 1 {
            log::info!("{}", "[NRL] Run field_goal");
            field_goal(state);
            execute_events(state);
        } else {
            log::info!("{}", "[NRL] Get next play type");
            let next_play_result: NextPlayOutputs = next_play(state);
            execute_events(state);
            log::info!("{}", "[NRL] Set current play type");
            state.current_play_type = next_play_result.play_type.clone().to_string();
            execute_events(state);
            log::info!("{}", "[NRL] Line Dropout");
            if state.current_play_type == "LineDropout" {
                log::info!("{}", "[NRL] Run swap_possession");
                swap_possession(state);
                execute_events(state);
                log::info!("{}", "[NRL] Run kickoff");
                kickoff(state, true, true);
                execute_events(state);
            } else {
                log::info!("{}", "[NRL] Get XY model result");
                let xy_result: XyOutputs = xy(state);
                execute_events(state);
                log::info!("{}", "[NRL] Set ball location");
                state.ball_location = xy_result.field_position;
                execute_events(state);
                log::info!("{}", "[NRL] Process play type");
                if state.current_play_type == "KickTurnover" {
                    log::info!("{}", "[NRL] Run swap_possession");
                    swap_possession(state);
                    execute_events(state);
                } else {
                    if state.current_play_type == "Run" {
                    } else {
                        if state.current_play_type == "ConcededPenalty" {
                            log::info!("{}", "[NRL] Run process_penalty");
                            process_penalty(state);
                            execute_events(state);
                        } else {
                            if state.current_play_type == "ErrorAttack" {
                                log::info!("{}", "[NRL] Run swap_possession");
                                swap_possession(state);
                                execute_events(state);
                            } else {
                                if state.current_play_type == "ErrorDefence" {
                                    log::info!("{}", "[NRL] Set variables");
                                    state.tackles = STARTING_TACKLE;
                                    execute_events(state);
                                } else {
                                    if state.current_play_type == "RunTry" {
                                        log::info!("{}", "[NRL] Run process_try");
                                        process_try(state);
                                        execute_events(state);
                                    } else {
                                        if state.current_play_type == "WonPenalty" {
                                            log::info!("{}", "[NRL] Run process_penalty");
                                            process_penalty(state);
                                            execute_events(state);
                                        } else {
                                            if state.current_play_type == "KickRetain" {
                                            } else {
                                                if state.current_play_type == "KickRetainTry" {
                                                    log::info!("{}", "[NRL] Run process_try");
                                                    process_try(state);
                                                    execute_events(state);
                                                } else {
                                                    if state.current_play_type == "KickRetainTackle"
                                                    {
                                                        log::info!(
                                                            "{}",
                                                            "[NRL] Run process_tackle"
                                                        );
                                                        process_tackle(state);
                                                        execute_events(state);
                                                    } else {
                                                        if state.current_play_type == "RunTackle" {
                                                            log::info!(
                                                                "{}",
                                                                "[NRL] Run process_tackle"
                                                            );
                                                            process_tackle(state);
                                                            execute_events(state);
                                                        } else {
                                                            if state.current_play_type == "Pass" {}
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
        log::info!("{}", "[NRL] Snapshot elapsed minute before clock");
        previous_minute = ((state.time_elapsed as f64) / (60 as f64)) as i32;
        execute_events(state);
        log::info!("{}", "[NRL] Advance clock");
        clock(state);
        execute_events(state);
        log::info!("{}", "[NRL] Compute elapsed minute after clock");
        current_minute = ((state.time_elapsed as f64) / (60 as f64)) as i32;
        execute_events(state);
        log::info!("{}", "[NRL] Compute minute delta");
        minute_delta = current_minute - previous_minute;
        execute_events(state);
        log::info!("{}", "[NRL] Process interchanges when minute advances");
        if (state.include_players) && (minute_delta > 0) {
            log::info!("{}", "[NRL] Run apply_time_on_ground");
            apply_time_on_ground(state, "Home".to_string(), (minute_delta * 60) as f64);
            execute_events(state);
            log::info!("{}", "[NRL] Run apply_time_on_ground");
            apply_time_on_ground(state, "Away".to_string(), (minute_delta * 60) as f64);
            execute_events(state);
            log::info!("{}", "[NRL] Run interchange");
            interchange(state);
            execute_events(state);
        }
        execute_events(state);
        log::info!("{}", "[NRL] Check game status and period transitions");
        check_game_status(state);
        execute_events(state);
        log::info!("{}", "[NRL] Stop if complete");
        if state.is_over {
            log::info!("{}", "[NRL] Exit");
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
    log::info!(
        "{}",
        "[CheckGameStatus] Check if normal time half is complete"
    );
    let _cse_temp_0 =
        (state.time_elapsed > HALF_LENGTH_IN_SECONDS) && (state.period.name != "ExtraTime");
    if _cse_temp_0 {
        log::info!("{}", "[CheckGameStatus] Handle end of first half");
        if state.period.name == "FirstHalf" {
            log::info!("{}", "[CheckGameStatus] Transition to second half");
            state.game_status = "Period2".to_string();
            state.period.number = 2;
            state.period.name = "SecondHalf".to_string();
            execute_events(state);
        } else {
            log::info!("{}", "[CheckGameStatus] Check if normal time is complete");
            if state.time_elapsed >= GAME_LENGTH_IN_SECONDS {
                log::info!("{}", "[CheckGameStatus] Check for extra time or game end");
                if state.home_match_score == state.away_match_score {
                    log::info!("{}", "[CheckGameStatus] Enter extra time");
                    state.period.name = "ExtraTime".to_string();
                    state.game_status = "ExtraTime".to_string();
                    state.period.number = 3;
                    state.is_extratime = true;
                    execute_events(state);
                } else {
                    log::info!("{}", "[CheckGameStatus] End game");
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
    log::info!("{}", "[CheckGameStatus] Check extra time conditions");
    if state.period.name == "ExtraTime" {
        log::info!("{}", "[CheckGameStatus] Check if extra time should end");
        let _cse_temp_1 =
            state.time_elapsed >= GAME_LENGTH_IN_SECONDS + EXTRA_TIME_LENGTH_IN_SECONDS;
        let _cse_temp_2 =
            (_cse_temp_1 != 0.0) || (state.home_match_score != state.away_match_score);
        if _cse_temp_2 != 0.0 {
            log::info!("{}", "[CheckGameStatus] End game");
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
    log::info!("{}", "[Clock] Run clock model");
    let model: ClockInferOutputs0 = infer::<ClockInferOutputs0>(
        "clock".to_string(),
        vec![
            btf(state.current_play_type == "Pass"),
            btf(state.current_play_type == "RunTackle"),
            btf(state.current_play_type == "KickTurnover"),
            btf(state.current_play_type == "ErrorAttack"),
            btf(state.current_play_type == "ErrorDefence"),
            btf(state.current_play_type == "RunTry"),
            btf(state.current_play_type == "ConcededPenalty"),
            btf(state.current_play_type == "WonPenalty"),
            btf(state.current_play_type == "LineDropout"),
            btf(state.current_play_type == "KickRetainTackle"),
            btf(state.current_play_type == "KickRetainTry"),
            btf(state.current_play_type == "KickRetain"),
            btf(state.previous_play_type == "Pass"),
            btf(state.previous_play_type == "RunTackle"),
            state.tackles as f64,
            (if state.time_elapsed as i32 > FULL_TIME_SECONDS {
                NEAR_END_SECONDS
            } else {
                state.time_elapsed as i32
            }) as f64,
            (if state.team_in_possession == "Home" {
                state.ball_location.x
            } else {
                (PLAYING_FIELD_WIDTH - state.ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession == "Home" {
                state.ball_location.y
            } else {
                (PLAYING_FIELD_HEIGHT - state.ball_location.y).abs()
            }) as f64,
            (if state.team_in_possession == "Home" {
                state.previous_ball_location.x
            } else {
                (PLAYING_FIELD_WIDTH - state.previous_ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession == "Home" {
                state.previous_ball_location.y
            } else {
                (PLAYING_FIELD_HEIGHT - state.previous_ball_location.y).abs()
            }) as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[Clock] Round to 1 decimal place");
    seconds_to_add_initial = {
        let multiplier = (10.0_f64).powi(1 as i32);
        (model.variable.get(0usize).cloned().unwrap() as f64 * multiplier).round() / multiplier
    };
    execute_events(state);
    log::info!("{}", "[Clock] Cap non-try increments at 20 seconds");
    if !["RunTry", "KickRetainTry"].contains(&state.current_play_type.as_str()) {
        seconds_to_add_initial = f64::max(seconds_to_add_initial, 20.0);
    }
    execute_events(state);
    log::info!("{}", "[Clock] Default time adjustment");
    time_adjustment = 1.0;
    execute_events(state);
    log::info!("{}", "[Clock] Apply period-specific time adjustment");
    if state.period.name == "FIRST_HALF" {
        log::info!("{}", "[Clock] Set variables");
        time_adjustment = 0.86;
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Clock] Calculate final seconds to add");
    seconds_to_add = seconds_to_add_initial * time_adjustment;
    execute_events(state);
    log::info!("{}", "[Clock] Update game state");
    let _cse_temp_0 = state.time_elapsed + seconds_to_add as i32;
    state.time_elapsed = _cse_temp_0;
    execute_events(state);
    log::info!("{}", "[Clock] Log");
    log::info!("{}", format!("current_play_type: {}, seconds_to_add: {}, seconds_to_add_initial: {}, adjustment: {}, elapsed: {}.", state.current_play_type.clone(), seconds_to_add, seconds_to_add_initial, time_adjustment, state.time_elapsed));
    execute_events(state);
    log::info!("{}", "[Clock] Return seconds added");
    return ClockOutputs::new(seconds_to_add);
}
pub fn conversion(state: &mut State) {
    log::info!("{}", "[Conversion] Run conversion model");
    let conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state);
    execute_events(state);
    log::info!("{}", "[Conversion] Add conversion points if scored");
    if conversion_result.scored {
        log::info!("{}", "[Conversion] Run add_conversion");
        add_conversion(state);
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Conversion] Swap possession");
    swap_possession(state);
    execute_events(state);
    log::info!(
        "{}",
        "[Conversion] Process kickoff with force possession change"
    );
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
    log::info!("{}", "[EndZone] Derive grid coordinate from ball location");
    grid_coordinate = derive_grid_coordinate(
        &state.ball_location,
        state.team_in_possession.clone().clone(),
    );
    execute_events(state);
    log::info!(
        "{}",
        "[EndZone] Calculate end zone classification from grid coordinate"
    );
    zone_division = (grid_coordinate as f64) / (6 as f64);
    execute_events(state);
    log::info!("{}", "[EndZone] Calculate zone type booleans");
    is_zj_zone = zone_division == 11;
    is_end_zone = (zone_division == 10) || (zone_division == 11);
    is_za_zone = zone_division == 10;
    execute_events(state);
    log::info!("{}", "[EndZone] Update game state");
    state.end_zone_type = if is_za_zone {
        "za".to_string()
    } else {
        if is_zj_zone {
            "zj".to_string()
        } else {
            "none".to_string()
        }
    };
    state.is_in_end_zone = is_end_zone;
    execute_events(state);
    return;
}
pub fn field_goal(state: &mut State) {
    let mut adjusted_x: i32 = 0;
    let mut is_two_pointer: bool = false;
    log::info!("{}", "[FieldGoal] Run field goal model");
    let field_goal_result: GetFieldGoalModelResultOutputs = get_field_goal_model_result(state);
    execute_events(state);
    log::info!("{}", "[FieldGoal] Handle scored field goal");
    if field_goal_result.scored {
        log::info!("{}", "[FieldGoal] Compute adjusted x for two-pointer check");
        if state.team_in_possession == "Home" {
            log::info!("{}", "[FieldGoal] Set variables");
            adjusted_x = state.ball_location.x;
            execute_events(state);
        } else {
            log::info!("{}", "[FieldGoal] Set variables");
            adjusted_x = PLAYING_FIELD_WIDTH - state.ball_location.x;
            execute_events(state);
        }
        execute_events(state);
        log::info!("{}", "[FieldGoal] Determine if two-pointer");
        is_two_pointer = adjusted_x < PLAYING_FIELD_WIDTH - TWO_POINT_FIELD_GOAL_DISTANCE;
        execute_events(state);
        log::info!("{}", "[FieldGoal] Add field goal points");
        add_field_goal(state, is_two_pointer);
        execute_events(state);
        log::info!("{}", "[FieldGoal] Process kickoff");
        kickoff(state, false, false);
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[FieldGoal] Handle missed field goal");
    if !field_goal_result.scored {
        log::info!("{}", "[FieldGoal] Swap possession");
        swap_possession(state);
        execute_events(state);
        log::info!("{}", "[FieldGoal] Set ball location for missed field goal");
        if state.team_in_possession == "Home" {
            log::info!("{}", "[FieldGoal] Set variables");
            state.ball_location.y = CENTRE_OF_THE_FIELD_Y;
            state.ball_location.x = MISSED_FIELD_GOAL_RESTART_X;
            execute_events(state);
        } else {
            log::info!("{}", "[FieldGoal] Set variables");
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
    log::info!("{}", "[FieldGoalAttempt] Run field goal attempt model");
    let model: FieldGoalAttemptInferOutputs0 = infer::<FieldGoalAttemptInferOutputs0>(
        "field_goal_attempt".to_string(),
        vec![
            (if state.time_elapsed < HALF_LENGTH_IN_SECONDS {
                HALF_LENGTH_IN_SECONDS - state.time_elapsed
            } else {
                if state.time_elapsed < GAME_LENGTH_IN_SECONDS {
                    GAME_LENGTH_IN_SECONDS - state.time_elapsed
                } else {
                    FIELD_GOAL_ATTEMPT_EXTRA_TIME_SECONDS
                }
            }) as f64,
            if (calculate_margin(state)).abs() < 2 {
                1.0
            } else {
                0.0
            },
            if calculate_margin(state) > 10 {
                1.0
            } else {
                0.0
            },
            std::cmp::max(calculate_dist_to_try_line(state), 0) as f64,
            (state.ball_location.y - CENTRE_OF_THE_FIELD_Y).abs() as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[FieldGoalAttempt] Return result");
    return FieldGoalAttemptOutputs::new(sample(
        &model.probabilities,
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
    ));
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
    log::info!("{}", "[FieldGoalDecision] Evaluate spatial bounds");
    in_bounds = (state.ball_location.x > FIELD_GOAL_MIN_ABSOLUTE)
        && (state.ball_location.x < FIELD_GOAL_MAX_ABSOLUTE);
    let _cse_temp_1 =
        (state.team_in_possession == "Away") && (state.ball_location.x < FIELD_GOAL_MIN_AWAY);
    let _cse_temp_2 =
        (state.team_in_possession == "Home") && (state.ball_location.x > FIELD_GOAL_MIN_HOME);
    in_direction = (_cse_temp_1 != 0.0) || (_cse_temp_2 != 0.0);
    let _cse_temp_3 = (state.time_elapsed >= FIELD_GOAL_MIN_TIME_FIRST)
        && (state.time_elapsed <= FIELD_GOAL_MAX_TIME_FIRST);
    in_time = (_cse_temp_3 != 0.0) || (state.time_elapsed >= FIELD_GOAL_MIN_TIME_SECOND);
    execute_events(state);
    log::info!("{}", "[FieldGoalDecision] Return decision");
    return FieldGoalDecisionOutputs::new(((in_bounds) && (in_time)) && (in_direction));
}
pub fn get_conversion_model_result(state: &State) -> GetConversionModelResultOutputs {
    let mut conversion_k: f64 = 0.0;
    let mut is_extra_time: bool = false;
    let mut player_advantage: i32 = 0;
    log::info!("{}", "[GetConversionModelResult] Set variables");
    is_extra_time = state.period.name == "ExtraTime";
    player_advantage = (if state.team_in_possession == "Home" {
        (state.away_sin_bin.len() as i32).saturating_sub(state.home_sin_bin.len() as i32)
    } else {
        (state.home_sin_bin.len() as i32).saturating_sub(state.away_sin_bin.len() as i32)
    }) as i32;
    conversion_k = DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>());
    execute_events(state);
    log::info!(
        "{}",
        "[GetConversionModelResult] Run xy model to determine goal line grid"
    );
    let xy_model: GetConversionModelResultInferOutputs0 =
        infer::<GetConversionModelResultInferOutputs0>(
            "xy".to_string(),
            vec![
                state.tackles as f64,
                (if state.team_in_possession == "Home" {
                    state.ball_location.x
                } else {
                    (PLAYING_FIELD_WIDTH - state.ball_location.x).abs()
                }) as f64,
                (if state.team_in_possession == "Home" {
                    state.ball_location.y
                } else {
                    (700 - state.ball_location.y).abs()
                }) as f64,
                state.simulation_invariants.total_points as f64,
                get_team_handicap(state),
                btf(player_advantage == 1),
                btf(player_advantage > 1),
                btf(player_advantage == -1),
                btf(player_advantage < -1),
                calculate_margin(state) as f64,
                btf((state.current_play_type == "Pass") || (is_extra_time)),
                btf(
                    (["Run", "RunTackle"].contains(&state.current_play_type.as_str()))
                        || ((state.current_play_type == "RunTry") && (!is_extra_time)),
                ),
                btf((["KickRetain", "KickRetainTackle", "KickTurnover"]
                    .contains(&state.current_play_type.as_str()))
                    || ((state.current_play_type == "KickRetainTry") && (!is_extra_time))),
                btf((state.current_play_type == "RunTry")
                    || ((state.current_play_type == "KickRetainTry") && (!is_extra_time))),
            ],
        );
    execute_events(state);
    log::info!("{}", "[GetConversionModelResult] Run conversion model");
    let conversion_model: GetConversionModelResultInferOutputs1 =
        infer::<GetConversionModelResultInferOutputs1>(
            "conversion".to_string(),
            vec![
                compute_conversion_location_from_grid(
                    sample(
                        &xy_model.probabilities,
                        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
                    ),
                    conversion_k,
                ) as f64,
                (CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs() as f64,
            ],
        );
    execute_events(state);
    log::info!("{}", "[GetConversionModelResult] Return result");
    return GetConversionModelResultOutputs::new(
        sample(&conversion_model.probabilities, conversion_k) == 1,
    );
}
pub fn get_field_goal_model_result(state: &State) -> GetFieldGoalModelResultOutputs {
    log::info!("{}", "[GetFieldGoalModelResult] Run field goal model");
    let model: GetFieldGoalModelResultInferOutputs0 = infer::<GetFieldGoalModelResultInferOutputs0>(
        "field_goal".to_string(),
        vec![
            calculate_dist_to_try_line(state) as f64,
            (CENTRE_OF_THE_FIELD_Y - state.ball_location.y).abs() as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[GetFieldGoalModelResult] Return result");
    return GetFieldGoalModelResultOutputs::new(
        sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ) == 1,
    );
}
pub fn get_kickoff_model_result(state: &State) -> GetKickoffModelResultOutputs {
    log::info!("{}", "[GetKickoffModelResult] Run kickoff model");
    let model: GetKickoffModelResultInferOutputs0 = infer::<GetKickoffModelResultInferOutputs0>(
        "kickoff".to_string(),
        vec![
            btf(state.team_in_possession == "Home"),
            state.time_elapsed as f64,
            0.0,
            (if state.team_in_possession == "Home" {
                (state.home_players.len() as i32).saturating_sub(state.away_players.len() as i32)
            } else {
                (state.away_players.len() as i32).saturating_sub(state.home_players.len() as i32)
            }) as f64,
            btf(state.current_play_type == "LineDropout"),
        ],
    );
    execute_events(state);
    log::info!("{}", "[GetKickoffModelResult] Get kickoff result");
    let kickoff_result = KICKOFF_RESULTS
        .get(sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ) as i32 as usize)
        .cloned()
        .unwrap();
    execute_events(state);
    log::info!("{}", "[GetKickoffModelResult] Return result");
    return GetKickoffModelResultOutputs::new(
        !KICKOFF_RETAIN_RESULTS.clone().contains(&kickoff_result),
        kickoff_result.to_string(),
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
    log::info!("{}", "[Interchange] Reset interchange context");
    home_interchange_executed = false;
    home_should_interchange = false;
    away_interchange_executed = false;
    away_off_player_index = None;
    away_on_player_index = None;
    home_off_player_index = None;
    away_should_interchange = false;
    home_on_player_index = None;
    execute_events(state);
    log::info!("{}", "[Interchange] Sample home interchange decision");
    if state.home_remaining_interchanges > 0 {
        log::info!("{}", "[Interchange] Infer home interchange model");
        let home_interchange_model: InterchangeInferOutputs0 = infer::<InterchangeInferOutputs0>(
            "interchange".to_string(),
            vec![
                (((state.time_elapsed as f64) / (60 as f64)) as i32) as f64,
                get_time_on_field_for_position(state, "Home".to_string(), "FullBack".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "WingerOne".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "CentreOne".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "CentreTwo".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "WingerTwo".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "FiveEighth".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "HalfBack".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "PropOne".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "Hooker".to_string()),
                get_time_on_field_for_position(state, "Home".to_string(), "PropTwo".to_string()),
                get_time_on_field_for_position(
                    state,
                    "Home".to_string(),
                    "SecondRowOne".to_string(),
                ),
                get_time_on_field_for_position(
                    state,
                    "Home".to_string(),
                    "SecondRowTwo".to_string(),
                ),
                get_time_on_field_for_position(state, "Home".to_string(), "Lock".to_string()),
                get_interchanges_used(state, "Home".to_string()),
            ],
        );
        execute_events(state);
        log::info!("{}", "[Interchange] Decide home interchange outcome");
        home_should_interchange = home_interchange_model
            .probabilities
            .get(0usize)
            .cloned()
            .unwrap() as i32
            == 1;
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Sample home player selection");
    if home_should_interchange {
        log::info!("{}", "[Interchange] Infer home player selection");
        let home_execution: PlayerInterchangeOutputs =
            player_interchange(state, home_selection.position_index, "Home".to_string());
        execute_events(state);
        log::info!("{}", "[Interchange] Finalise home interchange");
        if home_execution.executed {
            home_on_player_index = Some(home_execution.on_player_index);
            home_interchange_executed = true;
            home_off_player_index = Some(home_execution.off_player_index);
        }
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Reduce home interchange counter");
    if home_interchange_executed {
        decrement_remaining_interchanges(state, "Home".to_string());
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Sample away interchange decision");
    if state.away_remaining_interchanges > 0 {
        log::info!("{}", "[Interchange] Infer away interchange model");
        let away_interchange_model: InterchangeInferOutputs1 = infer::<InterchangeInferOutputs1>(
            "Interchange".to_string(),
            vec![
                (((state.time_elapsed as f64) / (60 as f64)) as i32) as f64,
                get_time_on_field_for_position(state, "Away".to_string(), "FullBack".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "WingerOne".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "CentreOne".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "CentreTwo".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "WingerTwo".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "FiveEighth".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "HalfBack".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "PropOne".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "Hooker".to_string()),
                get_time_on_field_for_position(state, "Away".to_string(), "PropTwo".to_string()),
                get_time_on_field_for_position(
                    state,
                    "Away".to_string(),
                    "SecondRowOne".to_string(),
                ),
                get_time_on_field_for_position(
                    state,
                    "Away".to_string(),
                    "SecondRowTwo".to_string(),
                ),
                get_time_on_field_for_position(state, "Away".to_string(), "Lock".to_string()),
                get_interchanges_used(state, "Away".to_string()),
            ],
        );
        execute_events(state);
        log::info!("{}", "[Interchange] Decide away interchange outcome");
        away_should_interchange = away_interchange_model
            .probabilities
            .get(0usize)
            .cloned()
            .unwrap() as i32
            == 1;
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Sample away player selection");
    if away_should_interchange {
        log::info!("{}", "[Interchange] Infer away player selection");
        let away_execution: PlayerInterchangeOutputs =
            player_interchange(state, away_selection.position_index, "Away".to_string());
        execute_events(state);
        log::info!("{}", "[Interchange] Finalise away interchange");
        if away_execution.executed {
            away_on_player_index = Some(away_execution.on_player_index);
            away_interchange_executed = true;
            away_off_player_index = Some(away_execution.off_player_index);
        }
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Reduce away interchange counter");
    if away_interchange_executed {
        decrement_remaining_interchanges(state, "Away".to_string());
    }
    execute_events(state);
    log::info!("{}", "[Interchange] Update game state after interchanges");
    if (home_interchange_executed) || (away_interchange_executed) {
        let home_executed = home_interchange_executed;
        let home_off_index = home_off_player_index;
        let home_on_index = home_on_player_index;
        let away_executed = away_interchange_executed;
        let last_event = "INTERCHANGE".to_string();
        let away_off_index = away_off_player_index;
        let away_on_index = away_on_player_index;
    }
    execute_events(state);
    return;
}
pub fn kickoff(state: &mut State, force_possession_change: bool, is_line_dropout: bool) {
    let mut field_position: FieldPosition = FieldPosition::new(0, 0);
    let mut new_team_in_possession: String = "".to_string();
    let mut valid: bool = false;
    log::info!("{}", "[Kickoff] Ensure team in possession is initialised");
    if state.team_in_possession == "NotSet" {
        log::info!("{}", "[Kickoff] If statement");
        if DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()) > 0.5 {
            log::info!("{}", "[Kickoff] Set variables");
            state.team_in_possession = "Away".to_string();
            execute_events(state);
        } else {
            log::info!("{}", "[Kickoff] Set variables");
            state.team_in_possession = "Home".to_string();
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Kickoff] Seed kickoff possession");
    new_team_in_possession = state.team_in_possession.clone();
    execute_events(state);
    log::info!("{}", "[Kickoff] Loop until valid kickoff position is found");
    while true {
        log::info!("{}", "[Kickoff] Run kickoff model");
        let kickoff: GetKickoffModelResultOutputs = get_kickoff_model_result(state);
        execute_events(state);
        log::info!("{}", "[Kickoff] Swap team if possession changed");
        if kickoff.change_possession {
            swap_possession(state);
        }
        execute_events(state);
        log::info!("{}", "[Kickoff] Convert to field position");
        if !is_line_dropout {
            log::info!("{}", "[Kickoff] If statement");
            if state.team_in_possession == "Home" {
                log::info!("{}", "[Kickoff] Set variables");
                field_position =
                    convert_field_position(state, kickoff.grid_result.clone(), "Away".to_string());
                execute_events(state);
            } else {
                log::info!("{}", "[Kickoff] Set variables");
                field_position =
                    convert_field_position(state, kickoff.grid_result.clone(), "Home".to_string());
                execute_events(state);
            }
            execute_events(state);
        } else {
            log::info!("{}", "[Kickoff] Set variables");
            field_position = convert_field_position(
                state,
                kickoff.grid_result.clone(),
                state.team_in_possession.clone(),
            );
            execute_events(state);
        }
        execute_events(state);
        log::info!("{}", "[Kickoff] Validate position constraints");
        valid = ((((is_line_dropout)
            && ((field_position.x >= DROPOUT_X) && (field_position.x <= PLAYING_FIELD_WIDTH)))
            || (((!is_line_dropout) && (new_team_in_possession == "Home"))
                && (field_position.x <= KICKOFF_X)))
            || (((!is_line_dropout) && (new_team_in_possession == "Away"))
                && (field_position.x >= KICKOFF_X)))
            && ((!force_possession_change) || (kickoff.change_possession));
        execute_events(state);
        log::info!("{}", "[Kickoff] Exit loop if valid");
        if valid {
            log::info!("{}", "[Kickoff] Exit");
            break;
            execute_events(state);
        }
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[Kickoff] Update game state");
    state.ball_location = field_position;
    execute_events(state);
    return;
}
pub fn next_play(state: &State) -> NextPlayOutputs {
    let mut distance_to_try_line: i32 = 0;
    let mut player_advantage: i32 = 0;
    let mut possessing_team_margin: f64 = 0.0;
    log::info!("{}", "[NextPlay] Set variables");
    distance_to_try_line = calculate_dist_to_try_line(state);
    player_advantage = (if state.team_in_possession == "Home" {
        (state.away_sin_bin.len() as i32).saturating_sub(state.home_sin_bin.len() as i32)
    } else {
        (state.home_sin_bin.len() as i32).saturating_sub(state.away_sin_bin.len() as i32)
    }) as i32;
    possessing_team_margin = calculate_margin(state) as f64;
    execute_events(state);
    log::info!("{}", "[NextPlay] Run next play model");
    let model: NextPlayInferOutputs0 = infer::<NextPlayInferOutputs0>(
        "next_play".to_string(),
        vec![
            state.tackles as f64,
            distance_to_try_line as f64,
            calculate_dist_from_centre(state) as f64,
            get_team_handicap(state),
            state.simulation_invariants.total_points as f64,
            std::cmp::min(state.home_match_score + state.away_match_score, 100) as f64,
            f64::max(f64::min(possessing_team_margin / 2.0, 10.0), -10.0),
            btf(player_advantage == 1),
            btf(player_advantage > 1),
            btf(player_advantage == -1),
            btf(player_advantage < -1),
            if (0 < distance_to_try_line) && (distance_to_try_line <= 10) {
                0.8
            } else {
                if (10 < distance_to_try_line) && (distance_to_try_line <= 20) {
                    0.06
                } else {
                    0.0
                }
            },
        ],
    );
    execute_events(state);
    log::info!("{}", "[NextPlay] Return next play");
    return NextPlayOutputs::new(
        NEXT_PLAY_TYPES
            .get(sample(
                &model.probabilities,
                DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
            ) as i32 as usize)
            .cloned()
            .unwrap(),
    );
}
pub fn penalty(state: &mut State) {
    log::info!("{}", "[Penalty] Execute penalty flow");
    process_penalty(state);
    execute_events(state);
    return;
}
pub fn penalty_type(state: &State) -> PenaltyTypeOutputs {
    log::info!("{}", "[PenaltyType] Run penalty type model");
    let model: PenaltyTypeInferOutputs0 = infer::<PenaltyTypeInferOutputs0>(
        "penalty_type".to_string(),
        vec![
            btf(state.current_play_type == "WonPenalty"),
            if state.team_in_possession == "Home" {
                state.ball_location.x as f64
            } else {
                (1000 - state.ball_location.x).abs() as f64
            },
            if state.team_in_possession == "Home" {
                state.ball_location.y as f64
            } else {
                (700 - state.ball_location.y).abs() as f64
            },
            (700 - state.ball_location.y).abs() as f64,
            (if state.time_elapsed < 2400 {
                2400 - state.time_elapsed
            } else {
                2 * 2400 - state.time_elapsed
            }) as f64,
            get_team_handicap(state),
            calculate_margin(state) as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[PenaltyType] Return penalty type");
    return PenaltyTypeOutputs::new(
        sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ),
        sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ),
    );
}
pub fn player_interchange(state: &State, team: String) -> PlayerInterchangeOutputs {
    log::info!("{}", "[PlayerInterchange] Run player interchange model");
    let model: PlayerInterchangeInferOutputs0 = infer::<PlayerInterchangeInferOutputs0>(
        "player_interchange".to_string(),
        vec![
            state.time_elapsed as f64,
            get_time_on_field_for_position(state, &team, "FullBack".to_string()),
            get_time_on_field_for_position(state, &team, "WingerOne".to_string()),
            get_time_on_field_for_position(state, &team, "CentreOne".to_string()),
            get_time_on_field_for_position(state, &team, "CentreTwo".to_string()),
            get_time_on_field_for_position(state, &team, "WingerTwo".to_string()),
            get_time_on_field_for_position(state, &team, "FiveEighth".to_string()),
            get_time_on_field_for_position(state, &team, "HalfBack".to_string()),
            get_time_on_field_for_position(state, &team, "PropOne".to_string()),
            get_time_on_field_for_position(state, &team, "Hooker".to_string()),
            get_time_on_field_for_position(state, &team, "PropTwo".to_string()),
            get_time_on_field_for_position(state, &team, "SecondRowOne".to_string()),
            get_time_on_field_for_position(state, &team, "SecondRowTwo".to_string()),
            get_time_on_field_for_position(state, &team, "Lock".to_string()),
            state.set as f64,
            get_team_margin(state, team.to_string()),
        ],
    );
    execute_events(state);
    log::info!("{}", "[PlayerInterchange] Return interchange position");
    return PlayerInterchangeOutputs::new(true, -1, -1, team);
}
pub fn player_meters(state: &State) -> PlayerMetersOutputs {
    log::info!("{}", "[PlayerMeters] Run player meters model");
    let model: PlayerMetersInferOutputs0 =
        infer::<PlayerMetersInferOutputs0>("player_meters".to_string(), vec![]);
    execute_events(state);
    log::info!("{}", "[PlayerMeters] Return sampled player");
    return PlayerMetersOutputs::new(sample(
        &model.probabilities,
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
    ));
}
pub fn player_of_the_match(state: &mut State, enabled: bool) -> PlayerOfTheMatchOutputs {
    let mut distribution_sum: f64 = 0.0;
    let mut player_distributions: Vec<f64> = Vec::new();
    let mut pom_player_index: i32 = 0;
    let mut sample_index: i32 = 0;
    log::info!(
        "{}",
        "[PlayerOfTheMatch] Skip processing when feature disabled"
    );
    if !enabled {
        log::info!("{}", "[PlayerOfTheMatch] Return");
        return PlayerOfTheMatchOutputs::new(state.player_of_the_match);
        execute_events(state);
    }
    execute_events(state);
    log::info!(
        "{}",
        "[PlayerOfTheMatch] Calculate player of the match distributions"
    );
    player_distributions = calculate_player_of_match_distributions(state);
    execute_events(state);
    log::info!("{}", "[PlayerOfTheMatch] Aggregate distribution sum");
    distribution_sum = player_distributions.iter().sum::<f64>() as f64;
    execute_events(state);
    log::info!("{}", "[PlayerOfTheMatch] Sample player index");
    if distribution_sum > 0.0 {
        log::info!("{}", "[PlayerOfTheMatch] Set variables");
        sample_index = sample_scaled(
            &player_distributions,
            distribution_sum,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        );
        execute_events(state);
    } else {
        log::info!("{}", "[PlayerOfTheMatch] Set variables");
        let _cse_temp_0 = ((DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>())) as f64)
            * (player_distributions.len() as i32) as f64;
        sample_index = _cse_temp_0 as i32;
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[PlayerOfTheMatch] Clamp sampled index to bounds");
    let _cse_temp_1 = (player_distributions.len() as i32).saturating_sub(1);
    sample_index = f64::min((std::cmp::max(sample_index as i32, 0)) as f64, _cse_temp_1);
    execute_events(state);
    log::info!("{}", "[PlayerOfTheMatch] Resolve player index from state");
    pom_player_index = state
        .all_players
        .get(sample_index as usize)
        .cloned()
        .unwrap()
        .player_index;
    execute_events(state);
    log::info!(
        "{}",
        "[PlayerOfTheMatch] Persist player of the match on state"
    );
    state.player_of_the_match = pom_player_index;
    execute_events(state);
    log::info!("{}", "[PlayerOfTheMatch] Return");
    return PlayerOfTheMatchOutputs::new(pom_player_index);
}
pub fn player_sin_bin(state: &mut State, sin_bin_team_is_home: bool) {
    let mut is_home: f64 = 0.0;
    let mut penalty_team_is_home: bool = false;
    let mut sin_bin_player_index: i32 = 0;
    let mut sin_bin_team: String = "".to_string();
    log::info!("{}", "[PlayerSinBin] Determine penalty team side");
    penalty_team_is_home = sin_bin_team_is_home;
    execute_events(state);
    log::info!("{}", "[PlayerSinBin] Set is_home variable");
    is_home = if penalty_team_is_home { 1.0 } else { 0.0 };
    execute_events(state);
    log::info!("{}", "[PlayerSinBin] Sample player sin bin model");
    let model: PlayerSinBinInferOutputs0 = infer::<PlayerSinBinInferOutputs0>(
        "player_sin_bin".to_string(),
        vec![
            is_home,
            state.time_elapsed as f64,
            state.set as f64,
            state.tackles as f64,
            (if penalty_team_is_home {
                state.simulation_invariants.home_price
            } else {
                state.simulation_invariants.away_price
            }) as f64,
            state.simulation_invariants.total_points as f64,
            (state.home_match_score + state.away_match_score) as f64,
            calculate_foul_team_margin(state) as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[PlayerSinBin] Persist sin bin selection");
    sin_bin_player_index = sample(
        &model.probabilities,
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
    );
    sin_bin_team = if penalty_team_is_home {
        "Home".to_string()
    } else {
        "Away".to_string()
    };
    execute_events(state);
    log::info!("{}", "[PlayerSinBin] Record player sin bin event");
    if sin_bin_player_index >= 0 {
        record_player_sin_bin(
            state,
            sin_bin_player_index,
            &sin_bin_team,
            "YellowCard".to_string(),
        );
    }
    execute_events(state);
    log::info!(
        "{}",
        "[PlayerSinBin] Update state markers for sin bin event"
    );
    if sin_bin_player_index >= 0 {
        let last_sin_bin_player_index = sin_bin_player_index;
        let last_sin_bin_team = sin_bin_team;
        let last_event = "PLAYER_SIN_BIN".to_string();
    }
    execute_events(state);
    return;
}
pub fn player_tries(state: &State, team: String) -> PlayerTriesOutputs {
    let player_index: i32 = 0;
    log::info!("{}", "[PlayerTries] Calculate player try distributions");
    let percentage: Vec<f64> = get_team_try_distributions(state);
    execute_events(state);
    log::info!("{}", "[PlayerTries] Run player tries model");
    let model: PlayerTriesInferOutputs0 = infer::<PlayerTriesInferOutputs0>(
        "player_tries".to_string(),
        vec![
            percentage.get(0usize).cloned().unwrap(),
            percentage.get(1usize).cloned().unwrap(),
            percentage.get(2usize).cloned().unwrap(),
            percentage.get(3usize).cloned().unwrap(),
            percentage.get(4usize).cloned().unwrap(),
            percentage.get(5usize).cloned().unwrap(),
            percentage.get(6usize).cloned().unwrap(),
            percentage.get(7usize).cloned().unwrap(),
            percentage.get(8usize).cloned().unwrap(),
            percentage.get(9usize).cloned().unwrap(),
            percentage.get(10usize).cloned().unwrap(),
            percentage.get(11usize).cloned().unwrap(),
            percentage.get(12usize).cloned().unwrap(),
        ],
    );
    execute_events(state);
    log::info!("{}", "[PlayerTries] Return sampled player context");
    return PlayerTriesOutputs::new(
        sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ),
        team,
    );
}
pub fn process_penalty(state: &mut State) {
    log::info!(
        "{}",
        "[ProcessPenalty] Swap possession if penalty was conceded before sampling"
    );
    if state.current_play_type == "ConcededPenalty" {
        swap_possession(state);
    }
    execute_events(state);
    log::info!("{}", "[ProcessPenalty] Get penalty type outcome");
    let penalty_outcome_result: PenaltyTypeOutputs = penalty_type(state);
    execute_events(state);
    log::info!("{}", "[ProcessPenalty] Handle penalty shot");
    if penalty_outcome_result.result_index == 0 {
        log::info!(
            "{}",
            "[ProcessPenalty] Run conversion model for penalty shot"
        );
        let conversion_result: GetConversionModelResultOutputs = get_conversion_model_result(state);
        execute_events(state);
        log::info!("{}", "[ProcessPenalty] Add penalty points if scored");
        if conversion_result.scored {
            log::info!("{}", "[ProcessPenalty] Run add_penalty");
            add_penalty(state);
            execute_events(state);
        }
        execute_events(state);
        log::info!("{}", "[ProcessPenalty] Process kickoff after penalty shot");
        kickoff(state, false, false);
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[ProcessPenalty] Handle scrum lost");
    if penalty_outcome_result.result_index == 2 {
        swap_possession(state);
    }
    execute_events(state);
    log::info!("{}", "[ProcessPenalty] Always check sin bin after penalty");
    process_sin_bin_check(state);
    execute_events(state);
    return;
}
pub fn process_sin_bin_check(state: &mut State) {
    log::info!("{}", "[ProcessSinBinCheck] Run sin bin model");
    let send_off_result: SinBinOutputs = sin_bin(state);
    execute_events(state);
    log::info!("{}", "[ProcessSinBinCheck] Run player sin bin model");
    if send_off_result.sent_off {
        player_sin_bin(
            state,
            ((state.current_play_type == "WonPenalty") && (state.team_in_possession == "Home"))
                || ((state.current_play_type != "WonPenalty")
                    && (state.team_in_possession == "Away")),
        );
    }
    execute_events(state);
    return;
}
pub fn process_tackle(state: &mut State) {
    const TACKLES_PER_SET: i32 = 6;
    log::info!("{}", "[ProcessTackle] Record tackle statistics");
    record_tackle(state);
    execute_events(state);
    log::info!("{}", "[ProcessTackle] Add tackle");
    state.tackles += 1;
    execute_events(state);
    log::info!("{}", "[ProcessTackle] Check if tackle limit exceeded");
    if state.tackles > TACKLES_PER_SET {
        swap_possession(state);
    }
    execute_events(state);
    return;
}
pub fn process_try(state: &mut State) {
    log::info!("{}", "[ProcessTry] Add try points for team in possession");
    add_try(state);
    execute_events(state);
    log::info!(
        "{}",
        "[ProcessTry] Player try assignment if players enabled"
    );
    if state.include_players {
        log::info!("{}", "[ProcessTry] Get player tries model result");
        let try_selection: PlayerTriesOutputs =
            player_tries(state, state.team_in_possession.clone().clone());
        execute_events(state);
        log::info!("{}", "[ProcessTry] Assign try to selected player");
        assign_try(
            state,
            try_selection.player_index,
            try_selection.team.clone(),
        );
        execute_events(state);
    }
    execute_events(state);
    log::info!("{}", "[ProcessTry] Process conversion");
    conversion(state);
    execute_events(state);
    return;
}
pub fn sin_bin(state: &State) -> SinBinOutputs {
    log::info!("{}", "[SinBin] Run sin bin model");
    let model: SinBinInferOutputs0 = infer::<SinBinInferOutputs0>(
        "sin_bin".to_string(),
        vec![
            state.ball_location.x as f64,
            state.ball_location.y as f64,
            calculate_foul_team_margin(state) as f64,
        ],
    );
    execute_events(state);
    log::info!("{}", "[SinBin] Record event");
    return SinBinOutputs::new(
        sample(
            &model.probabilities,
            DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
        ) == 1,
    );
}
pub fn xy(state: &State) -> XyOutputs {
    let mut is_extra_time: bool = false;
    let mut player_advantage: i32 = 0;
    log::info!("{}", "[Xy] Set variables");
    is_extra_time = state.period.name == "ExtraTime";
    player_advantage = (if state.team_in_possession == "Home" {
        (state.away_sin_bin.len() as i32).saturating_sub(state.home_sin_bin.len() as i32)
    } else {
        (state.home_sin_bin.len() as i32).saturating_sub(state.away_sin_bin.len() as i32)
    }) as i32;
    execute_events(state);
    log::info!("{}", "[Xy] Run xy model");
    let model: XyInferOutputs0 = infer::<XyInferOutputs0>(
        "xy".to_string(),
        vec![
            state.tackles as f64,
            (if state.team_in_possession == "Home" {
                state.ball_location.x
            } else {
                (1000 - state.ball_location.x).abs()
            }) as f64,
            (if state.team_in_possession == "Home" {
                state.ball_location.y
            } else {
                (700 - state.ball_location.y).abs()
            }) as f64,
            state.simulation_invariants.total_points as f64,
            get_team_handicap(state),
            btf(player_advantage == 1),
            btf(player_advantage > 1),
            btf(player_advantage == -1),
            btf(player_advantage < -1),
            calculate_margin(state) as f64,
            btf((state.current_play_type == "Pass") || (is_extra_time)),
            btf(
                (["Run", "RunTackle"].contains(&state.current_play_type.as_str()))
                    || ((state.current_play_type == "RunTry") && (!is_extra_time)),
            ),
            btf((["KickRetain", "KickRetainTackle", "KickTurnover"]
                .contains(&state.current_play_type.as_str()))
                || ((state.current_play_type == "KickRetainTry") && (!is_extra_time))),
            btf((state.current_play_type == "RunTry")
                || ((state.current_play_type == "KickRetainTry") && (!is_extra_time))),
        ],
    );
    execute_events(state);
    log::info!("{}", "[Xy] Get grid result");
    let grid_result = sample(
        &model.probabilities,
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
    );
    execute_events(state);
    log::info!("{}", "[Xy] Goal line position");
    let _cse_temp_0 = sample(
        &model.probabilities,
        DEPYLER_RNG.with(|rng| rng.borrow_mut().gen::<f64>()),
    ) % 6;
    let position = GRID_RESULT
        .iter()
        .position(|x| x == &"ZJ1")
        .map(|i| i as i32)
        .expect("ValueError: value is not in list")
        + _cse_temp_0;
    execute_events(state);
    log::info!("{}", "[Xy] Convert grid to field position");
    return XyOutputs::new(FieldPosition::new(0, 0));
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
