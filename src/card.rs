/// All implemented cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CardId {
    // Ironclad basics
    Strike,
    Defend,
    Bash,
    // Common attacks
    TwinStrike,
    IronWave,
    Cleave,
    Clothesline,
    HeavyBlade,
    BodySlam,
    Thunderclap,
    PommelStrike,
    Anger,
    WildStrike,
    SwordBoomerang,
    Dropkick,
    // Common skills
    ShrugItOff,
    TrueGrit,
    Flex,
    Intimidate,
    Armaments,
    Warcry,
    Headbutt,
    Entrench,
    // Power cards
    Inflame,
    Metallicize,
    DemonForm,
    // Status cards (added to deck by enemies/effects, not rewards)
    Slimed,
    Wound,
    Dazed,
}

/// A card instance in the player's deck or hand.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Card {
    pub id: CardId,
    pub upgraded: bool,
    pub cost: i32,
}

impl Card {
    pub fn new(id: CardId) -> Self {
        let cost = base_cost(id);
        Card { id, upgraded: false, cost }
    }

    pub fn upgraded(id: CardId) -> Self {
        Card { id, upgraded: true, cost: upgraded_cost(id) }
    }
}

pub fn base_cost(id: CardId) -> i32 {
    match id {
        CardId::Strike       => 1,
        CardId::Defend       => 1,
        CardId::Bash         => 2,
        CardId::TwinStrike   => 1,
        CardId::IronWave     => 1,
        CardId::Cleave       => 1,
        CardId::Clothesline  => 2,
        CardId::HeavyBlade   => 2,
        CardId::BodySlam     => 1,
        CardId::Thunderclap  => 1,
        CardId::PommelStrike => 1,
        CardId::Anger        => 0,
        CardId::WildStrike   => 1,
        CardId::SwordBoomerang => 1,
        CardId::Dropkick     => 1,
        CardId::ShrugItOff   => 1,
        CardId::TrueGrit     => 1,
        CardId::Flex         => 0,
        CardId::Intimidate   => 0,
        CardId::Armaments    => 1,
        CardId::Warcry       => 0,
        CardId::Headbutt     => 1,
        CardId::Entrench     => 2,
        CardId::Inflame      => 1,
        CardId::Metallicize  => 1,
        CardId::DemonForm    => 3,
        CardId::Slimed       => 1,
        CardId::Wound        => 99, // effectively unplayable
        CardId::Dazed        => 99,
    }
}

fn upgraded_cost(id: CardId) -> i32 {
    match id {
        CardId::BodySlam => 0,
        other            => base_cost(other),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType { Attack, Skill, Power, Status }

pub fn card_type(id: CardId) -> CardType {
    match id {
        CardId::Strike | CardId::Bash | CardId::TwinStrike | CardId::IronWave |
        CardId::Cleave | CardId::Clothesline | CardId::HeavyBlade | CardId::BodySlam |
        CardId::Thunderclap | CardId::PommelStrike | CardId::Anger | CardId::WildStrike |
        CardId::SwordBoomerang | CardId::Dropkick => CardType::Attack,

        CardId::Defend | CardId::ShrugItOff | CardId::TrueGrit | CardId::Flex |
        CardId::Intimidate | CardId::Armaments | CardId::Warcry | CardId::Headbutt |
        CardId::Entrench => CardType::Skill,

        CardId::Inflame | CardId::Metallicize | CardId::DemonForm => CardType::Power,

        CardId::Slimed | CardId::Wound | CardId::Dazed => CardType::Status,
    }
}

/// Whether a card requires selecting an enemy target.
/// AoE and self-targeting cards return false; random-target cards also return false
/// (target is chosen internally in combat resolution).
pub fn requires_target(id: CardId) -> bool {
    matches!(id,
        CardId::Strike | CardId::Bash | CardId::TwinStrike | CardId::IronWave |
        CardId::Clothesline | CardId::HeavyBlade | CardId::BodySlam |
        CardId::PommelStrike | CardId::Anger | CardId::WildStrike |
        CardId::Dropkick | CardId::Headbutt
    )
}

/// Whether a card can be selected as an action, regardless of energy.
pub fn is_playable(id: CardId) -> bool {
    !matches!(id, CardId::Wound | CardId::Dazed)
}

pub fn is_ethereal(id: CardId) -> bool {
    matches!(id, CardId::Dazed)
}

/// Stable ordinal used for OBS encoding (1-indexed, 1..=CARD_COUNT).
pub const CARD_COUNT: u32 = 29;
pub fn card_ordinal(id: CardId) -> u32 {
    match id {
        CardId::Strike        =>  1,
        CardId::Defend        =>  2,
        CardId::Bash          =>  3,
        CardId::TwinStrike    =>  4,
        CardId::IronWave      =>  5,
        CardId::Cleave        =>  6,
        CardId::Clothesline   =>  7,
        CardId::HeavyBlade    =>  8,
        CardId::BodySlam      =>  9,
        CardId::Thunderclap   => 10,
        CardId::PommelStrike  => 11,
        CardId::Anger         => 12,
        CardId::WildStrike    => 13,
        CardId::SwordBoomerang=> 14,
        CardId::Dropkick      => 15,
        CardId::ShrugItOff    => 16,
        CardId::TrueGrit      => 17,
        CardId::Flex          => 18,
        CardId::Intimidate    => 19,
        CardId::Armaments     => 20,
        CardId::Warcry        => 21,
        CardId::Headbutt      => 22,
        CardId::Entrench      => 23,
        CardId::Inflame       => 24,
        CardId::Metallicize   => 25,
        CardId::DemonForm     => 26,
        CardId::Slimed        => 27,
        CardId::Wound         => 28,
        CardId::Dazed         => 29,
    }
}
