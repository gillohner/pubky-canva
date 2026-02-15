# Pubky Canva - Implementation Plan

A collaborative pixel canvas (like Reddit r/place) built on the Pubky protocol.

## Architecture Overview

```
User Browser (Next.js)
  ├── Writes pixel events to homeserver via @synonymdev/pubky SDK
  ├── Reads canvas state from canva-indexer REST API
  └── Receives real-time updates via SSE (Server-Sent Events)

canva-indexer (Rust binary)
  ├── Polls homeservers for pixel events
  ├── Validates placement rules (cooldown, bounds)
  ├── Stores state in SQLite
  ├── Serves REST API (Axum)
  └── Pushes SSE updates to connected clients
```

## Data Model

### Pixel Event (on user's homeserver)

**Path:** `/pub/pubky-canva/pixels/<timestamp_id>`

```json
{
  "x": 7,
  "y": 3,
  "color": 5
}
```

- `x`, `y`: u32 coordinates (0-indexed, must be within current canvas bounds)
- `color`: u8 palette index (0-15) into the PICO-8 palette
- ID: Timestamp-based (Crockford Base32 of unix microseconds, same as pubky-app-specs)

This is intentionally minimal. The timestamp is encoded in the ID itself, so no separate timestamp field is needed. The indexer validates and orders by the ID's embedded timestamp.

### Color Palette: PICO-8

16 colors with excellent coverage for pixel art. Stored as index (0-15) on homeserver, resolved to hex by both indexer and frontend.

| Index | Hex       | Name         |
|-------|-----------|--------------|
| 0     | `#000000` | Black        |
| 1     | `#1D2B53` | Dark Blue    |
| 2     | `#7E2553` | Dark Purple  |
| 3     | `#008751` | Dark Green   |
| 4     | `#AB5236` | Brown        |
| 5     | `#5F574F` | Dark Grey    |
| 6     | `#C2C3C7` | Light Grey   |
| 7     | `#FFF1E8` | White        |
| 8     | `#FF004D` | Red          |
| 9     | `#FFA300` | Orange       |
| 10    | `#FFEC27` | Yellow       |
| 11    | `#00E436` | Green        |
| 12    | `#29ADFF` | Blue         |
| 13    | `#83769C` | Lavender     |
| 14    | `#FF77A8` | Pink         |
| 15    | `#FFCCAA` | Peach        |

### Credit System

Users have a **pixel credit** balance instead of a simple cooldown:
- **Max credits**: 10 (configurable)
- **Regeneration rate**: 1 credit every 10 minutes (600 seconds, configurable)
- Placing a pixel costs 1 credit
- Credits regenerate passively up to the max
- A new user starts with 10 credits
- The indexer enforces this: it tracks each user's last N placements and calculates available credits at the time of each event's timestamp

### No changes to pubky-app-specs

We use a custom namespace (`pubky-canva`) and define our own simple model in the indexer crate. The pixel struct is too app-specific and simple to warrant adding to the shared specs repo. We'll define a small `CanvaPixel` struct with `Serialize`/`Deserialize` in the indexer for validation.

---

## Part 1: Indexer (`canva-indexer/`)

### Crate Structure

```
canva-indexer/
  Cargo.toml
  config.toml              # default config
  src/
    main.rs                 # CLI entry, starts watcher + API
    config.rs               # TOML config (canvas_size, cooldown, db path, homeserver, etc.)
    db.rs                   # SQLite schema + queries (rusqlite)
    watcher.rs              # Polls homeservers, processes pixel events
    api.rs                  # Axum REST + SSE endpoints
    pixel.rs                # CanvaPixel struct, validation, ID parsing
    canvas.rs               # Canvas state logic (bounds, resize check)
```

### Config (`config.toml`)

```toml
[server]
listen = "127.0.0.1:3001"

[watcher]
testnet = true
poll_interval_ms = 5000
events_limit = 50

[canvas]
initial_size = 16
max_credits = 10
credit_regen_seconds = 600    # 10 minutes per credit

[database]
path = "canva.db"
```

No fixed homeserver is configured. When a user is ingested via `PUT /api/ingest/:public_key`, the indexer uses the Pubky SDK to resolve their homeserver via Pkarr/DHT lookup and stores it in the `users` table. The watcher then polls all discovered homeservers.

### SQLite Schema

