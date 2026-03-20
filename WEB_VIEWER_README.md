# Asciiquarium WebSocket Viewer - Quick Start

## Build & Run

```bash
# Build the web server binary
cargo build --features web --bin web_server

# Run the server
./target/debug/web_server

# Or with a custom port (if 3000 is busy)
PORT=3001 ./target/debug/web_server
```

## Expected Output

```
✓ Web server listening on http://127.0.0.1:3000
  Open in browser: http://localhost:3000
  To use a different port, set: export PORT=3001
  Then run: cargo run --features web --bin web_server
```

## Access the Viewer

Open your browser to: **http://localhost:3000**

You should see:
1. Initially: "Connecting..." status (orange)
2. After 1-2 seconds: "Connected" status (green) with animated colorized ASCII aquarium
3. Grid size controls, Add Fish, Add Crab, and Reset buttons

## Stop the Server

Press `Ctrl+C` in the terminal running the server, or:

```bash
killall web_server
```

## Features

- 30 FPS smooth animation
- Colorized ASCII (water, seaweed, bubbles, crab with distinct colors)
- Interactive controls (add creatures, resize grid, reset)
- Multi-client support (multiple browser tabs sync automatically)
- Deterministic physics (reproducible behavior)
- Auto-reconnect on disconnection

## Troubleshooting

**"Connecting..." stays visible:**
- Port 3000 might be in use. Try: `PORT=3001 ./target/debug/web_server`

**Page won't load:**
- Check server output for error messages
- Verify port is actually bound: `lsof -i :3000`

**Colors not showing:**
- Check browser console (F12) for JavaScript errors
- Ensure you're using a modern browser (Chrome, Firefox, Safari, Edge)

## Architecture

- **Backend**: Axum (async HTTP + WebSocket)
- **Frontend**: Vanilla HTML/CSS/JavaScript
- **Animation**: 30 FPS WebSocket frame broadcast
- **Simulation**: Deterministic physics (no randomness)
- **Colors**: CSS class-based (`.water`, `.seaweed`, `.bubble`, `.crab`)

See `WEB_SERVER_SETUP.md` for complete technical documentation.
