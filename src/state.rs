use std::{collections::HashMap, sync::Arc};
use std::fs;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use crate::types::{CardType, Deck, ElementType, HandType};
use poise::serenity_prelude::UserId;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize)]
pub struct PlayerState {
    pub deck: Deck,
    pub hand: Vec<CardType>,    
    pub discard: Vec<CardType>, 
}

impl PlayerState {
    pub fn new() -> Self {
        let mut deck = Deck::new();
        deck.shuffle();
        PlayerState {
            deck,
            hand: Vec::new(),
            discard: Vec::new(),
        }
    }

    pub fn draw_to_hand(&mut self, num_cards: usize) -> Result<(), String> {
        for _ in 0..num_cards {
            if let Some(card) = self.deck.cards.pop() {
                self.hand.push(card);
            } else {
                if !self.discard.is_empty() {
                    self.deck.cards.append(&mut self.discard);
                    self.deck.shuffle();
                    
                    if let Some(card) = self.deck.cards.pop() {
                        self.hand.push(card);
                    } else {
                        return Err("No cards left in deck or discard".to_string());
                    }
                } else {
                    return Err("No cards left in deck or discard".to_string());
                }
            }
        }
        Ok(())
    } 
    
    pub fn discard_from_hand(&mut self, card_index: usize) -> Result<(), String> {
        if card_index >= self.hand.len() {
            return Err("Card index out of bounds".to_string());
        }
        
        let card = self.hand.remove(card_index);
        self.discard.push(card);
        Ok(())
    } 
    
    pub fn find_possible_hands(&self) -> Vec<HandType> {
        let mut hands = Vec::new();
        let hand_len = self.hand.len();
        
        // Check for triples
        for i in 0..hand_len {
            for j in (i + 1)..hand_len {
                for k in (j + 1)..hand_len {
                    if let Some(hand_type) = self.check_triple(i, j, k) {
                        hands.push(hand_type);
                    }
                }
            }
        }
        
        // Check for pairs
        for i in 0..hand_len {
            for j in (i + 1)..hand_len {
                if let Some(hand_type) = self.check_pair(i, j) {
                    hands.push(hand_type);
                }
            }
        }
        
        hands
    }

    fn check_triple(&self, i: usize, j: usize, k: usize) -> Option<HandType> {
        let cards = [&self.hand[i], &self.hand[j], &self.hand[k]];
        let mut value = None;
        let mut joker_count = 0;
        let mut non_joker_suits = Vec::new();

        // First pass to find value and count jokers
        for card in &cards {
            match card {
                CardType::Number(v, suit) => {
                    if let Some(num) = v {
                        if value.is_none() {
                            value = Some(*num);
                        } else if value != Some(*num) {
                            return None;
                        }
                        if suit.element != ElementType::None {
                            non_joker_suits.push(suit.element.clone());
                        }
                    } else {
                        joker_count += 1;
                    }
                },
                CardType::Joker { .. } => {
                    joker_count += 1;
                }
            }
        }

        // Use 7 as default if no value was found (all jokers)
        let value = value.unwrap_or(7);
        
        if joker_count + non_joker_suits.len() == 3 {
            Some(HandType::TripleThreat {
                value,
                suits: if joker_count > 0 {
                    vec![ElementType::Air, ElementType::Earth, ElementType::Fire, ElementType::Ice]
                } else {
                    non_joker_suits
                },
                card_indices: vec![i, j, k],
            })
        } else {
            None
        }
    }

    fn check_pair(&self, i: usize, j: usize) -> Option<HandType> {
        let cards = [&self.hand[i], &self.hand[j]];
        let mut value = None;
        let mut joker_count = 0;
        let mut non_joker_suits = Vec::new();

        // First pass to find value and count jokers
        for card in &cards {
            match card {
                CardType::Number(v, suit) => {
                    if let Some(num) = v {
                        if value.is_none() {
                            value = Some(*num);
                        } else if value != Some(*num) {
                            return None;
                        }
                        if suit.element != ElementType::None {
                            non_joker_suits.push(suit.element.clone());
                        }
                    } else {
                        joker_count += 1;
                    }
                },
                CardType::Joker { .. } => {
                    joker_count += 1;
                }
            }
        }

        // Use 7 as default if no value was found (all jokers)
        let value = value.unwrap_or(7);
        
        if joker_count + non_joker_suits.len() == 2 {
            Some(HandType::MatchedEdge {
                value,
                suits: if joker_count > 0 {
                    vec![ElementType::Air, ElementType::Earth, ElementType::Fire, ElementType::Ice]
                } else {
                    non_joker_suits
                },
                card_indices: vec![i, j],
            })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct PlayerStateManager {
    pub players: HashMap<UserId, PlayerState>,
    #[serde(skip)]
    dirty: bool,
    #[serde(skip)]
    last_save: Option<Instant>,
}

impl PlayerStateManager {
    pub fn new() -> Self {
        PlayerStateManager {
            players: HashMap::new(),
            dirty: false,
            last_save: Some(Instant::now()),
        }
    }

    pub fn get_player_state(&mut self, user_id: UserId) -> Option<&mut PlayerState> {
        self.players.get_mut(&user_id)
    }

    pub fn start_new_combat(&mut self, user_id: UserId) -> &mut PlayerState {
        self.players.insert(user_id, PlayerState::new());
        self.mark_dirty();
        self.players.get_mut(&user_id).unwrap()
    }
    
    // Save state to file
    pub fn save_state(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.dirty {
            return Ok(());
        }
        
        let json = serde_json::to_string(&self)?;
        // Create a temp file first to avoid corruption if the process crashes
        fs::write("player_state.json.tmp", &json)?;
        fs::rename("player_state.json.tmp", "player_state.json")?;
        
        self.dirty = false;
        self.last_save = Some(Instant::now());
        Ok(())
    }

    pub async fn save_if_needed(arc_self: &Arc<Mutex<Self>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut self_guard = arc_self.lock().await;
    if self_guard.dirty {
        self_guard.save_state()?;
    }
    Ok(())
}
    
    // Load state from file
    pub fn load_state() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match fs::read_to_string("player_state.json") {
            Ok(json) => {
                let mut state: PlayerStateManager = serde_json::from_str(&json)?;
                state.dirty = false;
                state.last_save = Some(Instant::now());
                Ok(state)
            },
            Err(_) => Ok(Self::new()) // Create new if file doesn't exist
        }
    }
    
    // Mark state as modified
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}