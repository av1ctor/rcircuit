pub const COST_TO_PLAY: u128 = (0.01 * 1_00000000.0) as _;

pub const WINNER_REWARD: u128 = 25; // * 100

pub const SKILLS: &[&str] = &[
    "seek-target"
];

pub const STATES: &[&str] = &[
    "idle",
    "walking",
    "celebrating",
    "attacking",
    "hit",
    "dying",
    "dead",
];

