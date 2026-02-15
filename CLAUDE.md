# Pubky Canva

Collaborative pixel canvas (r/place style) built on the Pubky protocol.

## Architecture

- **`canva-indexer/`** — Rust binary (Axum + SQLite) that polls homeservers, validates pixel events, and serves REST + SSE
- **`frontend/`** — Next.js 15 app that renders the canvas and writes pixels via @synonymdev/pubky SDK

## Data Flow

- **Writes**: Frontend → user's Pubky homeserver at `/pub/pubky-canva/pixels/<timestamp_id>`
- **Reads**: Frontend → canva-indexer REST API (`GET /api/canvas`)
- **Real-time**: Frontend ← canva-indexer SSE (`GET /api/events`)
- **Indexing**: canva-indexer polls homeservers → validates → stores in SQLite → broadcasts SSE

## Running Locally

### Prerequisites
- Rust toolchain
- Node.js 20+
- `pubky-testnet` running (`cargo install pubky-testnet && pubky-testnet`)

### Indexer
```bash
cd canva-indexer
cargo run -- config.toml
# Listens on http://localhost:3001
```

### Frontend
```bash
cd frontend
npm install
npm run dev
# Listens on http://localhost:3000
```

## Key Design Decisions

- **PICO-8 palette**: 16 colors (indexed 0-15), stored as u8 on homeserver
- **Credit system**: 10 max credits, 1 credit per pixel, regenerates 1 every 10 minutes
- **No fixed homeserver**: Indexer discovers homeservers via Pkarr when users are ingested
- **Canvas resize**: Doubles when 100% filled + 50% overwritten by different users
- **Anti-cheat**: Indexer enforces credit limits, bounds, timestamps — frontend is advisory only
