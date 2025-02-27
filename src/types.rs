use serde::{Serialize, Deserialize};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ElementType {
    Fire,
    Ice,
    Earth,
    Air,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Suit {
    pub element: ElementType,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CardType {
    Number(Option<u8>, Suit),
    Joker {
        current_value: Option<u8>,
        current_suit: Option<Suit>,
        symbol: String,
    },
}

impl CardType {
    pub fn number_to_emoji(number: u8) -> String {
        match number {
            1 => "1️⃣".to_string(),
            2 => "2️⃣".to_string(),
            3 => "3️⃣".to_string(),
            4 => "4️⃣".to_string(),
            5 => "5️⃣".to_string(),
            6 => "6️⃣".to_string(),
            7 => "7️⃣".to_string(),
            _ => "❓".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Deck {
    pub cards: Vec<CardType>,
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::new();
        
        // Create suits
        let fire_suit = Suit {
            element: ElementType::Fire,
            symbol: "🔥".to_string(),
        };
        
        let ice_suit = Suit {
            element: ElementType::Ice,
            symbol: "❄️".to_string(),
        };
        
        let earth_suit = Suit {
            element: ElementType::Earth,
            symbol: "🪨".to_string(),
        };
        
        let air_suit = Suit {
            element: ElementType::Air,
            symbol: "💨".to_string(),
        };
        
        // Add numbered cards
        for number in 1..=7 {
            cards.push(CardType::Number(Some(number), fire_suit.clone()));
            cards.push(CardType::Number(Some(number), ice_suit.clone()));
            cards.push(CardType::Number(Some(number), earth_suit.clone()));
            cards.push(CardType::Number(Some(number), air_suit.clone()));
        }
        
        // Add jokers
        cards.push(CardType::Joker {
            current_value: None,
            current_suit: None,
            symbol: "🃏".to_string(),
        });
        
        cards.push(CardType::Joker {
            current_value: None,
            current_suit: None,
            symbol: "🃏".to_string(),
        });
        
        Deck { cards }
    }
    
    pub fn shuffle(&mut self) {
        self.cards.shuffle(&mut thread_rng());
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

impl HandType {
    pub fn to_string(&self) -> String {
        match self {
            HandType::TripleThreat { value, suits, .. } => {
                let elements = format_element_list(suits);
                format!("Triple Threat: {} (Elements: {})", value, elements)
            },
            HandType::MatchedEdge { value, suits, .. } => {
                let elements = format_element_list(suits);
                format!("Matched Edge: {} (Elements: {})", value, elements)
            }
        }
    }
}

// Helper function to format element lists
pub fn format_element_list(elements: &[ElementType]) -> String {
    if elements.is_empty() {
        return "None".to_string();
    }
    
    let mut emoji_list = Vec::new();
    for element in elements {
        let element_emoji = match element {
            ElementType::Fire => "🔥",
            ElementType::Ice => "❄️",
            ElementType::Earth => "🪨",
            ElementType::Air => "💨",
            ElementType::None => "",
        };
        if !emoji_list.contains(&element_emoji) && !element_emoji.is_empty() {
            emoji_list.push(element_emoji);
        }
    }
    
    if emoji_list.is_empty() {
        return "None".to_string();
    }
    
    format!("[{}]", emoji_list.join(", "))
}