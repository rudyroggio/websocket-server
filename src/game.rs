use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Game not found with code: {0}")]
    GameNotFound(String),
    #[error("Game is not active")]
    GameNotActive,
    #[error("Player not found")]
    PlayerNotFound,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub players: HashMap<Uuid, Player>,
    pub is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            is_active: false,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn add_player(&mut self, id: Uuid, name: String) -> Player {
        let player = Player { name, score: 0 };
        self.players.insert(id, player.clone());
        player
    }

    pub fn remove_player(&mut self, id: &Uuid) -> Option<Player> {
        self.players.remove(id)
    }

    pub fn increment_score(&mut self, id: &Uuid) -> Result<i32, GameError> {
        let player = self.players.get_mut(id).ok_or(GameError::PlayerNotFound)?;
        player.score += 1;
        Ok(player.score)
    }

    pub fn get_players(&self) -> Vec<Player> {
        self.players.values().cloned().collect()
    }
}

pub struct GameManager {
    games: RwLock<HashMap<String, GameState>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_game(&self, code: String, player_id: Uuid, player_name: String) -> GameState {
        let mut games = self.games.write();
        let mut game_state = GameState::new();
        game_state.add_player(player_id, player_name);
        games.insert(code.clone(), game_state.clone());
        game_state
    }

    pub fn join_game(&self, code: &str, player_id: Uuid, player_name: String) -> Result<GameState, GameError> {
        let mut games = self.games.write();
        let game = games.get_mut(code).ok_or(GameError::GameNotFound(code.to_string()))?;
        game.add_player(player_id, player_name);
        Ok(game.clone())
    }

    pub fn start_game(&self, code: &str) -> Result<(), GameError> {
        let mut games = self.games.write();
        let game = games.get_mut(code).ok_or(GameError::GameNotFound(code.to_string()))?;
        game.is_active = true;
        Ok(())
    }

    pub fn submit_solution(&self, code: &str, player_id: &Uuid, used_hint: bool) -> Result<Vec<Player>, GameError> {
        let mut games = self.games.write();
        let game = games.get_mut(code).ok_or(GameError::GameNotFound(code.to_string()))?;

        if !game.is_active {
            return Err(GameError::GameNotActive);
        }

        if !used_hint {
            game.increment_score(player_id)?;
        }

        Ok(game.get_players())
    }

    pub fn remove_player(&self, code: &str, player_id: &Uuid) -> Option<()> {
        let mut games = self.games.write();
        let game = games.get_mut(code)?;

        game.remove_player(player_id);

        if game.players.is_empty() {
            games.remove(code);
        }

        Some(())
    }
}