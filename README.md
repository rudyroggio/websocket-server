# WebSocket Game Server

Lightweight WebSocket game server built with Rust, designed for managing multiplayer game sessions with real-time scoring. 

## Running the Server

Start the server:
```bash
cargo run
```

The server will start on `localhost:3001`

## API Endpoints

- `GET /health` - Health check endpoint
- `GET /ws` - WebSocket connection endpoint

## WebSocket Events

### Client to Server

1. Create Game
```json
{
    "event": "createGame",
    "playerName": "Player1"
}
```

2. Join Game
```json
{
    "event": "joinGame",
    "code": "GAMECODE",
    "playerName": "Player2"
}
```

3. Start Game
```json
{
    "event": "startGame"
}
```

4. Submit Solution
```json
{
    "event": "submitSolution",
    "usedHint": false
}
```

### Server to Client

1. Game Created
```json
{
    "event": "gameCreated",
    "gameCode": "ABC123"
}
```

2. Player Joined
```json
{
    "event": "playerJoined",
    "players": [
        {"name": "Player1", "score": 0},
        {"name": "Player2", "score": 0}
    ]
}
```

3. Game Started
```json
{
    "event": "gameStarted"
}
```

4. Scores Updated
```json
{
    "event": "updateScores",
    "players": [
        {"name": "Player1", "score": 1},
        {"name": "Player2", "score": 0}
    ]
}
```

