#[cfg(feature = "web")]
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::Html,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use asciiquarium_rust::{
    update_aquarium, get_all_fish_assets, render_aquarium_to_string, 
    widgets::asciiquarium::FishBehavior, FishInstance, AquariumState
};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ColorPalette {
    water: String,
    water_trail: String,
    seaweed: String,
    bubble: String,
    text: String,
    crab: String,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            water: "#78b4ff".to_string(),   // 120, 180, 255
            water_trail: "#78b4ff".to_string(),
            seaweed: "#3cb478".to_string(),  // 60, 180, 120
            bubble: "#c8e6ff".to_string(),   // 200, 230, 255
            text: "#b4dcff".to_string(),     // 180, 220, 255
            crab: "#dc5050".to_string(),     // 220, 80, 80
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FrameMessage {
    grid_html: String,  // HTML with color spans
    tick: u64,
    width: usize,
    height: usize,
    palette: ColorPalette,
    enable_color: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command {
    AddFish,
    AddCrab,
    #[serde(rename = "set_grid_size")]
    SetGridSize { width: usize, height: usize },
    Reset,
}

// Spawn deterministic random fish
fn spawn_random_fish(state: &mut AquariumState, asset_count: usize) {
    if asset_count == 0 {
        return;
    }
    let mut s: u64 = 0x1234_5678_9ABC_DEF0u64 ^ (asset_count as u64);
    let mut next_u32 = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        (s >> 32) as u32
    };

    let idx = (next_u32() as usize) % asset_count;
    let max_x = if state.size.0 > 0 { state.size.0 - 1 } else { 0 };
    let max_y = if state.size.1 > 0 { state.size.1 - 1 } else { 0 };
    let x = (next_u32() as usize % (max_x + 1)) as f32;
    let y = (next_u32() as usize % (max_y + 1)) as f32;

    let r = (next_u32() as f32) / (u32::MAX as f32);
    let speed = 2.5_f32 + r * (22.5_f32 - 2.5_f32);
    let dir = if (next_u32() & 1) == 0 { -1.0 } else { 1.0 };
    let vx = dir * speed;
    let mut vy = -0.6_f32 + (next_u32() as f32) / (u32::MAX as f32) * 1.2_f32;
    if vy.abs() < 0.05 {
        vy = 0.0;
    }

    state.fishes.push(FishInstance {
        fish_art_index: idx,
        position: (x, y),
        velocity: (vx, vy),
    });
    state.fish_behaviors.push(FishBehavior::Normal);
}

fn spawn_crab(state: &mut AquariumState, assets: &[asciiquarium_rust::FishArt]) {
    if assets.is_empty() {
        return;
    }
    let idx = assets
        .iter()
        .position(|a| a.art.contains("__^_^__") || a.art.contains("o o"))
        .unwrap_or(assets.len() - 1);

    let art = &assets[idx];
    let fw = art.width;
    let fh = art.height;
    let x = if state.size.0 > fw { (state.size.0 - fw) / 2 } else { 0 };
    let y = if state.size.1 > fh { state.size.1 - fh } else { 0 };

    let dir = if (state.tick + state.fishes.len() as u64) % 2 == 0 { 1.0 } else { -1.0 };
    let speed = 6.0_f32 * dir;

    state.fishes.push(FishInstance {
        fish_art_index: idx,
        position: (x as f32, y as f32),
        velocity: (speed, 0.0),
    });
    state.fish_behaviors.push(FishBehavior::Normal);
}

fn colorize_grid(grid: &str, width: usize, height: usize, _palette: &ColorPalette, enable_color: bool, state: &AquariumState, assets: &[asciiquarium_rust::FishArt]) -> String {
    if !enable_color {
        // Escape HTML entities and return plain grid
        return grid
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;");
    }

    // Build crab mask to identify which cells contain crab
    let mut crab_mask = vec![false; width.saturating_mul(height)];
    let crab_idx = assets
        .iter()
        .position(|a| a.art.contains("__^_^__") || a.art.contains("o o"));
    
    if let Some(crab_idx) = crab_idx {
        for fish in &state.fishes {
            if fish.fish_art_index != crab_idx {
                continue;
            }
            let art = match assets.get(fish.fish_art_index) {
                Some(a) => a,
                None => continue,
            };
            let x0 = fish.position.0.floor() as isize;
            let y0 = fish.position.1.floor() as isize;
            let mirror = if art.no_mirror {
                false
            } else {
                (fish.velocity.0 < 0.0 && art.prefers_right)
                    || (fish.velocity.0 > 0.0 && !art.prefers_right)
            };
            let lines_iter = if mirror {
                art.mirrored.lines().collect::<Vec<_>>()
            } else {
                art.art.lines().collect::<Vec<_>>()
            };
            for (dy, raw_line) in lines_iter.iter().enumerate() {
                let y = y0 + dy as isize;
                if y < 0 || y >= height as isize {
                    continue;
                }
                for (dx, ch) in raw_line.chars().enumerate() {
                    if ch == ' ' || ch == '?' {
                        continue;
                    }
                    let x = x0 + dx as isize;
                    if x < 0 || x >= width as isize {
                        continue;
                    }
                    let idx = (y as usize).saturating_mul(width) + (x as usize);
                    if idx < crab_mask.len() {
                        crab_mask[idx] = true;
                    }
                }
            }
        }
    }

    let mut html = String::new();
    let mut row_idx: usize = 0;
    
    for line in grid.lines() {
        let mut col_idx = 0;
        let mut run_buf = String::new();
        let mut run_class: Option<&str> = None;

        let push_run = |html: &mut String, buf: &mut String, class: Option<&str>| {
            if !buf.is_empty() {
                if let Some(cls) = class {
                    html.push_str(&format!("<span class=\"{}\">{}</span>", cls, buf));
                } else {
                    html.push_str(&buf.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"));
                }
                buf.clear();
            }
        };

        for ch in line.chars() {
            let cell_idx = row_idx.saturating_mul(width) + col_idx;
            let class = if cell_idx < crab_mask.len() && crab_mask[cell_idx] {
                Some("crab")
            } else {
                match ch {
                    '~' | '^' => Some("water"),
                    '(' | ')' => Some("seaweed"),
                    '.' => Some("bubble"),
                    '?' => Some("trail"),
                    _ => None,
                }
            };

            match run_class {
                Some(c) if c == class.unwrap_or("default") => run_buf.push(ch),
                Some(_) => {
                    push_run(&mut html, &mut run_buf, run_class);
                    run_class = class;
                    run_buf.push(ch);
                }
                None => {
                    run_class = class;
                    run_buf.push(ch);
                }
            }
            col_idx += 1;
        }

        if !run_buf.is_empty() {
            push_run(&mut html, &mut run_buf, run_class);
        }

        if row_idx + 1 < height {
            html.push('\n');
        }
        row_idx += 1;
    }

    html
}

type SharedState = Arc<Mutex<AquariumState>>;
type BroadcastTx = broadcast::Sender<FrameMessage>;


async fn handle_socket(
    socket: WebSocket,
    state: SharedState,
    tx: BroadcastTx,
    assets: Vec<asciiquarium_rust::FishArt>,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Task to send frames to client
    let sender_handle = tokio::spawn(async move {
        while let Ok(frame) = rx.recv().await {
            let msg = serde_json::to_vec(&frame).unwrap_or_default();
            if sender.send(Message::Binary(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming commands
    while let Some(Ok(msg)) = receiver.next().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            if let Ok(cmd) = serde_json::from_str::<Command>(&text) {
                let mut aquarium = state.lock().await;
                match cmd {
                    Command::AddFish => {
                        spawn_random_fish(&mut aquarium, assets.len());
                    }
                    Command::AddCrab => {
                        spawn_crab(&mut aquarium, &assets);
                    }
                    Command::SetGridSize { width, height } => {
                        aquarium.size = (width, height);
                    }
                    Command::Reset => {
                        aquarium.fishes.clear();
                        aquarium.fish_behaviors.clear();
                        aquarium.bubbles.clear();
                        for _ in 0..6 {
                            spawn_random_fish(&mut aquarium, assets.len());
                        }
                        spawn_crab(&mut aquarium, &assets);
                    }
                }
            }
        }
    }

    sender_handle.abort();
}

#[cfg(feature = "web")]
#[tokio::main]
async fn main() {
    let assets = get_all_fish_assets();
    let size = (80usize, 24usize);

    let mut initial_state = AquariumState {
        size,
        fishes: Vec::new(),
        ..Default::default()
    };

    for _ in 0..6 {
        spawn_random_fish(&mut initial_state, assets.len());
    }
    spawn_crab(&mut initial_state, &assets);

    let state = SharedState::new(Mutex::new(initial_state));
    let (tx, _) = broadcast::channel::<FrameMessage>(10);

    let state_clone = Arc::clone(&state);
    let tx_clone = tx.clone();
    let assets_clone = assets.clone();

    // Spawn frame update task
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(33));
        let palette = ColorPalette::default();
        let enable_color = true;
        
        loop {
            interval.tick().await;

            let mut aquarium = state_clone.lock().await;
            update_aquarium(&mut aquarium, &assets_clone);

            let grid = render_aquarium_to_string(&aquarium, &assets_clone);
            let grid_html = colorize_grid(&grid, aquarium.size.0, aquarium.size.1, &palette, enable_color, &aquarium, &assets_clone);
            
            let frame = FrameMessage {
                grid_html,
                tick: aquarium.tick,
                width: aquarium.size.0,
                height: aquarium.size.1,
                palette: palette.clone(),
                enable_color,
            };

            let _ = tx_clone.send(frame);
        }
    });

    // HTML handler
    async fn index() -> Html<&'static str> {
        Html(include_str!("../../assets/index.html"))
    }

    // WebSocket handler
    async fn ws_handler(
        ws: WebSocketUpgrade,
        State((state, tx)): State<(SharedState, BroadcastTx)>,
    ) -> impl axum::response::IntoResponse {
        let assets = get_all_fish_assets();
        ws.on_upgrade(move |socket| handle_socket(socket, state, tx, assets))
    }

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler))
        .with_state((state, tx))
        .layer(tower_http::cors::CorsLayer::permissive());

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("127.0.0.1:{}", port);
    
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", bind_addr, e);
            eprintln!("Try a different port: PORT=3001 (or export PORT=3001 before running)");
            std::process::exit(1);
        }
    };

    println!("✓ Web server listening on http://127.0.0.1:{}", port);
    println!("  Open in browser: http://localhost:{}", port);
    println!("  To use a different port, set: export PORT=3001");
    println!("  Then run: cargo run --features web --bin web_server");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

#[cfg(not(feature = "web"))]
fn main() {
    eprintln!("This binary requires the 'web' feature. Run with: cargo run --features web --bin web_server");
}

