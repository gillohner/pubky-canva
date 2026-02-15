#!/bin/bash

# Script to start Pubky Canva development environment
# Usage: ./start.sh

set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting Pubky Canva${NC}"
echo "====================="
echo ""

# Check prerequisites
if ! command -v tmux &> /dev/null; then
    echo -e "${RED}Error: tmux is not installed${NC}"
    echo "Install it with: sudo pacman -S tmux"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found${NC}"
    echo "Install from https://rustup.rs/"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: Node.js not found${NC}"
    echo "Install from https://nodejs.org/"
    exit 1
fi

# Install frontend deps if needed
if [ ! -d "$ROOT/frontend/node_modules" ]; then
    echo -e "${YELLOW}Installing frontend dependencies...${NC}"
    (cd "$ROOT/frontend" && npm install)
fi

# Kill existing tmux session if it exists
if tmux has-session -t canva-dev 2>/dev/null; then
    echo -e "${YELLOW}Killing existing canva-dev session...${NC}"
    tmux kill-session -t canva-dev
fi

# Create new tmux session
echo -e "${GREEN}Creating tmux session 'canva-dev'...${NC}"
tmux new-session -d -s canva-dev -n "indexer"

# Window 1: Canva Indexer
echo -e "${GREEN}[1/2] Starting Canva Indexer...${NC}"
tmux send-keys -t canva-dev:indexer "cd $ROOT/canva-indexer && cargo run" C-m

# Window 2: Frontend
echo -e "${GREEN}[2/2] Starting Frontend...${NC}"
tmux new-window -t canva-dev -n "frontend"
tmux send-keys -t canva-dev:frontend "cd $ROOT/frontend && npm run dev" C-m

# Select indexer window
tmux select-window -t canva-dev:indexer

echo ""
echo -e "${GREEN}Environment started in tmux session 'canva-dev'${NC}"
echo ""
echo "Services:"
echo "  Indexer API:  http://localhost:3001"
echo "  Frontend:     http://localhost:3000"
echo ""
echo "Attaching to tmux session..."
echo "  - Use Ctrl+b then 1/2 to switch between windows"
echo "  - Use Ctrl+b then d to detach (services keep running)"
echo "  - Use 'tmux attach -t canva-dev' to reattach"
echo "  - Use 'tmux kill-session -t canva-dev' to stop everything"
echo ""
sleep 1

# Attach to the session
tmux attach-session -t canva-dev