```sql
-- Users the indexer is watching
CREATE TABLE users (
  public_key TEXT PRIMARY KEY,
  homeserver TEXT NOT NULL,               -- homeserver public key (discovered via Pkarr)
  cursor TEXT NOT NULL DEFAULT '',        -- homeserver event cursor for this user
  created_at INTEGER NOT NULL             -- unix timestamp
);

-- Every valid pixel placement (history)
CREATE TABLE pixel_events (
  id TEXT PRIMARY KEY,                     -- timestamp_id from homeserver path
  user_pk TEXT NOT NULL,
  x INTEGER NOT NULL,
  y INTEGER NOT NULL,
  color INTEGER NOT NULL,                  -- palette index 0-15
  placed_at INTEGER NOT NULL,             -- unix microseconds (from ID)
  FOREIGN KEY (user_pk) REFERENCES users(public_key)
);

-- Current canvas state (materialized for fast reads)
CREATE TABLE canvas_state (
  x INTEGER NOT NULL,
  y INTEGER NOT NULL,
  color INTEGER NOT NULL,                  -- palette index 0-15
  user_pk TEXT NOT NULL,
  placed_at INTEGER NOT NULL,
  overwritten_count INTEGER NOT NULL DEFAULT 0,  -- times this cell was painted over
  PRIMARY KEY (x, y)
);

-- Canvas metadata
CREATE TABLE canvas_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
-- Initial rows: ('size', '16'), ('total_pixels', '256'), ('filled_count', '0'), ('overwritten_count', '0')
```

### Watcher Logic

