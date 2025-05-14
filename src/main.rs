mod types;
mod state;

use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use types::{format_element_list, HandType};
use crate::state::PlayerStateManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::types::CardType;

type Error = Box<dyn std::error::Error + Send + Sync>;
pub struct Data {
    player_state_manager: Arc<Mutex<PlayerStateManager>>,
}

fn format_hand_display(hand: &[CardType]) -> String {
    if hand.is_empty() {
        return String::from("Your hand is empty!");
    }

    let mut display = String::from("Your hand:\n");
    for (i, card) in hand.iter().enumerate() {
        let card_display = match card {
            CardType::Number(num, suit) => {
                format!("{}. {} {}\n", 
                    i + 1,
                    CardType::number_to_emoji(num.unwrap_or(0)),
                    suit.symbol
                )
            },
            CardType::Joker { current_value, current_suit, symbol } => {
                match (current_value, current_suit) {
                    (Some(val), Some(suit)) => format!("{}. {} {}\n",
                        i + 1,
                        CardType::number_to_emoji(*val),
                        suit.symbol
                    ),
                    _ => format!("{}. :question: {}\n",
                        i + 1,
                        symbol
                    )
                }
            }
        };
        display.push_str(&card_display);
    }
    display
}

#[poise::command(slash_command)]
pub async fn start_new_combat(
    ctx: poise::Context<'_, Data, Error>,
) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let mut player_state_manager = ctx.data().player_state_manager.lock().await;
    let player = player_state_manager.start_new_combat(user_id);
    player.draw_to_hand(5)?;
    
    // Get the hand before dropping the lock
    let hand = player.hand.clone();
    drop(player_state_manager);
    
    // Combine both messages into one response
    let message = format!("Combat started! Drew 5 cards.\n{}", format_hand_display(&hand));
    ctx.say(message).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn mulligan(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Card positions to mulligan (1-5, space-separated)"] positions: String,
) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let mut player_state_manager = ctx.data().player_state_manager.lock().await;
    
    let Some(player) = player_state_manager.get_player_state(user_id) else {
        ctx.say("You haven't started a combat yet! Use /start_new_combat to begin.").await?;
        return Ok(());
    };
    
    let mut indices: Vec<usize> = positions
        .split_whitespace()
        .filter_map(|s| s.parse::<usize>().ok())
        .map(|n| n - 1) // Convert to 0-based indexing
        .collect();
    indices.sort_unstable_by(|a, b| b.cmp(a)); // Sort in reverse to remove from highest index first
    
    if indices.is_empty() || indices.len() > 5 || indices.iter().any(|&i| i >= player.hand.len()) {
        ctx.say("Please provide 1-5 valid card positions (1-5)").await?;
        return Ok(());
    }
    
    for &index in &indices {
        player.discard_from_hand(index)?;
    }
    
    player.draw_to_hand(indices.len())?;
    
    // Get the hand before dropping the lock
    let hand = player.hand.clone();
    drop(player_state_manager);
    
    // Combine both messages into one response
    let message = format!("Mulligan complete!\n{}", format_hand_display(&hand));
    ctx.say(message).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn view_hand(
    ctx: poise::Context<'_, Data, Error>,
) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let player_state_manager = ctx.data().player_state_manager.lock().await;
    
    let Some(player) = player_state_manager.players.get(&user_id) else {
        ctx.say("You haven't started a combat yet! Use /start_new_combat to begin.").await?;
        return Ok(());
    };

    let hand = player.hand.clone();
    drop(player_state_manager);
    
    ctx.say(format_hand_display(&hand)).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn view_possible_resolutions(
    ctx: poise::Context<'_, Data, Error>,
) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let player_state_manager = ctx.data().player_state_manager.lock().await;
    
    let Some(player) = player_state_manager.players.get(&user_id) else {
        ctx.say("You haven't started a combat yet! Use /start_new_combat to begin.").await?;
        return Ok(());
    };

    let possible_hands = player.find_possible_hands();
    if possible_hands.is_empty() {
        ctx.say("No valid hands available.").await?;
        return Ok(());
    }

    let mut response = String::from("Available hands:\n");
    for (i, hand) in possible_hands.iter().enumerate() {
        // Get the card indices (positions) for this hand
        let card_positions = match hand {
            HandType::TripleThreat { card_indices, .. } => card_indices,
            HandType::MatchedEdge { card_indices, .. } => card_indices,
            HandType::Jackpot { card_indices, .. } => card_indices,
            HandType::DoubleTrouble { card_indices, .. } => card_indices,
        };
        
        // Get MP cost based on hand type
        let mp_cost = match hand {
            HandType::TripleThreat { .. } => 10,
            HandType::MatchedEdge { .. } => 5,
            HandType::Jackpot { .. } => 20,
            HandType::DoubleTrouble { .. } => 20,
        };
        
        // Convert to 1-based indexing for display and sort for readability
        let mut display_positions: Vec<usize> = card_positions.iter().map(|&idx| idx + 1).collect();
        display_positions.sort();
        
        // Format the positions as a string like "Cards: 1, 3, 5"
        let positions_str = format!("Cards: {}", display_positions.iter()
            .map(|pos| pos.to_string())
            .collect::<Vec<_>>()
            .join(", "));
        
        response.push_str(&format!("{}. {} (MP Cost: {}, {})\n", i + 1, hand.to_string(), mp_cost, positions_str));
    }
    
    ctx.say(response).await?;
    Ok(())
}


