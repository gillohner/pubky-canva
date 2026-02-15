# Pubky Canva

A collaborative pixel canvas (like Reddit r/place) built on the [Pubky](https://pubky.tech) decentralized protocol. Users authenticate with their Pubky identity via [Pubky Ring](https://pubky.tech/ring) and place colored pixels on a shared canvas.

## Architecture

```
Browser (Next.js)                   Canva Indexer (Rust)
┌──────────────┐    SSE events     ┌──────────────────┐
│ View canvas  │◄──────────────────│ REST API + SSE   │
│ Place pixels │                   │ Poll homeservers │
│ Pubky auth   │──PUT /ingest/pk──►│ Validate pixels  │
│              │──write pixel──►HS │ Store in SQLite  │
└──────────────┘                   └──────────────────┘
```

1. User authenticates via Pubky Ring (QR code) or recovery file
2. Frontend writes pixel JSON to the user's homeserver at `/pub/pubky-canva/pixels/<id>`
3. Frontend calls `PUT /api/ingest/<pk>` to register the user with the indexer
4. Indexer discovers the user's homeserver via Pkarr/DHT, polls for new pixel events
5. Indexer validates placement (credits, bounds, anti-cheat) and updates SQLite
6. Real-time updates pushed to all connected browsers via SSE

## Features

- **PICO-8 palette** — 16 curated colors
- **Credit system** — 10 pixel credits max, regenerates 1 every 10 minutes
- **Anti-cheat** — indexer enforces credit limits server-side regardless of client
- **Auto-resize** — canvas doubles when 100% filled and 50% overwritten by different users
- **Pixel attribution** — click any pixel to see who placed it, with link to their Pubky profile
- **Real-time** — SSE stream for live canvas updates
- **Two auth methods** — Pubky Ring QR code, recovery file (.pkarr)

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [tmux](https://github.com/tmux/tmux) for the startup script
- A [Pubky Ring](https://pubky.tech/ring) account (for authentication)

## Quick Start

```bash
# Start both indexer and frontend in a tmux session:
./start.sh
```

This creates a tmux session `canva-dev` with 2 windows:
1. **indexer** — canva-indexer Rust backend
2. **frontend** — Next.js dev server

Use `Ctrl+b` then `1`/`2` to switch windows. `Ctrl+b d` to detach.

Or start each component manually:

```bash
# Terminal 1: Start the indexer
cd canva-indexer
cargo run

# Terminal 2: Start the frontend
cd frontend
npm install
npm run dev
```

Then open http://localhost:3000, click **Connect**, and scan the QR code with Pubky Ring.

## Configuration

### Indexer (`canva-indexer/config.toml`)

| Key | Default | Description |
|-----|---------|-------------|
| `server.listen` | `127.0.0.1:3001` | API listen address |
| `watcher.poll_interval_ms` | `5000` | How often to poll homeservers |
| `canvas.initial_size` | `16` | Starting canvas dimensions (NxN) |
| `canvas.max_credits` | `10` | Max pixel credits per user |
| `canvas.credit_regen_seconds` | `600` | Seconds to regenerate 1 credit |

### Frontend (`frontend/.env.local`)

| Variable | Default | Description |
|----------|---------|-------------|
| `NEXT_PUBLIC_INDEXER_URL` | `http://localhost:3001` | Indexer API URL |
| `NEXT_PUBLIC_RELAY_URL` | `https://httprelay.pubky.app/link/` | HTTP relay for Pubky Ring auth |
| `NEXT_PUBLIC_PUBKY_APP_URL` | `https://pubky.app` | Pubky app base URL for profile links |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/canvas` | Full canvas state (all pixels) |
| `GET` | `/api/canvas/meta` | Canvas size, fill stats |
| `GET` | `/api/canvas/pixel/{x}/{y}` | Single pixel info + history |
| `GET` | `/api/canvas/palette` | PICO-8 color palette |
| `GET` | `/api/events` | SSE stream (pixel, resize events) |
| `PUT` | `/api/ingest/{public_key}` | Register user for indexing |
| `GET` | `/api/user/{public_key}/credits` | User's available credits |

## Data Model

Pixels are stored on the user's homeserver at:

```
/pub/pubky-canva/pixels/<timestamp_id>
```

Where `timestamp_id` is a Crockford Base32 encoding of unix microseconds. Pixel JSON:

```json
{
  "x": 7,
  "y": 3,
  "color": 8
}
```

Color is an index (0-15) into the PICO-8 palette.
