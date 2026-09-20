use macroquad::prelude::*;
use std::sync::mpsc::{Receiver, TryRecvError};
use the_last_signal::{
    ai,
    core::{Game, Outcome, Pos, Tile, HEIGHT, RECORDS, WIDTH},
    save,
};

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
fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(s, x, y, size, color);
}
fn lines(s: &str, width: usize) -> Vec<String> {
    let mut out = vec![];
    for paragraph in s.lines() {
        let mut line = String::new();
        for c in paragraph.chars() {
            if line.chars().count() >= width {
                out.push(line);
                line = String::new();
            }
            line.push(c);
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
fn wrapped(s: &str, x: f32, y: f32, width: usize, size: f32, color: Color, max: usize) {
    for (i, l) in lines(s, width).iter().take(max).enumerate() {
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
fn draw_map(g: &Game) {
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
    for e in &g.enemies {
        if g.can_see(e.pos) {
            text(
                "S",
                31. + e.pos.x as f32 * 19.,
                124. + e.pos.y as f32 * 19.,
                19.,
                CORAL,
            );
        }
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
    NewConfirm,
}
struct Pending {
    rx: Receiver<Result<String, String>>,
    turn: u32,
    started: f64,
}

#[macroquad::main(config)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let initial_seed = args
        .windows(2)
        .find(|w| w[0] == "--seed")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or_else(seed);
    let mut g = Game::new(initial_seed);
    let mut screen = Screen::Title;
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
                        scroll = 0;
                    }
                    Err(e) => status = e,
                }
                pending = None;
            }
        }
        text("THE LAST SIGNAL", 28., 42., 32., LIGHT);
        text("EXPEDITION 01 / THE SILENT RELAY", 29., 65., 15., MUTED);
        text(
            &format!("SEED {}   /   TURN {:03}", g.seed, g.turn),
            29.,
            90.,
            15.,
            MUTED,
        );
        text(
            &format!(
                "HP {:02}/24     MEDKITS {}     SCANNER {}     KEYS {}/3",
                g.hp,
                g.medkits,
                g.energy,
                g.recovered()
            ),
            465.,
            48.,
            19.,
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
        draw_map(&g);
        text(
            "YOU  o     ARCHIVE  A     RELAY  R     LIFT  L     SENTINEL  S",
            28.,
            672.,
            17.,
            MUTED,
        );
        for (i, e) in g.events.iter().rev().take(4).enumerate() {
            let msg = format!("{:03}  {}", e.turn, e.text);
            text(
                &msg.chars().take(95).collect::<String>(),
                28.,
                700. + i as f32 * 20.,
                16.,
                if i == 0 { LIGHT } else { MUTED },
            );
        }
        text(
            "WASD / ARROWS move    E interact    H heal    F scan    SPACE wait    J journal",
            28.,
            788.,
            15.,
            MUTED,
        );
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
        let mut transcript: Vec<(String, Color)> = vec![];
        if g.chat.is_empty() {
            transcript.extend(lines("Recover evidence. Ask about what you have found. ECHO only receives discovered game facts, but its advice can still be mistaken.",36).into_iter().map(|l|(l,MUTED)));
        }
        for c in &g.chat {
            transcript.push((
                if c.role == "user" { "YOU" } else { "ECHO" }.into(),
                if c.role == "user" { AMBER } else { TEAL },
            ));
            transcript.extend(lines(&c.content, 36).into_iter().map(|l| (l, LIGHT)));
            transcript.push((String::new(), MUTED));
        }
        let (mx, my) = mouse_position();
        let vp = vec2(mx * 1280. / screen_width(), my * 800. / screen_height());
        let active = matches!(screen, Screen::Game);
        if active && Rect::new(890., 112., 366., 423.).contains(vp) {
            let (_, dy) = mouse_wheel();
            if dy > 0. {
                scroll = scroll.saturating_add(3);
            } else if dy < 0. {
                scroll = scroll.saturating_sub(3);
            }
        }
        scroll = scroll.min(transcript.len().saturating_sub(20));
        let end = transcript.len().saturating_sub(scroll);
        let start = end.saturating_sub(20);
        for (i, (l, c)) in transcript[start..end].iter().enumerate() {
            text(l, 909., 135. + i as f32 * 19., 17., *c);
        }
        if scroll > 0 {
            text("Scroll down for latest", 909., 529., 14., AMBER);
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
            wrapped(&prompt, 918., 635., 34, 17., LIGHT, 2);
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
            focused = false;
        }
        text("F5 save  /  F9 load  /  ESC pause", 907., 744., 16., MUTED);
        let mut submit = ask;
        if active {
            if focused {
                while let Some(c) = get_char_pressed() {
                    if !c.is_control() && prompt.chars().count() < 68 {
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
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                    g.step(0, -1);
                } else if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                    g.step(0, 1);
                } else if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                    g.step(-1, 0);
                } else if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                    g.step(1, 0);
                } else if is_key_pressed(KeyCode::E) {
                    g.interact();
                } else if is_key_pressed(KeyCode::H) {
                    g.heal();
                } else if is_key_pressed(KeyCode::F) {
                    g.scan();
                } else if is_key_pressed(KeyCode::Space) {
                    g.wait();
                }
                if is_key_pressed(KeyCode::J) {
                    screen = Screen::Journal;
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
                            status = "Save loaded, including companion memory.".into();
                            scroll = 0;
                            prompt.clear();
                        }
                        Err(e) => status = e,
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
                    #[cfg(not(target_arch = "wasm32"))]
                    let rx = ai::start(ai_config.clone(), body);
                    #[cfg(target_arch = "wasm32")]
                    let rx = {
                        let _ = body;
                        ai::start_demo(&g, prompt.trim())
                    };
                    pending = Some(Pending {
                        rx,
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
                    wrapped("The complex is silent. Three archive fragments hold the keys to its relay. Recover the evidence, transmit the last signal, and return to the surface lift.",217.,270.,76,22.,LIGHT,4);
                    wrapped("ECHO is your optional local companion. Ask about discoveries as you explore. Ollama runs separately on your computer; no model is bundled. The expedition remains playable without it.",217.,377.,88,18.,MUTED,4);
                    text(
                        "E interact beside A / R / L    H medkit    F scan    J evidence",
                        217.,
                        471.,
                        18.,
                        AMBER,
                    );
                    text(
                        if DEMO {
                            "Browser demo: saving is off, so reloading the page starts over."
                        } else {
                            "F5 saves. Closing the window does not auto-save."
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
                                g = loaded;
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
                    wrapped(&status, 217., 626., 92, 17., MUTED, 2);
                }
                Screen::NewConfirm => {
                    text("START A NEW EXPEDITION?", 217., 190., 32., LIGHT);
                    wrapped("Unsaved progress in this window will be replaced. Your existing save file stays unchanged until you press F5.",217.,260.,77,21.,MUTED,4);
                    if button("NEW SEED", Rect::new(217., 410., 230., 42.), true) {
                        g = Game::new(seed());
                        screen = Screen::Game;
                        prompt.clear();
                        scroll = 0;
                        status =
                            "New expedition started. Your existing disk save is unchanged.".into();
                    }
                    if button("CANCEL / ESC", Rect::new(469., 410., 230., 42.), true)
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
                    for (id, record) in RECORDS.iter().enumerate() {
                        let y = 280. + id as f32 * 95.;
                        text(&format!("ARCHIVE {:02}", id + 1), 217., y, 18., AMBER);
                        let recovered = g.archives.iter().any(|a| a.id == id && a.recovered);
                        wrapped(
                            if recovered {
                                record
                            } else {
                                "Not recovered. Contents unknown."
                            },
                            217.,
                            y + 28.,
                            84,
                            18.,
                            if recovered { LIGHT } else { MUTED },
                            3,
                        );
                    }
                    if button("BACK / ESC", Rect::new(217., 603., 200., 40.), true)
                        || is_key_pressed(KeyCode::Escape)
                    {
                        screen = Screen::Game;
                    }
                }
                Screen::Game => {}
            }
        } else if g.outcome != Outcome::Exploring {
            draw_rectangle(185., 263., 540., 164., INK);
            draw_rectangle_lines(
                185.,
                263.,
                540.,
                164.,
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
                308.,
                34.,
                LIGHT,
            );
            wrapped(
                "You can still talk to ECHO. ESC opens the menu; F9 loads your saved expedition.",
                212.,
                349.,
                51,
                18.,
                MUTED,
                3,
            );
        }
        set_default_camera();
        next_frame().await;
    }
}