1. On startup, load known users from `users` table (each user has their own homeserver)
2. Every `poll_interval_ms`:
   a. Group users by homeserver, poll each homeserver's events endpoint using per-user cursor
   b. For each `PUT` event matching `/pub/pubky-canva/pixels/<id>`:
      - Fetch the pixel JSON from homeserver
      - Deserialize + validate (color index 0-15, coords within current canvas size)
      - Parse timestamp from ID
      - Check credits: query `pixel_events` for user's recent placements, calculate available credits at the event's timestamp. Reject if credits <= 0
      - If valid: insert into `pixel_events`, upsert `canvas_state`, update counters
      - Broadcast via SSE channel
   c. For `DEL` events: ignore (pixels can't be un-placed)
   d. Update user's cursor in DB
3. After processing all users, run resize check

### Credit Calculation (indexer-side)

To determine a user's available credits at a given timestamp `T`:
1. Fetch user's accepted pixel_events ordered by `placed_at` descending
2. Walk backwards: for each placement, check if enough time has passed for credits to regenerate
3. Formula: `available = min(max_credits, max_credits - active_placements + regenerated_credits)`
   - Count how many of the user's last `max_credits` placements fall within `(T - max_credits * regen_interval, T]`
   - Each placement outside that window has fully regenerated

This ensures even if a user writes many events to their homeserver rapidly, only up to 10 will be accepted, properly spaced by regeneration.

### Registration (Ingest) Endpoint

`PUT /api/ingest/:public_key` — called by frontend after login.

The indexer:
1. Uses `pubky.get_homeserver_of(public_key)` to discover the user's homeserver via Pkarr/DHT
2. Stores the user + homeserver in the `users` table
3. The watcher picks them up on the next poll cycle and begins monitoring their homeserver

### Canvas Resize Logic

After each full poll cycle, check:
1. `filled_count >= total_pixels` (every cell has been painted at least once)
2. `overwritten_count >= total_pixels * 0.5` (50% of cells have been painted by a different user than the first painter)

If both true:
- Double canvas size (16 -> 32 -> 64 -> ...)
- Update `canvas_meta` (size, total_pixels, reset filled_count and overwritten_count)
- The existing pixels remain; new area is empty
- Any pixel events that were written before the resize for coordinates outside the old bounds are **ignored** (anti-cheat: users can't pre-place pixels in future canvas areas)

### Anti-Cheat Rules (enforced by indexer)

1. **Credit enforcement**: A user's pixel is only accepted if they have >= 1 credit at the event's timestamp (calculated from their placement history, not client-reported)
2. **Bounds**: Pixel coordinates must be within `[0, current_size)` at the time of indexing
3. **No future timestamps**: Reject IDs with timestamps > now + 2 minutes (clock skew tolerance)
4. **No pre-placed out-of-bounds**: Pixels placed at coordinates beyond current canvas size are discarded permanently (even after resize)
5. **Valid palette**: Color index must be 0-15 (PICO-8 palette)
6. **Sequential processing**: Events are processed in order per user, so credit checks are consistent

### REST API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/canvas` | Full canvas state (size + all placed pixels) |
| `GET` | `/api/canvas/pixel/:x/:y` | Single pixel info (color, user, history) |
| `GET` | `/api/canvas/meta` | Canvas metadata (size, fill stats) |
| `GET` | `/api/events` | SSE stream of real-time pixel updates |
| `PUT` | `/api/ingest/:public_key` | Register user for watching |
| `GET` | `/api/user/:public_key/credits` | Current credits + time until next credit regenerates |

### SSE Event Format

```
event: pixel
data: {"x":7,"y":3,"color":5,"user":"<z32_pubkey>","placed_at":1739600000000000}

event: resize
data: {"new_size":32,"old_size":16}
```

### Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.34", features = ["bundled"] }
toml = "0.8"
pubky = { git = "https://github.com/pubky/pubky-core" }
tracing = "0.1"
tracing-subscriber = "0.3"
data-encoding = "2"
tokio-stream = "0.1"
```

---

## Part 2: Frontend (`frontend/`)

### Project Setup

```
frontend/
  package.json
  next.config.ts
  tsconfig.json
  tailwind.config.ts        # if needed (v4 uses CSS)
  app/
    layout.tsx               # Providers (QueryProvider, AuthProvider)
    page.tsx                 # Main canvas view
    globals.css              # Tailwind v4 + theme
  components/
    canvas/
      pixel-canvas.tsx       # The main 16x16+ canvas grid (renders current state)
      pixel-cell.tsx         # Individual pixel (clickable)
      color-picker.tsx       # Color selection palette
      pixel-info.tsx         # Popup showing who placed a pixel
    auth/
      login-dialog.tsx       # Login modal (recovery file for now, QR later)
      auth-guard.tsx         # Wrap actions requiring auth
    layout/
      header.tsx             # App bar with login button
      canvas-stats.tsx       # Fill %, size, credits remaining
  hooks/
    use-canvas.ts            # Fetch canvas state from indexer API
    use-canvas-sse.ts        # SSE subscription for real-time updates
    use-place-pixel.ts       # Mutation: write pixel to homeserver
    use-credits.ts           # Track user's pixel credit balance + regen timer
    use-auth.ts              # Auth state hook
  lib/
    pubky/
      client.ts              # PubkyClient singleton (same pattern as eventky)
      pixels.ts              # Write pixel to homeserver
    api/
      client.ts              # Axios client for indexer API
      canvas.ts              # Canvas API calls
    config.ts                # Environment config
  stores/
    auth-store.ts            # Zustand auth state (persisted)
  types/
    canvas.ts                # PixelData, CanvasState, CanvasMeta
    auth.ts                  # AuthData
```

### Key UI Components

#### PixelCanvas (`components/canvas/pixel-canvas.tsx`)

- Renders an N x N grid using CSS Grid
- Each cell is a `<PixelCell>` component
- Colors cells based on canvas state from API
- Empty cells shown as a subtle grid pattern (dark background)
- Scales to fit viewport (each cell is `min(calc(100vw / size), calc(80vh / size))`)
- Click a filled pixel -> show `<PixelInfo>` popover
- Click an empty pixel (or any pixel while authenticated) -> if not on cooldown, show `<ColorPicker>`

#### ColorPicker (`components/canvas/color-picker.tsx`)

- PICO-8 palette: 16 colors rendered as a 4x4 grid of swatches
- Click a color -> places the pixel (calls `usePlacePixel` mutation)
- Shows remaining credits + countdown to next credit regeneration

#### PixelInfo (`components/canvas/pixel-info.tsx`)

- Shows: color swatch, user's z32 public key (truncated), placement time
- Links to `https://pubky.app/profile/<pubky>` (clickable)
- Small floating panel / popover anchored to the clicked pixel

### Auth Flow

Following eventky patterns:
1. User clicks "Connect" in header
2. Login dialog opens with recovery file upload (+ passphrase) for v1
3. On success: session persisted to localStorage via zustand store
4. `ingestUserIntoNexus()` equivalent calls `PUT /api/ingest/:pk` on our indexer
5. Canvas now shows color picker on pixel click + credit counter

### Data Flow: Placing a Pixel

1. User clicks empty/any pixel on canvas -> color picker appears
2. User selects color
3. Frontend checks local credits > 0 (UI guard, not authoritative)
4. `usePlacePixel` mutation:
   a. Generate timestamp ID (same algo as pubky-app-specs: unix microseconds -> crockford base32)
   b. Write to homeserver: `session.storage.putJson("/pub/pubky-canva/pixels/<id>", { x, y, color })`
   c. Optimistic update: immediately show pixel on canvas locally, decrement local credits
5. Indexer picks it up on next poll cycle, validates, broadcasts via SSE
6. SSE event arrives -> canvas updates (confirms optimistic update or corrects if rejected)

### Data Flow: Viewing Canvas

1. On page load: `GET /api/canvas` fetches full state
2. Connect to `GET /api/events` SSE stream
3. Each SSE `pixel` event updates the local canvas state in-place
4. Each SSE `resize` event triggers a full canvas refetch

### Real-Time Updates (SSE)

Using `EventSource` browser API:
```typescript
const useCanvasSSE = (onPixel, onResize) => {
  useEffect(() => {
    const es = new EventSource(`${config.indexerUrl}/api/events`);
    es.addEventListener('pixel', (e) => onPixel(JSON.parse(e.data)));
    es.addEventListener('resize', (e) => onResize(JSON.parse(e.data)));
    return () => es.close();
  }, []);
};
```

### Tech Stack

- Next.js 15 (App Router)
- `@synonymdev/pubky` v0.6.0
- TanStack Query v5 (canvas state)
- Zustand v5 (auth state)
- Tailwind CSS v4
- shadcn/ui (dialog, popover, button)
- sonner (toasts)
- lucide-react (icons)

---

## Implementation Order

### Phase 1: Indexer Foundation
1. Initialize Cargo project with dependencies
2. Implement `config.rs` (TOML parsing)
3. Implement `pixel.rs` (CanvaPixel struct, validation, ID parsing)
4. Implement `db.rs` (SQLite schema, migrations, queries)
5. Implement `canvas.rs` (bounds checking, resize logic)
6. Implement `watcher.rs` (homeserver polling, event processing, cooldown enforcement)
7. Implement `api.rs` (Axum REST endpoints + SSE)
8. Implement `main.rs` (CLI, start watcher + API with tokio)
9. Test with `pubky-testnet`

### Phase 2: Frontend Foundation
10. Initialize Next.js project with dependencies
11. Set up `lib/config.ts`, `lib/pubky/client.ts`, `lib/api/client.ts`
12. Implement auth flow (`stores/auth-store.ts`, `components/auth/login-dialog.tsx`)
13. Implement canvas data layer (`hooks/use-canvas.ts`, `hooks/use-canvas-sse.ts`, `types/canvas.ts`)
14. Implement `PixelCanvas` + `PixelCell` components (read-only view)
15. Implement `ColorPicker` + `usePlacePixel` mutation
16. Implement `PixelInfo` popover (who placed it, link to pubky.app)
17. Implement credit tracker (`hooks/use-credits.ts`, UI credit counter + regen countdown)
18. Implement header with login button + canvas stats

### Phase 3: Polish & Integration
19. End-to-end testing with pubky-testnet
20. Canvas resize visual handling (smooth transition)
21. Mobile-responsive canvas (pinch to zoom, touch to place)
22. Error handling and edge cases

---

## Config / Environment Variables

### Indexer (`config.toml`)
- `server.listen` — API bind address (default `127.0.0.1:3001`)
- `watcher.testnet` — use pubky testnet (default `true`)
- `watcher.poll_interval_ms` — polling frequency (default `5000`)
- `canvas.initial_size` — starting canvas size (default `16`)
- `canvas.max_credits` — max pixel credits per user (default `10`)
- `canvas.credit_regen_seconds` — seconds to regenerate 1 credit (default `600`)
- `database.path` — SQLite file path (default `canva.db`)

### Frontend (`.env.local`)
- `NEXT_PUBLIC_INDEXER_URL` — canva-indexer API URL (default `http://localhost:3001`)
- `NEXT_PUBLIC_ENV` — `testnet` | `production`
- `NEXT_PUBLIC_HOMESERVER_PK` — homeserver z32 public key
- `NEXT_PUBLIC_PUBKY_APP_URL` — pubky.app base URL for profile links
