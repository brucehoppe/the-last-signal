// A release build on Windows is a window, not a console program.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
use macroquad::prelude::*;
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, TryRecvError},
};
use the_last_signal::{
    ai,
    core::{
        Awareness, Difficulty, EnemyKind, Faction, Game, Module, Outcome, PickupKind, Pos, Signal,
        TerminalState, Tile, ANALYZE_COST, CHALLENGES, FLOORS, FLOOR_NAMES, FRAGMENT_BASE, HEIGHT,
        PULSE_COST, RECORD_AUTHORS, SCAN_COST, WIDTH,
    },
    profile, save,
};

/// "Test all" skips models larger than this (loading many big models at once can
/// exhaust RAM); test those one at a time instead.
const TEST_ALL_MAX_GB: f32 = 16.;
const CONSOLE_ROWS: usize = 6;
/// Transcript columns that fit the ECHO panel at 17 px in the 0.6em-wide font.
const CHAT_COLS: usize = 33;
const CHAT_ROWS: usize = 17;
const INK: Color = Color::new(0.035, 0.055, 0.075, 1.);
const PANEL: Color = Color::new(0.065, 0.09, 0.115, 1.);
const LIGHT: Color = Color::new(0.86, 0.89, 0.85, 1.);
const MUTED: Color = Color::new(0.43, 0.54, 0.58, 1.);
const TEAL: Color = Color::new(0.33, 0.86, 0.72, 1.);
const AMBER: Color = Color::new(0.96, 0.68, 0.36, 1.);
const CORAL: Color = Color::new(0.99, 0.40, 0.34, 1.);
fn config() -> Conf {
    Conf {
        window_title: "The Last Signal | Expedition 01".into(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}
/// True in the browser build: no disk, no Ollama; ECHO is a built-in script.
const DEMO: bool = cfg!(target_arch = "wasm32");
fn seed() -> u64 {
    // miniquad's clock works natively and in the browser; SystemTime panics on wasm.
    (macroquad::miniquad::date::now() * 1000.) as u64
}
thread_local! {
    static FONT: RefCell<Option<Font>> = const { RefCell::new(None) };
}
fn init_font() {
    // JetBrains Mono (SIL OFL, see assets/fonts/OFL.txt): far more legible than
    // macroquad's built-in bitmap font, and monospaced so wrap widths hold.
    if let Ok(font) =
        load_ttf_font_from_bytes(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"))
    {
        FONT.with(|f| *f.borrow_mut() = Some(font));
    }
}
/// How many real pixels one design pixel covers. Glyphs are rasterized at this
/// scale, so text stays sharp on retina screens and in enlarged windows.
fn text_scale() -> f32 {
    (((screen_width() * screen_dpi_scale() / 1280.) * 4.).round() / 4.).clamp(1., 4.)
}
fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    let k = text_scale();
    FONT.with(|f| {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: f.borrow().as_ref(),
                font_size: (size * k).round() as u16,
                font_scale: 1. / k,
                color,
                ..Default::default()
            },
        )
    });
}
fn lines(s: &str, width: usize) -> Vec<String> {
    the_last_signal::wrap::wrap(s, width)
}
/// `width` is in legacy columns (tuned for a 0.5em-wide font); JetBrains Mono is
/// 0.6em wide, so scale to keep every line inside its panel.
fn wrapped(s: &str, x: f32, y: f32, width: usize, size: f32, color: Color, max: usize) {
    for (i, l) in lines(s, width * 5 / 6).iter().take(max).enumerate() {
        text(l, x, y + i as f32 * (size + 3.), size, color);
    }
}
fn button(label: &str, r: Rect, enabled: bool) -> bool {
    let (mx, my) = mouse_position();
    let hover = r.contains(vec2(
        mx * 1280. / screen_width(),
        my * 800. / screen_height(),
    ));
    draw_rectangle(
        r.x,
        r.y,
        r.w,
        r.h,
        if hover && enabled {
            Color::new(0.12, 0.25, 0.25, 1.)
        } else {
            PANEL
        },
    );
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1., if enabled { TEAL } else { MUTED });
    text(
        label,
        r.x + 12.,
        r.y + 23.,
        18.,
        if enabled { LIGHT } else { MUTED },
    );
    enabled && hover && is_mouse_button_pressed(MouseButton::Left)
}
/// A smaller button for ECHO's suggested questions.
fn chip(label: &str, r: Rect, enabled: bool) -> bool {
    let (mx, my) = mouse_position();
    let hover = enabled
        && r.contains(vec2(
            mx * 1280. / screen_width(),
            my * 800. / screen_height(),
        ));
    draw_rectangle(
        r.x,
        r.y,
        r.w,
        r.h,
        if hover {
            Color::new(0.12, 0.25, 0.25, 1.)
        } else {
            INK
        },
    );
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1., if hover { TEAL } else { MUTED });
    text(
        label,
        r.x + 8.,
        r.y + 18.,
        15.,
        if hover { LIGHT } else { TEAL },
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
/// Questions worth asking ECHO right now, most pressing first.
fn suggestions(g: &Game) -> Vec<&'static str> {
    let mut out = vec![];
    if g.records_found.iter().any(|id| !g.decoded.contains(id)) {
        out.push("What does the damaged record say?");
    }
    let terminal = g
        .terminal
        .as_ref()
        .is_some_and(|t| t.state == TerminalState::Locked && g.discovered(t.pos));
    if terminal && g.recovered() > 0 {
        out.push("Can we answer the terminal yet?");
    }
    if g.enemies.iter().any(|e| g.can_see(e.pos)) {
        out.push("What threat is that?");
    }
    out.push(if g.outcome != Outcome::Exploring {
        "What did all of this mean?"
    } else if g.restored {
        "Which way to the lift?"
    } else if g.recovered() == 3 {
        "Which way to the relay?"
    } else {
        "Where should I go next?"
    });
    if terminal {
        out.push("What is the terminal asking?");
    }
    out
}
/// Where a question about the way is pointing, if anywhere. The route itself is
/// always computed by the game from discovered tiles, never by the model.
fn route_target(g: &Game, question: &str) -> Option<Pos> {
    let q = question.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| q.contains(w));
    if !has(&[
        "way", "route", "where", "path", "get to", "lift", "relay", "terminal", "next", "cache",
    ]) {
        return None;
    }
    let target = if has(&["lift", "exit"]) {
        Some(g.lift)
    } else if has(&["relay"]) {
        Some(g.relay)
    } else if has(&["terminal"]) {
        g.terminal.as_ref().map(|t| t.pos)
    } else if has(&["cache"]) {
        g.caches.iter().find(|c| !c.taken).map(|c| c.pos)
    } else if g.restored {
        Some(g.lift)
    } else if g.recovered() == 3 {
        Some(g.relay)
    } else {
        g.archives
            .iter()
            .filter(|a| !a.recovered)
            .filter_map(|a| g.known_route(a.pos).map(|r| (r.len(), a.pos.x, a.pos.y)))
            .min()
            .map(|(_, x, y)| Pos { x, y })
    };
    target.filter(|t| g.known_route(*t).is_some())
}
/// The map tile under a point in design coordinates, if any.
fn map_tile(vp: Vec2) -> Option<Pos> {
    let (x, y) = (((vp.x - 28.) / 19.).floor(), ((vp.y - 109.) / 19.).floor());
    (x >= 0. && y >= 0. && x < WIDTH as f32 && y < HEIGHT as f32).then_some(Pos {
        x: x as i32,
        y: y as i32,
    })
}
fn draw_map(g: &Game, route: &[Pos]) {
    draw_rectangle(24., 105., 844., 540., PANEL);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let p = Pos { x, y };
            let i = Game::index(p).unwrap();
            if !g.seen[i] {
                continue;
            }
            let px = 28. + x as f32 * 19.;
            let py = 109. + y as f32 * 19.;
            let visible = g.visible[i];
            if g.tiles[i] == Tile::Wall {
                draw_rectangle(
                    px + 1.,
                    py + 1.,
                    17.,
                    17.,
                    if visible {
                        Color::new(0.20, 0.30, 0.32, 1.)
                    } else {
                        Color::new(0.10, 0.14, 0.16, 1.)
                    },
                );
                if visible {
                    draw_line(
                        px + 1.,
                        py + 1.,
                        px + 18.,
                        py + 1.,
                        1.,
                        Color::new(0.29, 0.41, 0.42, 1.),
                    );
                }
            } else {
                draw_rectangle(
                    px,
                    py,
                    19.,
                    19.,
                    if visible {
                        Color::new(0.08, 0.12, 0.14, 1.)
                    } else {
                        Color::new(0.045, 0.065, 0.08, 1.)
                    },
                );
                if visible {
                    draw_circle(px + 9., py + 9., 0.8, MUTED);
                }
            }
        }
    }
    for p in route.iter().skip(1) {
        draw_rectangle(
            28. + p.x as f32 * 19. + 7.,
            109. + p.y as f32 * 19. + 7.,
            5.,
            5.,
            Color::new(0.33, 0.86, 0.72, 0.75),
        );
    }
    if let Some(t) = g.terminal.as_ref().filter(|t| g.discovered(t.pos)) {
        text(
            "T",
            31. + t.pos.x as f32 * 19.,
            124. + t.pos.y as f32 * 19.,
            19.,
            if t.state == TerminalState::Locked {
                Color::new(0.85, 0.5, 0.95, 1.)
            } else {
                MUTED
            },
        );
    }
    for (pos, s, c) in [
        (g.lift, "L", TEAL),
        (g.relay, "R", if g.restored { TEAL } else { AMBER }),
    ] {
        if g.discovered(pos) {
            text(
                s,
                31. + pos.x as f32 * 19.,
                124. + pos.y as f32 * 19.,
                19.,
                c,
            );
        }
    }
    for a in &g.archives {
        if g.discovered(a.pos) && !a.recovered {
            text(
                "A",
                31. + a.pos.x as f32 * 19.,
                124. + a.pos.y as f32 * 19.,
                19.,
                AMBER,
            );
        }
    }
    for e in g.enemies.iter().filter(|e| g.can_see(e.pos)) {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let t = e.pos.offset(dx, dy);
            if g.floor(t) {
                draw_rectangle(
                    28. + t.x as f32 * 19.,
                    109. + t.y as f32 * 19.,
                    19.,
                    19.,
                    Color::new(0.99, 0.40, 0.34, 0.16),
                );
            }
        }
        draw_rectangle(
            30. + e.pos.x as f32 * 19.,
            110. + e.pos.y as f32 * 19.,
            15. * (e.hp.clamp(0, g.foe_hp(e.kind)) as f32 / g.foe_hp(e.kind) as f32),
            3.,
            CORAL,
        );
    }
    for c in g.caches.iter().filter(|c| !c.taken && g.discovered(c.pos)) {
        text(
            "C",
            31. + c.pos.x as f32 * 19.,
            124. + c.pos.y as f32 * 19.,
            19.,
            TEAL,
        );
    }
    for p in g.pickups.iter().filter(|p| !p.taken && g.discovered(p.pos)) {
        let (glyph, color) = match p.kind {
            PickupKind::PowerCell => ("*", AMBER),
            PickupKind::Medkit => ("+", Color::new(0.55, 0.9, 0.5, 1.)),
            PickupKind::Fragment => ("f", LIGHT),
            PickupKind::Decoy => ("b", TEAL),
            PickupKind::Overcharge => ("%", CORAL),
        };
        text(
            glyph,
            31. + p.pos.x as f32 * 19.,
            124. + p.pos.y as f32 * 19.,
            19.,
            color,
        );
    }
    for e in g.enemies.iter().filter(|e| g.can_see(e.pos)) {
        text(
            e.kind.glyph(),
            31. + e.pos.x as f32 * 19.,
            124. + e.pos.y as f32 * 19.,
            19.,
            match e.kind {
                EnemyKind::Sentinel => CORAL,
                EnemyKind::Hunter => AMBER,
                EnemyKind::Overseer => Color::new(0.85, 0.5, 0.95, 1.),
                EnemyKind::Drone => Color::new(0.55, 0.75, 0.98, 1.),
                EnemyKind::Turret => Color::new(0.95, 0.55, 0.55, 1.),
            },
        );
    }
    for e in g.enemies.iter().filter(|e| g.can_see(e.pos)) {
        let (mark, color) = if e.stun > 0 {
            ("z", TEAL)
        } else if e.passive {
            ("-", MUTED)
        } else {
            match e.awareness {
                Awareness::Alert => ("!", CORAL),
                Awareness::Searching => ("?", AMBER),
                Awareness::Idle => ("", MUTED),
            }
        };
        text(
            mark,
            41. + e.pos.x as f32 * 19.,
            118. + e.pos.y as f32 * 19.,
            12.,
            color,
        );
    }
    let px = 37. + g.player.x as f32 * 19.;
    let py = 118. + g.player.y as f32 * 19.;
    draw_circle(px, py, 7., TEAL);
    draw_circle(px, py, 3., INK);
}
enum Screen {
    Title,
    Game,
    Pause,
    Journal,
    Loadout,
    Equip,
    Console,
    NewConfirm,
    Terminal,
    Transmit,
    Records,
    Replay,
}
/// A finished run being played back from its action log on a fresh game.
struct Replay {
    game: Game,
    next: usize,
    paused: bool,
    /// Actions per second.
    speed: f64,
    last: f64,
}
struct Pending {
    rx: Receiver<Result<String, String>>,
    question: String,
    turn: u32,
    started: f64,
}

