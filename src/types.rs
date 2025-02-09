use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElementType {
    Air,
    Earth,
    Fire,
    Ice,
    None,  
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suit {
    pub symbol: String,
    pub element: ElementType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardType {
    Number(Option<u8>, Suit)
}

impl CardType {
    pub fn number_to_emoji(num: Option<u8>) -> &'static str {
        match num {
            Some(1) => ":one:",
            Some(2) => ":two:",
            Some(3) => ":three:",
            Some(4) => ":four:",
            Some(5) => ":five:",
            Some(6) => ":six:",
            Some(7) => ":seven:",
            Some(8) => ":eight:",
            Some(_) => ":question:",
            None => ":question:",
        }
    }
}

pub struct Deck {
    pub cards: Vec<CardType>,
}

impl Deck {
    pub fn new() -> Self {
        let suits = vec![
            Suit {
                symbol: ":diamonds: = :cloud_tornado:".to_string(),
                element: ElementType::Air,
            },
            Suit {
                symbol: ":hearts: = :fire:".to_string(), 
                element: ElementType::Fire,
            },
            Suit {
                symbol: ":spades: = :snowflake:".to_string(),
                element: ElementType::Ice,
            },
            Suit {
                symbol: ":clubs: = :rock:".to_string(),
                element: ElementType::Earth,
            },
        ];

        let mut cards = Vec::with_capacity(30);  // 28 numbered cards + 2 jokers

        for suit in &suits {
            for number in 1..=7 {  // Changed from 8 to 7
                cards.push(CardType::Number(Some(number), suit.clone()));
            }
        }

        // Add two jokers
        for _ in 0..2 {
            cards.push(CardType::Number(None, Suit {
                symbol: ":black_joker:".to_string(),
                element: ElementType::None,  
            }));
        }

        Deck { cards }
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.cards.shuffle(&mut rng);
    }
}

#[derive(Debug, Clone)]
pub enum HandType {
    TripleThreat {
        value: u8,
        suits: Vec<ElementType>,
        card_indices: Vec<usize>,
    },
    MatchedEdge {
        value: u8,
        suits: Vec<ElementType>,
        card_indices: Vec<usize>,
    },
}

impl ElementType {
    pub fn symbol(&self) -> &'static str {
        match self {
            ElementType::Air => ":cloud_tornado:",
            ElementType::Fire => ":fire:",
            ElementType::Ice => ":snowflake:",
            ElementType::Earth => ":rock:",
            ElementType::None => "❓",
        }
    }
}

impl HandType {
    pub fn to_string(&self) -> String {
        match self {
            HandType::TripleThreat { value: _, suits, card_indices, .. } => {
                let cards = card_indices.iter()
                    .map(|&i| format!("[{}]", i + 1))
                    .collect::<Vec<_>>()
                    .join(" ");
                let elements = suits.iter()
                    .map(|e| e.symbol())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("Triple Threat: {} - Elements: {}", cards, elements)
            },
            HandType::MatchedEdge { value: _, suits, card_indices, .. } => {
                let cards = card_indices.iter()
                    .map(|&i| format!("[{}]", i + 1))
                    .collect::<Vec<_>>()
                    .join(" ");
                let elements = suits.iter()
                    .map(|e| e.symbol())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("Matched Edge: {} - Elements: {}", cards, elements)
            }
        }
    }

    pub fn effect_description(&self) -> String {
        match self {
            HandType::TripleThreat { value, suits, .. } => {
                let elements = suits.iter()
                    .map(|e| e.symbol())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("Choose three targets present on the scene. For each target you may choose to:\n\
                • Have them recover from status effects (dazed/shaken/slow/weak) and heal {} Hit Points\n\
                • Have them suffer status effects (dazed/shaken/slow/weak) and take {} damage\n\
                Available elements: {}", value + 15, value + 5, elements)
            },
            HandType::MatchedEdge { value, suits, .. } => {
                let elements = suits.iter()
                    .map(|e| e.symbol())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("Perform a free attack with an equipped weapon. If this attack deals damage:\n\
                • Choose one element from {}\n\
                • All damage becomes that element type\n\
                • Deal {} additional damage", elements, value)
            }
        }
    }
}