#[poise::command(slash_command)]
pub async fn resolve_hand(
    ctx: poise::Context<'_, Data, Error>,
    #[description = "Hand number from the list"] hand_number: usize,
) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let mut player_state_manager = ctx.data().player_state_manager.lock().await;
    
    let Some(player) = player_state_manager.get_player_state(user_id) else {
        ctx.say("You haven't started a combat yet! Use /start_new_combat to begin.").await?;
        return Ok(());
    };

    let possible_hands = player.find_possible_hands();
    if hand_number == 0 || hand_number > possible_hands.len() {
        ctx.say("Invalid hand number.").await?;
        return Ok(());
    }

    let hand = &possible_hands[hand_number - 1];
    
    // Get the available elements for this hand
    let available_elements = match hand {
        HandType::TripleThreat { suits, .. } => suits,
        HandType::MatchedEdge { suits, .. } => suits,
        HandType::Jackpot { suits, .. } => suits,
        HandType::DoubleTrouble { suits, .. } => suits,
    };
    
    // Format elements as a string with square brackets
    let elements_str = format_element_list(available_elements);
    
    // Discard the used cards
    match hand {
        HandType::TripleThreat { card_indices, .. } |
        HandType::MatchedEdge { card_indices, .. } |
        HandType::Jackpot { card_indices, .. } |
        HandType::DoubleTrouble { card_indices, .. } => {
            for &index in card_indices.iter().rev() {
                player.discard_from_hand(index)?;
            }
            // Draw back up to 5
            let cards_needed = 5 - player.hand.len();
            if cards_needed > 0 {
                player.draw_to_hand(cards_needed)?;
            }
        }
    }

    let hand_clone = player.hand.clone();
    drop(player_state_manager);
    
    // Format the effect message with bracketed elements
    let effect_message = match hand {
        HandType::TripleThreat { value, .. } => {
            format!("Triple Threat resolved! Three targets of your choice recover or suffer from **dazed, shaken, slow or weak**. If the target recovers then it also heals Hit Points equal to {}. If the target suffers it also takes {} {} damage.", 
                value + 15, 
                value + 5, 
                elements_str)
        },
        HandType::MatchedEdge { value, .. } => {
            format!("Matched Edge resolved! Your weapon strike deals {} bonus {} damage!", 
                value, 
                elements_str)
        },
        HandType::Jackpot { value, .. } => {
            format!("Jackpot resolved! You and every ally present on the scene recover 777 Hit Points, 777 Mind Points, and recover from all status effects; any PCs who have surrendered but are still part of the scene immediately regain consciousness (this does not cancel the effects of their Surrender).")
        },
        HandType::DoubleTrouble { first_pair_value, second_pair_value, .. } => {
            format!("Double Trouble resolved! You deal damage equal to {} (15 + {} + {}) to each of up to two different enemies you can see that are present on the scene; the type of this damage is one of your choice among those matching the suits of the resolved cards: {}",
                15 + first_pair_value + second_pair_value,
                first_pair_value,
                second_pair_value,
                elements_str)
        }
    };
    
    let message = format!("{}\n{}", 
        effect_message,
        format_hand_display(&hand_clone));
    ctx.say(message).await?;
    Ok(())
}

// Define the commands list as a static
static COMMANDS: &[fn() -> poise::Command<Data, Error>] = &[
    start_new_combat,
    mulligan,
    view_hand,
    view_possible_resolutions,
    resolve_hand,
];

// Update your main() function to include the GameState
#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();
    
    // Try to load saved state, or create a new one if loading fails
    let player_state_manager = Arc::new(Mutex::new(
        PlayerStateManager::load_state().unwrap_or_else(|e| {
            eprintln!("Error loading state: {}, starting fresh", e);
            PlayerStateManager::new()
        })
    ));
    
    let state_manager_clone = player_state_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = PlayerStateManager::save_if_needed(&state_manager_clone).await {
                eprintln!("Failed to save state: {}", e);
            }
        }
    });
    
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: COMMANDS.iter().map(|cmd| cmd()).collect(),
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    player_state_manager: player_state_manager.clone(),
                })
            })
        });

    framework.run().await?;
    Ok(())
}