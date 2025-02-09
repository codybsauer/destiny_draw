mod types;
mod state;

use poise::serenity_prelude as serenity;
use dotenv::dotenv;
use types::HandType;
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
                    CardType::number_to_emoji(*num),
                    suit.symbol
                )
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
        response.push_str(&format!("{}. {}\n", i + 1, hand.to_string()));
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
    // Discard the used cards
    match hand {
        HandType::TripleThreat { card_indices, .. } |
        HandType::MatchedEdge { card_indices, .. } => {
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

    let hand = player.hand.clone();
    drop(player_state_manager);
    
    let message = format!("{}\n{}", 
        possible_hands[hand_number - 1].effect_description(),
        format_hand_display(&hand));
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
    let player_state_manager = Arc::new(Mutex::new(PlayerStateManager::new()));
    
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: COMMANDS.iter().map(|cmd| cmd()).collect(),
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Registering commands...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                println!("Commands registered successfully!");
                Ok(Data {
                    player_state_manager: player_state_manager.clone(),
                })
            })
        });

    framework.run().await?;
    Ok(())
}