/// Finder starts a bundled app in "/", where config.json can be neither found
/// nor saved. Work from the save folder instead.
#[cfg(not(target_arch = "wasm32"))]
fn settle_working_dir() {
    let bundled =
        std::env::current_exe().is_ok_and(|p| p.to_string_lossy().contains(".app/Contents/MacOS"));
    if let Some(dir) = save::default_path().parent().filter(|_| bundled) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::env::set_current_dir(dir);
    }
}
#[macroquad::main(config)]
async fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    settle_working_dir();
    let args: Vec<String> = std::env::args().collect();
    let initial_seed = args
        .windows(2)
        .find(|w| w[0] == "--seed")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or_else(seed);
    let mut difficulty = args
        .windows(2)
        .find(|w| w[0] == "--difficulty")
        .and_then(|w| Difficulty::parse(&w[1]))
        .unwrap_or_default();
    init_font();
    let mut loadout = the_last_signal::core::default_loadout();
    let mut g = Game::new_with_difficulty(initial_seed, &loadout, difficulty);
    // Digits typed into the seed field on the new-expedition screen.
    let mut seed_text = String::new();
    let mut screen = Screen::Title;
    // Developer aid: `--preview N` opens screen N on a revealed map with one record
    // recovered, so layouts can be checked without playing to them.
    if let Some(n) = args
        .windows(2)
        .find(|w| w[0] == "--preview")
        .and_then(|w| w[1].parse::<u32>().ok())
    {
        g.seen.fill(true);
        g.player = g.archives[0].pos.offset(-1, 0);
        g.interact();
        if let Some(t) = &g.terminal {
            g.player = t.pos.offset(1, 0);
        }
        g.update_visibility(7);
        if n == 7 {
            // A short lost run, for the end screen and its replay.
            for _ in 0..6 {
                g.step(-1, 0);
                g.step(0, 1);
            }
            g.hp = 0;
            g.outcome = Outcome::Dead;
        }
        screen = match n {
            2 => Screen::Journal,
            3 => Screen::Terminal,
            4 => Screen::Transmit,
            5 => Screen::Equip,
            6 => Screen::NewConfirm,
            _ => Screen::Game,
        };
    }
    let profile = std::env::var_os("LAST_SIGNAL_PROFILE").is_some();
    let (mut frames, mut frame_sum, mut frame_max, mut map_sum) = (0u32, 0., 0f32, 0.);
    let mut console_models: Vec<ai::console::ModelInfo> = vec![];
    let mut console_sel = 0usize;
    let mut console_top = 0usize;
    let mut journal_floor = 0usize;
    let mut console_results: Vec<ai::console::Bench> = vec![];
    let mut console_rx: Option<Receiver<ai::console::Msg>> = None;
    let mut console_note = String::new();
    let mut transcript_key = (usize::MAX, usize::MAX);
    let mut transcript: Vec<(String, Color)> = vec![];
    let path = save::default_path();
    let (mut ai_config, config_error) = match ai::Config::load() {
        Ok(c) => (c, None),
        Err(e) => (ai::Config::default(), Some(e)),
    };
    let mut status = config_error.unwrap_or_else(|| {
        if DEMO {
            "Browser demo: ECHO is a built-in script and saving is off. Get the desktop build for a real local AI.".into()
        } else {
            "Local companion ready to connect when you ask.".into()
        }
    });
    let mut prompt = String::new();
    let mut focused = false;
    let mut pending: Option<Pending> = None;
    let mut scroll = 0usize;
    // A route ECHO was asked for: (target, floor), redrawn from known tiles each turn.
    let mut overlay: Option<(Pos, usize)> = None;
    let mut overlay_path: Vec<Pos> = vec![];
    let mut overlay_turn = u32::MAX;
    // Short-lived feedback over the player: (label, colour, time born).
    let mut floats: Vec<(String, Color, f64)> = vec![];
    let mut events_seen = g.events.len();
    let mut hurt_at = -1.0f64;
    // Set once the current run's end has been dealt with (save retired, run recorded).
    let mut run_recorded = false;
    let mut replay: Option<Replay> = None;
    // Tiles still to walk after a map click or O (explore), one per tick.
    let mut travel: Vec<Pos> = vec![];
    let mut travel_tick = 0.0f64;
    let mut exploring = false;
    let mut history = profile::load();
    // What the end screen says about this run against the history.
    let mut end_note = String::new();
    loop {
        // from_display_rect is y-up in macroquad 0.4; the UI is laid out y-down.
        let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 1280., 800.));
        camera.zoom.y = -camera.zoom.y;
        set_camera(&camera);
        clear_background(INK);
        if let Some(p) = &pending {
            let result = match p.rx.try_recv() {
                Ok(r) => Some(r),
                Err(TryRecvError::Disconnected) => {
                    Some(Err("AI worker stopped unexpectedly. Try again.".into()))
                }
                Err(TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let turn = p.turn;
                match result {
                    Ok(reply) => {
                        g.add_chat("assistant", &reply);
                        status = format!(
                            "ECHO replied using turn {turn}. Advice does not change game rules."
                        );
                    }
                    // The model is optional: ECHO's built-in script answers from the
                    // same discovered-state snapshot when Ollama cannot.
                    Err(e) => {
                        let reply = ai::demo_reply(&g, &p.question);
                        g.add_chat("assistant", &reply);
                        status = format!("ECHO answered from its offline script. {e}");
                    }
                }
                if g.decode_all() > 0 {
                    status = "ECHO reconstructed your damaged records: read them whole in the journal (J).".into();
                }
                scroll = 0;
                pending = None;
            }
        }
        if let Some(rx) = &console_rx {
            use ai::console::Msg;
            loop {
                match rx.try_recv() {
                    Ok(Msg::Models(Ok(m))) => {
                        console_note = if m.is_empty() {
                            "No models installed. In a terminal run: ollama pull <model>".into()
                        } else {
                            String::new()
                        };
                        console_sel = console_sel.min(m.len().saturating_sub(1));
                        console_models = m;
                    }
                    Ok(Msg::Models(Err(e))) => {
                        console_models.clear();
                        console_note = e;
                    }
                    Ok(Msg::Bench(b)) => {
                        console_results.retain(|r| r.model != b.model);
                        console_results.push(b);
                    }
                    Ok(Msg::Done) | Err(TryRecvError::Disconnected) => {
                        console_rx = None;
                        break;
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
        }
        text("THE LAST SIGNAL", 28., 42., 32., LIGHT);
        text(
            &format!(
                "FLOOR {}/{}  {}",
                g.floor + 1,
                FLOORS,
                FLOOR_NAMES[g.floor].to_uppercase()
            ),
            29.,
            65.,
            15.,
            MUTED,
        );
        text(
            &format!(
                "SEED {}   /   TURN {:03}   /   {}",
                g.seed,
                g.turn,
                g.difficulty.name().to_uppercase()
            ),
            29.,
            90.,
            15.,
            MUTED,
        );
        text(
            &format!(
                "HP {:02}/24  MEDKITS {}  POWER {}  KEYS {}/3{}",
                g.hp,
                g.medkits,
                g.energy,
                g.recovered(),
                if g.decoys > 0 {
                    format!("  DECOYS {}", g.decoys)
                } else {
                    String::new()
                }
            ),
            465.,
            48.,
            17.,
            if g.hp < 8 { CORAL } else { TEAL },
        );
        text(
            if g.restored {
                "RELAY ONLINE  /  RETURN TO LIFT"
            } else {
                "RECOVER ARCHIVES  /  RESTORE RELAY"
            },
            465.,
            76.,
            17.,
            AMBER,
        );
        {
            let mut x = 465.;
            let mut chip = |label: String, ok: bool| {
                text(&label, x, 99., 15., if ok { MUTED } else { CORAL });
                x += (label.chars().count() + 3) as f32 * 9.;
            };
            if g.loadout.is_empty() {
                chip("NO MODULES FITTED".into(), true);
            }
            for m in &g.loadout {
                match m {
                    Module::Scanner => {
                        chip(format!("SCAN [F] -{SCAN_COST}"), g.energy >= SCAN_COST)
                    }
                    Module::Shield => chip("SHIELD -1/hit".into(), g.energy >= 1),
                    Module::Analyzer => chip(
                        format!("ANALYZE [G] -{ANALYZE_COST}"),
                        g.energy >= ANALYZE_COST,
                    ),
                    Module::Pulse => {
                        chip(format!("PULSE [Q] -{PULSE_COST}"), g.energy >= PULSE_COST)
                    }
                }
            }
        }
        if overlay.is_some_and(|(t, f)| f != g.floor || g.player.distance(t) <= 1) {
            overlay = None;
        }
        if overlay_turn != g.turn {
            overlay_turn = g.turn;
            overlay_path = overlay
                .and_then(|(t, _)| g.known_route(t))
                .unwrap_or_default();
        }
        let now = get_time();
        // A loaded or restarted game changes the log wholesale: nothing to animate.
        if g.events.len() < events_seen || g.events.len() > events_seen + 8 {
            events_seen = g.events.len();
        }
        for e in &g.events[events_seen..] {
            let float = match e.kind.as_str() {
                "damage" => {
                    hurt_at = now;
                    Some(("HIT", CORAL))
                }
                "combat" if e.text.starts_with("Ambush") => Some(("AMBUSH 6", AMBER)),
                "combat" if e.text.ends_with("disabled.") => Some(("DISABLED", LIGHT)),
                "shield" => Some(("ABSORBED", TEAL)),
                "pickup" => Some(("SUPPLIES", TEAL)),
                "pulse" => Some(("PULSE", TEAL)),
                "decoy" => Some(("DECOY", TEAL)),
                "surge" => Some(("SURGE", CORAL)),
                "alert" => Some(("SPOTTED", CORAL)),
                "retreat" => Some(("FALLING BACK", AMBER)),
                "reinforce" => Some(("REINFORCED", CORAL)),
                "lockdown" => Some(("LOCKDOWN", CORAL)),
                "standdown" => Some(("TRUCE", TEAL)),
                _ => None,
            };
            if let Some((label, color)) = float {
                floats.push((label.into(), color, now));
            }
        }
        events_seen = g.events.len();
        floats.retain(|f| now - f.2 < 1.1);
        if g.outcome != Outcome::Exploring && !run_recorded {
            run_recorded = true;
            // One life: an earlier save of this run cannot bring it back.
            if save::retire_if_same_run(&path, g.seed) {
                status = "Expedition over. Its save file has been retired.".into();
            }
            let previous_best = history.best().map_or(0, |r| r.score);
            if let Some(run) =
                history.record(&g, &profile::date_of(macroquad::miniquad::date::now()))
            {
                end_note = if run.score > previous_best && history.runs.len() > 1 {
                    format!("New best score (previous {previous_best}).")
                } else if history.runs.len() == 1 {
                    "First expedition recorded.".into()
                } else {
                    format!(
                        "Best so far {}. Run {} recorded.",
                        previous_best.max(run.score),
                        history.runs.len()
                    )
                };
                if let Err(e) = profile::store(&history) {
                    end_note = format!("Run not recorded: {e}");
                }
            }
        }
        let map_t = get_time();
        draw_map(&g, &overlay_path);
        let (mx, my) = mouse_position();
        let vp = vec2(mx * 1280. / screen_width(), my * 800. / screen_height());
        let looked = matches!(screen, Screen::Game)
            .then(|| map_tile(vp))
            .flatten()
            .and_then(|p| g.describe(p).map(|d| (p, d)));
        if let Some((p, _)) = &looked {
            draw_rectangle_lines(
                28. + p.x as f32 * 19.,
                109. + p.y as f32 * 19.,
                19.,
                19.,
                1.5,
                LIGHT,
            );
        }
        for (i, (label, color, born)) in floats.iter().rev().take(3).enumerate() {
            let age = (now - born) as f32;
            let x = (37. + g.player.x as f32 * 19. - label.len() as f32 * 4.).clamp(30., 780.);
            let y = 100. + g.player.y as f32 * 19. - age * 26. - i as f32 * 15.;
            text(
                label,
                x,
                y.max(118.),
                14.,
                Color::new(color.r, color.g, color.b, (1.1 - age).clamp(0., 1.)),
            );
        }
        if now - hurt_at < 0.25 {
            draw_rectangle_lines(
                24.,
                105.,
                844.,
                540.,
                6.,
                Color::new(0.99, 0.40, 0.34, 1. - ((now - hurt_at) / 0.25) as f32),
            );
        }
        map_sum += get_time() - map_t;
        match &looked {
            Some((_, d)) => text(d, 28., 672., 15., LIGHT),
            None => text(
                "ARCHIVE A  RELAY R  LIFT L  CACHE C  TERMINAL T  FRAGMENT f  POWER * %  MEDKIT +  DECOY b  FOE S H D T O  /  hover: look",
                28.,
                672.,
                13.,
                MUTED,
            ),
        }
        for (i, e) in g.events.iter().rev().take(4).enumerate() {
            let msg = format!("{:03}  {}", e.turn, e.text);
            text(
                &msg.chars().take(86).collect::<String>(),
                28.,
                700. + i as f32 * 20.,
                16.,
                if i == 0 { LIGHT } else { MUTED },
            );
        }
        match g.hint() {
            Some(h) => text(&format!("> {h}"), 28., 788., 16., AMBER),
            None => text(
                "WASD move  E interact  H heal  F scan  G analyze  Q pulse  B decoy  I equip  SPACE wait  O explore  J journal  click: walk",
                28.,
                788.,
                14.,
                MUTED,
            ),
        }
        draw_rectangle(890., 24., 366., 738., PANEL);
        text("ECHO", 909., 55., 27., TEAL);
        text("LOCAL EXPEDITION COMPANION", 909., 78., 14., MUTED);
        text(
            if DEMO {
                "demo script"
            } else {
                &ai_config.model
            },
            909.,
            101.,
            15.,
            AMBER,
        );
        // Re-wrapping the whole conversation every frame is wasted work: rebuild
        // only when the history actually changes.
        let key = (
            g.chat.len(),
            g.chat.iter().map(|c| c.content.len()).sum::<usize>(),
        );
        if key != transcript_key {
            transcript_key = key;
            transcript.clear();
            if g.chat.is_empty() {
                transcript.extend(lines("Recover evidence. Ask about what you have found. ECHO only receives discovered game facts, but its advice can still be mistaken.",CHAT_COLS).into_iter().map(|l|(l,MUTED)));
            }
            for c in &g.chat {
                transcript.push((
                    if c.role == "user" { "YOU" } else { "ECHO" }.into(),
                    if c.role == "user" { AMBER } else { TEAL },
                ));
                transcript.extend(lines(&c.content, CHAT_COLS).into_iter().map(|l| (l, LIGHT)));
                transcript.push((String::new(), MUTED));
            }
        }
        let active = matches!(screen, Screen::Game);
        if active && Rect::new(890., 112., 366., 423.).contains(vp) {
            let (_, dy) = mouse_wheel();
            if dy > 0. {
                scroll = scroll.saturating_add(3);
            } else if dy < 0. {
                scroll = scroll.saturating_sub(3);
            }
        }
        scroll = scroll.min(transcript.len().saturating_sub(CHAT_ROWS));
        let end = transcript.len().saturating_sub(scroll);
        let start = end.saturating_sub(CHAT_ROWS);
        for (i, (l, c)) in transcript[start..end].iter().enumerate() {
            text(l, 909., 135. + i as f32 * 19., 17., *c);
        }
        if scroll > 0 {
            text("Scroll for latest", 1100., 101., 14., AMBER);
        }
        // One-click questions drawn from the current situation.
        let mut submit = false;
        if pending.is_none() {
            for (i, q) in suggestions(&g).into_iter().take(2).enumerate() {
                if chip(
                    &format!("> {q}"),
                    Rect::new(907., 466. + i as f32 * 32., 332., 27.),
                    active,
                ) {
                    prompt = q.to_string();
                    submit = true;
                }
            }
        }
        if let Some(p) = &pending {
            wrapped(
                &format!(
                    "Thinking locally... {:.0}s / snapshot turn {}",
                    get_time() - p.started,
                    p.turn
                ),
                909.,
                552.,
                37,
                15.,
                AMBER,
                2,
            );
        } else {
            wrapped(&status, 909., 552., 39, 15., MUTED, 3);
        }
        let input_rect = Rect::new(907., 612., 332., 58.);
        draw_rectangle(input_rect.x, input_rect.y, input_rect.w, input_rect.h, INK);
        draw_rectangle_lines(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            1.,
            if focused { TEAL } else { MUTED },
        );
        if prompt.is_empty() {
            text("Click here to ask ECHO...", 918., 637., 17., MUTED);
        } else {
            let shown = lines(&prompt, 35);
            for (i, l) in shown.iter().skip(shown.len().saturating_sub(3)).enumerate() {
                text(l, 916., 630. + i as f32 * 17., 15., LIGHT);
            }
        }
        if active && is_mouse_button_pressed(MouseButton::Left) {
            focused = input_rect.contains(vp);
        }
        let ask = button(
            if pending.is_some() {
                "THINKING..."
            } else {
                "SEND / ENTER"
            },
            Rect::new(907., 681., 158., 35.),
            active && pending.is_none() && !prompt.trim().is_empty(),
        );
        if button("JOURNAL / J", Rect::new(1078., 681., 161., 35.), active) {
            screen = Screen::Journal;
            journal_floor = g.floor;
            focused = false;
        }
        text("F5 save  /  F9 load  /  ESC pause", 907., 744., 16., MUTED);
        submit |= ask;
        if active {
            if focused {
                while let Some(c) = get_char_pressed() {
                    if !c.is_control() && prompt.chars().count() < 140 {
                        prompt.push(c);
                    }
                }
                if is_key_pressed(KeyCode::Backspace) {
                    prompt.pop();
                }
                if is_key_pressed(KeyCode::Enter) {
                    submit = true;
                }
                if is_key_pressed(KeyCode::Escape) {
                    focused = false;
                }
            } else {
                while get_char_pressed().is_some() {}
                // Any key interrupts walking; a click on a known tile starts it.
                if get_last_key_pressed().is_some() && !travel.is_empty() {
                    travel.clear();
                    exploring = false;
                    status = "Walk interrupted.".into();
                }
                if is_key_pressed(KeyCode::O) && g.outcome == Outcome::Exploring {
                    match g.frontier().and_then(|f| g.known_route(f)) {
                        Some(route) => {
                            travel = route[1..].to_vec();
                            exploring = true;
                            status =
                                "Exploring. Any key stops; a foe in sight stops it too.".into();
                        }
                        None => {
                            status = if g.enemies.iter().any(|e| g.can_see(e.pos) && !e.passive) {
                                "Not with a foe in sight.".into()
                            } else {
                                "Nothing left to explore on the known map.".into()
                            }
                        }
                    }
                }
                if is_mouse_button_pressed(MouseButton::Left) && g.outcome == Outcome::Exploring {
                    if let Some(t) = map_tile(vp).filter(|t| *t != g.player) {
                        match g.known_route(t) {
                            Some(route)
                                if !g.enemies.iter().any(|e| g.can_see(e.pos) && !e.passive) =>
                            {
                                travel = route[1..].to_vec();
                                exploring = false;
                            }
                            Some(_) => status = "Not with a foe in sight.".into(),
                            None if g.discovered(t) && g.floor(t) => {
                                status = "No known way there yet.".into()
                            }
                            None => {}
                        }
                    }
                }
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    g.step(0, -1);
                } else if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    g.step(0, 1);
                } else if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                    g.step(-1, 0);
                } else if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                    g.step(1, 0);
                } else if is_key_pressed(KeyCode::E) {
                    if g.terminal_in_reach() {
                        screen = Screen::Terminal;
                    }
                    if g.ready_to_transmit() {
                        screen = Screen::Transmit;
                    } else {
                        g.interact();
                    }
                } else if is_key_pressed(KeyCode::H) {
                    g.heal();
                } else if is_key_pressed(KeyCode::F) {
                    g.scan();
                } else if is_key_pressed(KeyCode::G) {
                    g.analyze();
                } else if is_key_pressed(KeyCode::Q) {
                    g.pulse();
                } else if is_key_pressed(KeyCode::Space) {
                    g.wait();
                } else if is_key_pressed(KeyCode::B) {
                    g.decoy();
                }
                if is_key_pressed(KeyCode::I) {
                    screen = Screen::Equip;
                }
                if is_key_pressed(KeyCode::J) {
                    screen = Screen::Journal;
                    journal_floor = g.floor;
                }
                if is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Pause;
                }
            }
            if is_key_pressed(KeyCode::F5) {
                status = match save::write(&g, &path) {
                    Ok(()) => format!("Saved turn {} and conversation history.", g.turn),
                    Err(e) => format!("Save failed: {e}"),
                };
            }
            if is_key_pressed(KeyCode::F9) {
                if pending.is_some() {
                    status = "Wait for ECHO before loading a different snapshot.".into();
                } else {
                    match save::read(&path) {
                        Ok(loaded) => {
                            g = loaded;
                            let _ = save::retire(&path);
                            run_recorded = false;
                            status = "Save loaded, including companion memory. One life: the save file is used up until you press F5 again.".into();
                            scroll = 0;
                            prompt.clear();
                        }
                        Err(e) => status = e,
                    }
                }
            }
        }
        // Walk one tile per tick along the chosen route, through the ordinary
        // move rules, so the action log and replay stay exact.
        if active && !travel.is_empty() && now - travel_tick > 0.07 {
            travel_tick = now;
            let hp_before = g.hp;
            let next = travel[0];
            let blocked = !g.floor(next)
                || g.enemies.iter().any(|e| e.pos == next)
                || next.distance(g.player) != 1;
            if blocked || g.outcome != Outcome::Exploring {
                travel.clear();
                exploring = false;
            } else {
                g.step(next.x - g.player.x, next.y - g.player.y);
                travel.remove(0);
                if g.enemies.iter().any(|e| g.can_see(e.pos) && !e.passive) {
                    travel.clear();
                    exploring = false;
                    status = "Halted: a foe is in sight.".into();
                } else if g.hp < hp_before {
                    travel.clear();
                    exploring = false;
                }
            }
            if travel.is_empty() && exploring && g.outcome == Outcome::Exploring {
                match g.frontier().and_then(|f| g.known_route(f)) {
                    Some(route) => travel = route[1..].to_vec(),
                    None => {
                        exploring = false;
                        status = "Explored everything reachable on this floor.".into();
                    }
                }
            }
        }
        if submit && active && pending.is_none() && !prompt.trim().is_empty() {
            match ai::Config::load() {
                Ok(c) => {
                    ai_config = c;
                    let body = ai::payload(&g, prompt.trim(), &ai_config);
                    g.add_chat("user", prompt.trim());
                    overlay = route_target(&g, prompt.trim()).map(|t| (t, g.floor));
                    overlay_turn = u32::MAX;
                    #[cfg(not(target_arch = "wasm32"))]
                    let rx = ai::start(ai_config.clone(), body);
                    #[cfg(target_arch = "wasm32")]
                    let rx = {
                        let _ = body;
                        ai::start_demo(&g, prompt.trim())
                    };
                    pending = Some(Pending {
                        rx,
                        question: prompt.trim().to_string(),
                        turn: g.turn,
                        started: get_time(),
                    });
                    prompt.clear();
                    focused = false;
                    scroll = 0;
                }
                Err(e) => status = e,
            }
        }
        if !matches!(screen, Screen::Game) {
            draw_rectangle(0., 0., 1280., 800., Color::new(0.015, 0.025, 0.04, 0.94));
            draw_rectangle(170., 110., 940., 580., PANEL);
            draw_rectangle_lines(170., 110., 940., 580., 1., MUTED);
            match screen {
                Screen::Title | Screen::Pause => {
                    text("THE LAST SIGNAL", 215., 180., 44., LIGHT);
                    text(
                        "A LOCAL AI EXPEDITION / PROTOTYPE 0.1",
                        217.,
                        214.,
                        18.,
                        TEAL,
                    );
                    wrapped("The complex is silent. On each of five floors, recover three archive keys, restore the relay, and take the lift down. Transmit the last signal from the vault.",217.,270.,76,22.,LIGHT,4);
                    wrapped("The archives are failing: records come out damaged, and only ECHO, your companion, can read them back. Ask it what they say, how to answer the Custodian terminals, and which way to go. With Ollama it is a real local model; without it, a built-in script answers.",217.,377.,88,18.,MUTED,4);
                    text(
                        "E interact   H medkit   F scan  G analyze  Q pulse   I equip   J evidence",
                        217.,
                        471.,
                        18.,
                        AMBER,
                    );
                    text(
                        if DEMO {
                            "Browser demo: saving is off, so reloading the page starts over."
                        } else {
                            "F5 saves. One life: a loaded save is used up, and death retires it."
                        },
                        217.,
                        501.,
                        18.,
                        MUTED,
                    );
                    if button("EXPLORE / ENTER", Rect::new(217., 541., 225., 42.), true)
                        || is_key_pressed(KeyCode::Enter)
                    {
                        screen = Screen::Game;
                    }
                    if !DEMO
                        && button(
                            "LOAD SAVE",
                            Rect::new(460., 541., 180., 42.),
                            pending.is_none(),
                        )
                    {
                        match save::read(&path) {
                            Ok(loaded) => {
                                loadout = loaded.loadout.clone();
                                g = loaded;
                                let _ = save::retire(&path);
                                run_recorded = false;
                                status = "Save loaded. One life: the save file is used up until you press F5 again.".into();
                                screen = Screen::Game;
                                scroll = 0;
                            }
                            Err(e) => status = e,
                        }
                    }
                    if button(
                        "NEW EXPEDITION",
                        Rect::new(659., 541., 232., 42.),
                        pending.is_none(),
                    ) {
                        screen = Screen::NewConfirm;
                    }
                    if !DEMO
                        && (button(
                            "AI CONSOLE / C",
                            Rect::new(217., 591., 225., 36.),
                            pending.is_none(),
                        ) || is_key_pressed(KeyCode::C))
                    {
                        screen = Screen::Console;
                        if console_models.is_empty() && console_rx.is_none() {
                            console_rx = Some(ai::console::spawn_list(ai_config.port));
                        }
                    }
                    if button("STARTER / M", Rect::new(909., 541., 170., 42.), g.turn == 0)
                        || (g.turn == 0 && is_key_pressed(KeyCode::M))
                    {
                        screen = Screen::Loadout;
                    }
                    if button("RECORDS / R", Rect::new(460., 591., 180., 36.), true)
                        || is_key_pressed(KeyCode::R)
                    {
                        screen = Screen::Records;
                    }
                    text(
                        &history.headline(profile::day_of(macroquad::miniquad::date::now())),
                        217.,
                        645.,
                        15.,
                        TEAL,
                    );
                    wrapped(&status, 217., 665., 92, 16., MUTED, 2);
                }
                Screen::Console => {
                    let busy = console_rx.is_some();
                    text("AI CONSOLE", 217., 185., 34., LIGHT);
                    text(
                        &format!(
                            "Active: {}   /   Ollama on 127.0.0.1:{}",
                            ai_config.model, ai_config.port
                        ),
                        217.,
                        216.,
                        17.,
                        TEAL,
                    );
                    wrapped(
                        "Tests use this game's rules: no leaking unrecovered records, use recovered ones, valid JSON, brevity. Score out of 100.",
                        217.,
                        242.,
                        84,
                        15.,
                        MUTED,
                        2,
                    );
                    if is_key_pressed(KeyCode::Down) {
                        console_sel = (console_sel + 1).min(console_models.len().saturating_sub(1));
                    }
                    if is_key_pressed(KeyCode::Up) {
                        console_sel = console_sel.saturating_sub(1);
                    }
                    if console_sel < console_top {
                        console_top = console_sel;
                    } else if console_sel >= console_top + CONSOLE_ROWS {
                        console_top = console_sel + 1 - CONSOLE_ROWS;
                    }
                    let (_, wheel) = mouse_wheel();
                    if wheel != 0. && console_models.len() > CONSOLE_ROWS {
                        console_top = if wheel > 0. {
                            console_top.saturating_sub(1)
                        } else {
                            (console_top + 1).min(console_models.len() - CONSOLE_ROWS)
                        };
                        console_sel =
                            console_sel.clamp(console_top, console_top + CONSOLE_ROWS - 1);
                    }
                    if console_models.len() > CONSOLE_ROWS {
                        text(
                            &format!(
                                "{} models: showing {}-{} (arrows or wheel to scroll)",
                                console_models.len(),
                                console_top + 1,
                                (console_top + CONSOLE_ROWS).min(console_models.len())
                            ),
                            640.,
                            548.,
                            15.,
                            MUTED,
                        );
                    }
                    for (n, m) in console_models
                        .iter()
                        .enumerate()
                        .skip(console_top)
                        .take(CONSOLE_ROWS)
                    {
                        let i = n;
                        let y = 300. + (n - console_top) as f32 * 38.;
                        let row = Rect::new(217., y - 22., 846., 32.);
                        if row.contains(vp) && is_mouse_button_pressed(MouseButton::Left) {
                            console_sel = i;
                        }
                        if i == console_sel {
                            draw_rectangle(
                                row.x,
                                row.y,
                                row.w,
                                row.h,
                                Color::new(0.12, 0.25, 0.25, 1.),
                            );
                            draw_rectangle_lines(row.x, row.y, row.w, row.h, 1., TEAL);
                        }
                        let name: String = m.name.chars().take(24).collect();
                        text(&format!("{name:<24}"), 230., y, 17., LIGHT);
                        text(&format!("{:>5.1} GB", m.size_gb), 500., y, 15., MUTED);
                        match console_results.iter().find(|r| r.model == m.name) {
                            Some(b) if b.error.is_some() => {
                                let e: String =
                                    b.error.as_deref().unwrap_or("").chars().take(36).collect();
                                text(&format!("FAILED: {e}"), 610., y, 15., CORAL);
                            }
                            Some(b) => text(
                                &format!(
                                    "SCORE {:>3}  {:>4.1}s  {}  {}",
                                    b.score(),
                                    b.avg_secs,
                                    if b.leaked { "LEAKS" } else { "no-leak" },
                                    if b.recalled { "recalls" } else { "no-recall" },
                                ),
                                610.,
                                y,
                                15.,
                                if b.score() >= 70 { TEAL } else { AMBER },
                            ),
                            None => text("not tested", 610., y, 15., MUTED),
                        }
                        if m.name == ai_config.model {
                            text("[ACTIVE]", 975., y, 15., AMBER);
                        }
                    }
                    if !console_note.is_empty() {
                        let (y, c) = if console_models.is_empty() {
                            (322., CORAL)
                        } else {
                            (
                                526.,
                                if console_note.starts_with("Saved") {
                                    TEAL
                                } else {
                                    CORAL
                                },
                            )
                        };
                        wrapped(&console_note, 217., y, 90, 16., c, 1);
                    }
                    if busy {
                        text(
                            &format!(
                                "Working... {} tested (local models can be slow)",
                                console_results.len()
                            ),
                            217.,
                            548.,
                            16.,
                            AMBER,
                        );
                    } else if let Some(b) = ai::console::best(&console_results) {
                        text(
                            &format!(
                                "BEST SO FAR: {} (score {}, {:.1}s)",
                                b.model,
                                b.score(),
                                b.avg_secs
                            ),
                            217.,
                            548.,
                            16.,
                            TEAL,
                        );
                    }
                    let have = !console_models.is_empty();
                    if button("REFRESH / R", Rect::new(217., 562., 150., 36.), !busy)
                        || (!busy && is_key_pressed(KeyCode::R))
                    {
                        console_rx = Some(ai::console::spawn_list(ai_config.port));
                    }
                    if button(
                        "TEST SELECTED / T",
                        Rect::new(380., 562., 215., 36.),
                        !busy && have,
                    ) || (!busy && have && is_key_pressed(KeyCode::T))
                    {
                        let name = console_models[console_sel].name.clone();
                        console_rx = Some(ai::console::spawn_bench(ai_config.clone(), vec![name]));
                    }
                    if button(
                        "TEST ALL / A",
                        Rect::new(608., 562., 160., 36.),
                        !busy && have,
                    ) || (!busy && have && is_key_pressed(KeyCode::A))
                    {
                        let all: Vec<String> = console_models
                            .iter()
                            .filter(|m| m.size_gb <= TEST_ALL_MAX_GB)
                            .map(|m| m.name.clone())
                            .collect();
                        let skipped = console_models.len() - all.len();
                        console_note = if skipped > 0 {
                            format!(
                                "Skipped {skipped} model(s) over {TEST_ALL_MAX_GB:.0} GB; test those with T."
                            )
                        } else {
                            String::new()
                        };
                        console_rx = Some(ai::console::spawn_bench(ai_config.clone(), all));
                    }
                    let mut chosen: Option<String> = None;
                    if button(
                        "USE SELECTED / ENTER",
                        Rect::new(781., 562., 260., 36.),
                        !busy && have,
                    ) || (!busy && have && is_key_pressed(KeyCode::Enter))
                    {
                        chosen = Some(console_models[console_sel].name.clone());
                    }
                    let best_model = ai::console::best(&console_results).map(|b| b.model.clone());
                    if button(
                        "USE BEST",
                        Rect::new(217., 612., 150., 36.),
                        !busy && best_model.is_some(),
                    ) {
                        chosen = best_model;
                    }
                    if let Some(name) = chosen {
                        let c = ai::Config {
                            model: name.clone(),
                            ..ai_config.clone()
                        };
                        match ai::console::save_config(&c) {
                            Ok(()) => {
                                ai_config = c;
                                status = format!("ECHO will now use {name}.");
                                console_note = format!("Saved {name} to config.json.");
                            }
                            Err(e) => console_note = e,
                        }
                    }
                    if button("BACK / ESC", Rect::new(380., 612., 200., 36.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Title;
                    }
                }
                Screen::Loadout => {
                    text("STARTING MODULE", 217., 185., 34., LIGHT);
                    wrapped(
                        &format!(
                            "Choose the one module you begin with. The others are found in caches (C) on the way down, and a slot opens on each new floor. All modules draw on one shared power pool of {}.",
                            the_last_signal::core::START_ENERGY
                        ),
                        217.,
                        220.,
                        88,
                        18.,
                        MUTED,
                        3,
                    );
                    for (i, m) in Module::ALL.iter().enumerate() {
                        let y = 290. + i as f32 * 74.;
                        let chosen = loadout.contains(m);
                        let hit = button(
                            if chosen { "CHOSEN" } else { "CHOOSE" },
                            Rect::new(217., y, 130., 42.),
                            true,
                        ) || is_key_pressed(
                            [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4][i],
                        );
                        text(
                            &format!("{}  [{}]  {}", i + 1, m.key_hint(), m.name()),
                            370.,
                            y + 18.,
                            20.,
                            if chosen { TEAL } else { LIGHT },
                        );
                        text(m.effect(), 370., y + 42., 16., MUTED);
                        if hit && !chosen {
                            loadout = vec![*m];
                            g = Game::new_with_difficulty(g.seed, &loadout, g.difficulty);
                        }
                    }
                    if button("DONE / ESC", Rect::new(217., 603., 200., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                        || is_key_pressed(KeyCode::Enter)
                    {
                        screen = Screen::Title;
                    }
                }
                Screen::Equip => {
                    text("EQUIPMENT", 217., 185., 34., LIGHT);
                    text(
                        &format!(
                            "{} of {} slots in use on this floor. Fitting or unfitting takes a turn.",
                            g.loadout.len(),
                            g.slots()
                        ),
                        217.,
                        220.,
                        17.,
                        TEAL,
                    );
                    for (i, m) in Module::ALL.iter().enumerate() {
                        let y = 262. + i as f32 * 68.;
                        let owned = g.owned.contains(m);
                        let fitted = g.has(*m);
                        if owned {
                            let hit = button(
                                if fitted { "UNFIT" } else { "FIT" },
                                Rect::new(217., y, 130., 42.),
                                g.outcome == Outcome::Exploring,
                            ) || is_key_pressed(
                                [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4][i],
                            );
                            if hit {
                                if fitted {
                                    g.unfit(*m);
                                } else {
                                    g.fit(*m);
                                }
                            }
                        } else {
                            text("NOT FOUND", 225., y + 26., 16., MUTED);
                        }
                        text(
                            &format!("{}  [{}]  {}", i + 1, m.key_hint(), m.name()),
                            370.,
                            y + 18.,
                            20.,
                            if fitted {
                                TEAL
                            } else if owned {
                                LIGHT
                            } else {
                                MUTED
                            },
                        );
                        text(m.effect(), 370., y + 42., 16., MUTED);
                    }
                    if let Some(e) = g.events.last() {
                        wrapped(&e.text, 217., 562., 96, 16., AMBER, 1);
                    }
                    if button("DONE / ESC", Rect::new(217., 603., 200., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::NewConfirm => {
                    text("START A NEW EXPEDITION?", 217., 190., 32., LIGHT);
                    wrapped("Unsaved progress in this window will be replaced. Your existing save file stays unchanged until you press F5.",217.,240.,77,19.,MUTED,2);
                    text("DIFFICULTY", 217., 312., 16., MUTED);
                    for (i, d) in Difficulty::ALL.iter().enumerate() {
                        let r = Rect::new(217. + i as f32 * 183., 322., 170., 34.);
                        if button(
                            &if *d == difficulty {
                                format!("* {}", d.name().to_uppercase())
                            } else {
                                d.name().to_uppercase()
                            },
                            r,
                            true,
                        ) {
                            difficulty = *d;
                        }
                    }
                    wrapped(difficulty.about(), 217., 380., 92, 16., AMBER, 2);
                    let mut start = |g: &mut Game, new_seed: u64, note: &str| {
                        *g = Game::new_with_difficulty(new_seed, &loadout, difficulty);
                        run_recorded = false;
                        screen = Screen::Game;
                        prompt.clear();
                        scroll = 0;
                        status = note.to_string();
                    };
                    if button("NEW SEED", Rect::new(217., 425., 230., 42.), true) {
                        start(
                            &mut g,
                            seed(),
                            "New expedition started. Your existing disk save is unchanged.",
                        );
                    }
                    if button("DAILY SIGNAL", Rect::new(217., 485., 230., 42.), true) {
                        // Same complex for everyone today: compare scores.
                        let day = (macroquad::miniquad::date::now() / 86_400.) as u64;
                        start(
                            &mut g,
                            20_000_000 + day,
                            "Daily signal: everyone gets this same complex today. Compare scores.",
                        );
                    }
                    wrapped("Daily signal: one shared seed per day, the same complex for everyone. Your score is shown when the run ends.", 469., 503., 70, 16., MUTED, 2);
                    // A typed seed: replay a run you liked, or one a friend named.
                    let field = Rect::new(217., 545., 230., 42.);
                    draw_rectangle(field.x, field.y, field.w, field.h, INK);
                    draw_rectangle_lines(field.x, field.y, field.w, field.h, 1., TEAL);
                    if seed_text.is_empty() {
                        text("Type a seed...", 229., 572., 17., MUTED);
                    } else {
                        text(&seed_text, 229., 572., 17., LIGHT);
                    }
                    while let Some(c) = get_char_pressed() {
                        if c.is_ascii_digit() && seed_text.len() < 18 {
                            seed_text.push(c);
                        }
                    }
                    if is_key_pressed(KeyCode::Backspace) {
                        seed_text.pop();
                    }
                    let typed = seed_text.parse::<u64>().ok();
                    if button(
                        "START WITH SEED",
                        Rect::new(469., 545., 230., 42.),
                        typed.is_some(),
                    ) || (typed.is_some() && is_key_pressed(KeyCode::Enter))
                    {
                        start(
                            &mut g,
                            typed.unwrap_or_default(),
                            "Expedition started from your seed. Same seed, same complex, every time.",
                        );
                    }
                    if button("CANCEL / ESC", Rect::new(469., 425., 230., 42.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::Replay => {
                    // The whole screen belongs to the replay: its own HUD, map and log.
                    draw_rectangle(0., 0., 1280., 800., INK);
                    if let Some(r) = replay.as_mut() {
                        if is_key_pressed(KeyCode::Space) {
                            r.paused = !r.paused;
                        }
                        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) {
                            r.speed = (r.speed * 2.).min(64.);
                        }
                        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) {
                            r.speed = (r.speed / 2.).max(1.);
                        }
                        let step_once =
                            is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Period);
                        let now = get_time();
                        let due = !r.paused && now - r.last >= 1. / r.speed;
                        if (due || step_once) && r.next < g.actions.len() {
                            r.game.apply(g.actions[r.next]);
                            r.next += 1;
                            r.last = now;
                        }
                        let done = r.next >= g.actions.len();
                        text("REPLAY", 28., 42., 32., AMBER);
                        text(
                            &format!(
                                "SEED {}  /  {}  /  FLOOR {}/{}  {}",
                                r.game.seed,
                                r.game.difficulty.name().to_uppercase(),
                                r.game.floor + 1,
                                FLOORS,
                                FLOOR_NAMES[r.game.floor].to_uppercase()
                            ),
                            29.,
                            65.,
                            15.,
                            MUTED,
                        );
                        text(
                            &format!(
                                "ACTION {}/{}   TURN {:03}   {}x{}",
                                r.next,
                                g.actions.len(),
                                r.game.turn,
                                r.speed,
                                if done {
                                    "   FINISHED"
                                } else if r.paused {
                                    "   PAUSED"
                                } else {
                                    ""
                                }
                            ),
                            29.,
                            90.,
                            15.,
                            MUTED,
                        );
                        text(
                            &format!(
                                "HP {:02}/24  MED {}  PWR {}  KEYS {}/3  KILLS {}",
                                r.game.hp,
                                r.game.medkits,
                                r.game.energy,
                                r.game.recovered(),
                                r.game.kills
                            ),
                            465.,
                            90.,
                            15.,
                            if r.game.hp < 8 { CORAL } else { TEAL },
                        );
                        draw_map(&r.game, &[]);
                        for (i, e) in r.game.events.iter().rev().take(4).enumerate() {
                            let msg = format!("{:03}  {}", e.turn, e.text);
                            text(
                                &msg.chars().take(86).collect::<String>(),
                                28.,
                                700. + i as f32 * 20.,
                                16.,
                                if i == 0 { LIGHT } else { MUTED },
                            );
                        }
                        text(
                            "SPACE pause   RIGHT step   + / - speed   ESC close",
                            28.,
                            788.,
                            15.,
                            AMBER,
                        );
                        draw_rectangle(890., 24., 366., 738., PANEL);
                        text("REPLAY", 909., 55., 27., TEAL);
                        wrapped(
                            "Every run replays exactly from its seed, starting module, difficulty and action log. What you see here is the same rules run again, not a recording.",
                            909.,
                            100.,
                            39,
                            15.,
                            MUTED,
                            6,
                        );
                        if button("CLOSE / ESC", Rect::new(907., 681., 158., 35.), true)
                            || is_key_pressed(KeyCode::Escape)
                        {
                            screen = Screen::Game;
                        }
                    } else {
                        screen = Screen::Game;
                    }
                    if !matches!(screen, Screen::Replay) {
                        replay = None;
                    }
                }
                Screen::Records => {
                    text("EXPEDITION RECORDS", 217., 185., 34., LIGHT);
                    text(
                        &history.headline(profile::day_of(macroquad::miniquad::date::now())),
                        217.,
                        218.,
                        16.,
                        TEAL,
                    );
                    if history.runs.is_empty() {
                        text(
                            "Finish an expedition, won or lost, and it is recorded here.",
                            217.,
                            262.,
                            17.,
                            MUTED,
                        );
                    }
                    for (i, r) in history.runs.iter().rev().take(16).enumerate() {
                        text(
                            &r.line(),
                            217.,
                            262. + i as f32 * 22.,
                            15.,
                            if r.won { LIGHT } else { MUTED },
                        );
                    }
                    text(
                        if DEMO {
                            "Kept in this browser's local storage."
                        } else {
                            "Kept in profile.json beside your save."
                        },
                        217.,
                        632.,
                        14.,
                        MUTED,
                    );
                    if button("BACK / ESC", Rect::new(217., 645., 200., 36.), true)
                        || is_key_pressed(KeyCode::Escape)
                        || is_key_pressed(KeyCode::Enter)
                    {
                        screen = if g.turn == 0 {
                            Screen::Title
                        } else {
                            Screen::Pause
                        };
                    }
                }
                Screen::Transmit => {
                    text("THE VAULT LIFT", 217., 185., 34., LIGHT);
                    text("CHOOSE THE LAST SIGNAL", 217., 216., 17., TEAL);
                    wrapped("The relay will carry one transmission. What you found and how you treated the Custodians decide what you can send. Ask ECHO if you are unsure: it knows these options.", 217., 250., 92, 17., MUTED, 3);
                    let mut pick = None;
                    for (i, sig) in Signal::ALL.iter().enumerate() {
                        let y = 330. + i as f32 * 88.;
                        let open = g.signal_open(*sig);
                        if button(
                            &format!("{}  {}", i + 1, sig.name()),
                            Rect::new(217., y, 330., 40.),
                            open.is_ok(),
                        ) || (open.is_ok()
                            && is_key_pressed([KeyCode::Key1, KeyCode::Key2, KeyCode::Key3][i]))
                        {
                            pick = Some(*sig);
                        }
                        wrapped(sig.about(), 565., y + 16., 60, 15., LIGHT, 2);
                        if let Err(why) = open {
                            wrapped(why, 565., y + 54., 60, 15., CORAL, 1);
                        }
                    }
                    if let Some(sig) = pick {
                        g.transmit(sig);
                        screen = Screen::Game;
                    }
                    if button("NOT YET / ESC", Rect::new(217., 610., 220., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::Terminal => {
                    let c = &CHALLENGES[g.floor];
                    text("CUSTODIAN TERMINAL", 217., 185., 34., LIGHT);
                    text(
                        "AUTHORISATION CHALLENGE  /  ONE ATTEMPT",
                        217.,
                        216.,
                        17.,
                        CORAL,
                    );
                    wrapped(c.question, 217., 270., 80, 24., AMBER, 2);
                    wrapped("Accepted: power recharged to full, Custodians +2. Rejected: it drains 2 power, locks and alerts the floor, Custodians -1. The answer is written in one of this floor's records. Yours are damaged: ECHO can read them back to you.", 217., 340., 92, 17., MUTED, 4);
                    let mut pick = None;
                    for (i, o) in c.options.iter().enumerate() {
                        if button(
                            &format!("{}  {}", i + 1, o),
                            Rect::new(217., 440. + i as f32 * 50., 600., 40.),
                            g.terminal_in_reach(),
                        ) || is_key_pressed([KeyCode::Key1, KeyCode::Key2, KeyCode::Key3][i])
                        {
                            pick = Some(i);
                        }
                    }
                    if let Some(i) = pick {
                        g.answer(i);
                        screen = Screen::Game;
                    }
                    if button("NOT YET / ESC", Rect::new(217., 610., 220., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::Journal => {
                    text("RECOVERED EVIDENCE", 217., 185., 34., LIGHT);
                    text(
                        "Verified game records / ECHO's conversation is advisory",
                        217.,
                        220.,
                        18.,
                        TEAL,
                    );
                    if is_key_pressed(KeyCode::Right) {
                        journal_floor = (journal_floor + 1).min(g.floor);
                    }
                    if is_key_pressed(KeyCode::Left) {
                        journal_floor = journal_floor.saturating_sub(1);
                    }
                    journal_floor = journal_floor.min(g.floor);
                    text(
                        &format!(
                            "FLOOR {}/{}  {}   (LEFT / RIGHT)",
                            journal_floor + 1,
                            FLOORS,
                            FLOOR_NAMES[journal_floor].to_uppercase()
                        ),
                        217.,
                        248.,
                        16.,
                        AMBER,
                    );
                    for n in 0..4 {
                        let id = if n < 3 {
                            journal_floor * 3 + n
                        } else {
                            FRAGMENT_BASE + journal_floor
                        };
                        let y = 280. + n as f32 * 60.;
                        text(
                            &if n < 3 {
                                format!("ARCHIVE {:02}  /  {}", id + 1, RECORD_AUTHORS[id].name())
                            } else {
                                format!(
                                    "DATA FRAGMENT (optional)  /  {}",
                                    RECORD_AUTHORS[id].name()
                                )
                            },
                            217.,
                            y,
                            16.,
                            AMBER,
                        );
                        let recovered = g.records_found.contains(&id);
                        if recovered && !g.decoded.contains(&id) {
                            text("DAMAGED: ASK ECHO WHAT IT SAYS", 640., y, 15., CORAL);
                        }
                        wrapped(
                            &if recovered {
                                g.record_text(id)
                            } else {
                                "Not recovered. Contents unknown.".to_string()
                            },
                            217.,
                            y + 20.,
                            100,
                            16.,
                            if recovered { LIGHT } else { MUTED },
                            2,
                        );
                    }
                    text("FACTIONS", 217., 530., 16., AMBER);
                    for (i, f) in Faction::ALL.iter().enumerate() {
                        let y = 553. + i as f32 * 38.;
                        let st = g.standing_of(*f);
                        text(
                            &format!("{}  {:+}", f.name().to_uppercase(), st),
                            217.,
                            y,
                            17.,
                            if st < 0 { CORAL } else { TEAL },
                        );
                        wrapped(f.about(), 217., y + 19., 96, 15., MUTED, 1);
                    }
                    if button("BACK / ESC", Rect::new(217., 640., 200., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::Game => {}
            }
        } else if g.outcome != Outcome::Exploring {
            draw_rectangle(185., 200., 540., 335., INK);
            draw_rectangle_lines(
                185.,
                200.,
                540.,
                335.,
                2.,
                if g.outcome == Outcome::Escaped {
                    TEAL
                } else {
                    CORAL
                },
            );
            text(
                if g.outcome == Outcome::Escaped {
                    "SIGNAL RECEIVED"
                } else {
                    "SIGNAL LOST"
                },
                212.,
                250.,
                34.,
                LIGHT,
            );
            let summary = g.summary();
            text(summary.rank, 212., 285., 22., AMBER);
            for (i, l) in summary.lines.iter().take(3).enumerate() {
                text(l, 212., 318. + i as f32 * 24., 17., LIGHT);
            }
            wrapped(&summary.lines[3], 212., 398., 55, 16., MUTED, 2);
            text(&end_note, 212., 448., 15., TEAL);
            wrapped(
                "You can still talk to ECHO. ESC opens the menu.",
                212.,
                472.,
                66,
                15.,
                MUTED,
                1,
            );
            let can_replay = active && g.replayable && !g.actions.is_empty();
            if button(
                "WATCH REPLAY / R",
                Rect::new(385., 490., 200., 34.),
                can_replay,
            ) || (can_replay && !focused && is_key_pressed(KeyCode::R))
            {
                replay = Some(Replay {
                    game: Game::new_with_difficulty(g.seed, &g.loadout, g.difficulty),
                    next: 0,
                    paused: false,
                    speed: 8.,
                    last: get_time(),
                });
                screen = Screen::Replay;
            }
            if button("COPY RECAP", Rect::new(212., 490., 160., 34.), active) {
                macroquad::miniquad::window::clipboard_set(&g.recap());
                #[cfg(target_arch = "wasm32")]
                {
                    status = "Recap copied to the clipboard: paste it anywhere.".into();
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let date = profile::date_of(macroquad::miniquad::date::now());
                    status = match save::write_morgue(&g, &path, &date) {
                        Ok(p) => format!("Recap copied. Morgue file written to {}", p.display()),
                        Err(e) => format!("Recap copied, but the morgue file failed: {e}"),
                    };
                }
            }
        }
        set_default_camera();
        if profile {
            let dt = get_frame_time();
            frames += 1;
            frame_sum += dt as f64;
            frame_max = frame_max.max(dt);
            if frames == 300 {
                eprintln!(
                    "profile: avg frame {:.2} ms, max {:.2} ms, draw_map cpu {:.3} ms/frame",
                    frame_sum / 300. * 1e3,
                    frame_max * 1e3,
                    map_sum / 300. * 1e3
                );
                (frames, frame_sum, frame_max, map_sum) = (0, 0., 0., 0.);
            }
        }
        next_frame().await;
    }
}
