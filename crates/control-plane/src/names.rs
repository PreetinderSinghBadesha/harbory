use rand::Rng;

/// Small, deliberately game-flavored word lists — matches the dashboard's
/// "party of robots" framing rather than Docker's classic
/// adjective_surname generator. Picked at registration time so every agent
/// gets a friendly default without the operator having to name it first.
const ADJECTIVES: &[&str] = &[
    "brave", "swift", "quiet", "rusty", "lucky", "clever", "sturdy", "nimble", "bold", "steady",
    "sharp", "cozy", "spry", "loyal", "plucky", "grim", "jolly", "quick", "wily", "stout",
];

const NOUNS: &[&str] = &[
    "falcon", "otter", "badger", "sparrow", "wolf", "beetle", "heron", "lynx", "raven", "gecko",
    "moth", "viper", "hawk", "mole", "crow", "ferret", "wren", "stag", "newt", "shrike",
];

/// e.g. "brave-falcon-42" — words plus a small number so collisions across
/// an account's agents are unlikely without needing a uniqueness check.
pub fn generate() -> String {
    let mut rng = rand::thread_rng();
    let adjective = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.gen_range(0..NOUNS.len())];
    let suffix: u16 = rng.gen_range(1..100);
    format!("{adjective}-{noun}-{suffix}")
}
