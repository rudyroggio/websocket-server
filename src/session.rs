use actix::{Actor, ActorContext, StreamHandler, AsyncContext};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use std::sync::Arc;
use crate::game::GameManager;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
enum ClientMessage {
    CreateGame { player_name: String },
    JoinGame { code: String, player_name: String },
    StartGame,
    SubmitSolution { used_hint: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
enum ServerMessage {
    GameCreated { game_code: String },
    PlayerJoined { players: Vec<crate::game::Player> },
    GameStarted,
    UpdateScores { players: Vec<crate::game::Player> },
    Error { message: String },
}

pub struct WsGameSession {
    id: Uuid,
    game_code: Option<String>,
    last_heartbeat: Instant,
    game_manager: std::sync::Arc<GameManager>,
}

impl WsGameSession {
    pub fn new(game_manager: std::sync::Arc<GameManager>) -> Self {
        Self {
            id: Uuid::new_v4(),
            game_code: None,
            last_heartbeat: Instant::now(),
            game_manager,
        }
    }

    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                warn!("Client heartbeat failed, disconnecting session {}", act.id);
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn handle_message(&mut self, msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) {
        let response = match msg {
            ClientMessage::CreateGame { player_name } => {
                let game_code = format!("{:06X}", rand::random::<u32>());
                info!("Creating game {} for player {}", game_code, player_name);

                let _game_state = self.game_manager.create_game(
                    game_code.clone(),
                    self.id,
                    player_name
                );
                self.game_code = Some(game_code.clone());

                ServerMessage::GameCreated { game_code }
            }

            ClientMessage::JoinGame { code, player_name } => {
                info!("Player {} attempting to join game {}", player_name, code);

                match self.game_manager.join_game(&code, self.id, player_name) {
                    Ok(game_state) => {
                        self.game_code = Some(code);
                        ServerMessage::PlayerJoined { players: game_state.get_players() }
                    }
                    Err(e) => ServerMessage::Error { message: e.to_string() }
                }
            }

            ClientMessage::StartGame => {
                if let Some(code) = &self.game_code {
                    match self.game_manager.start_game(code) {
                        Ok(_) => ServerMessage::GameStarted,
                        Err(e) => ServerMessage::Error { message: e.to_string() }
                    }
                } else {
                    ServerMessage::Error { message: "Not in a game".to_string() }
                }
            }

            ClientMessage::SubmitSolution { used_hint } => {
                if let Some(code) = &self.game_code {
                    match self.game_manager.submit_solution(code, &self.id, used_hint) {
                        Ok(players) => ServerMessage::UpdateScores { players },
                        Err(e) => ServerMessage::Error { message: e.to_string() }
                    }
                } else {
                    ServerMessage::Error { message: "Not in a game".to_string() }
                }
            }
        };

        if let Err(e) = serde_json::to_string(&response)
            .map(|json| ctx.text(json))
        {
            error!("Failed to serialize response: {}", e);
        }
    }
}

impl Actor for WsGameSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        info!("New session started: {}", self.id);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        if let Some(code) = &self.game_code {
            self.game_manager.remove_player(code, &self.id);
            info!("Player {} removed from game {}", self.id, code);
        }
        info!("Session stopped: {}", self.id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsGameSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                debug!("Received message: {}", text);
                match serde_json::from_str(&text) {
                    Ok(message) => self.handle_message(message, ctx),
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
                        let error = ServerMessage::Error {
                            message: "Invalid message format".to_string()
                        };
                        if let Ok(json) = serde_json::to_string(&error) {
                            ctx.text(json);
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => {
                info!("Client disconnected. Reason: {:?}", reason);
                ctx.stop();
            }
            Ok(ws::Message::Binary(_)) => {
                warn!("Unexpected binary message received");
            }
            Err(e) => {
                error!("Error handling message: {:?}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

pub async fn handle_ws_connection(
    req: HttpRequest,
    stream: web::Payload,
    game_manager: web::Data<Arc<GameManager>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsGameSession::new(game_manager.get_ref().clone()),
        &req,
        stream,
    )
}