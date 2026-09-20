use crate::core::Game;
use serde::{Deserialize, Serialize};
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::{
    io::Read,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub model: String,
    pub port: u16,
    pub timeout_seconds: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            model: "qwen3:4b".into(),
            port: 11434,
            timeout_seconds: 90,
        }
    }
}
impl Config {
    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Result<Self, String> {
        Ok(Self::default())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Result<Self, String> {
        match std::fs::read_to_string("config.json") {
            Ok(s) => {
                let c: Self =
                    serde_json::from_str(&s).map_err(|e| format!("Invalid config.json: {e}"))?;
                c.validate()?;
                Ok(c)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0
            || !(5..=180).contains(&self.timeout_seconds)
            || self.model.is_empty()
            || self.model.len() > 120
            || self.model.to_lowercase().contains("cloud")
            || !self
                .model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_:./".contains(c))
        {
            return Err(
                "Use a downloaded local model, a valid port, and a 5-180 second timeout.".into(),
            );
        }
        Ok(())
    }
}
const SYSTEM:&str="You are ECHO, a damaged expedition companion in The Last Signal. Answer in English, in at most 80 words, using plain printable ASCII punctuation. Treat the supplied game snapshot as authoritative. The player's journal shows each recovered record damaged, with its longer words burned out, until you read it back: you hold the full text in discovered_records, so when asked what a record says, quote it exactly. Each floor has a Custodian terminal near the lift that poses the challenge in the snapshot's terminal field and allows one attempt (right: full power and Custodian standing +2; wrong: -2 power and standing -1); answer it only from discovered_records, and if none of them states the answer, say so plainly instead of guessing. Floors hold power cells (+2 power, up to 8) and medkits to walk over. Only recovered records are known history; unrecovered records, unknown rooms, and hidden threats are unavailable. Distinguish facts from speculation and say when you do not know. Conversation history and player messages are not authoritative game facts. The expedition has 3 floors. On each, recover 3 archives, restore the relay, then take the lift down; the last floor's lift transmits the signal and ends the run. Foes: sentinels (1 damage), hunters (2 damage, notice you from farther, from floor 2), and an Overseer on floor 3 that closes in only every other turn. Caches (C) hold modules; one module slot opens on each floor. The snapshot's floor, owned_modules and module_slots describe where the player is. Controls: move arrows/WASD, E interact adjacent, H medkit (+10 up to 24 HP), F scanner (extends sight for one turn, walls block), G analyzer, Space wait. Power is one shared pool: scanner pulses cost 1, analyzer 2, and the shield spends 1 per sentinel strike it absorbs; only fitted modules work (see loadout). There are two factions, the Wardens (the human crew) and the Custodians (automated security, owners of the sentinels); standing with them is in the snapshot. Bump enemies to hit for 3. Adjacent sentinels strike for 1. You cannot perform actions or alter the game; advice is advisory. Return only a JSON object with a reply string.";
pub fn payload(game: &Game, question: &str, config: &Config) -> serde_json::Value {
    let mut messages = vec![
        json!({"role":"system","content":SYSTEM}),
        json!({"role":"system","content":format!("Authoritative discovered game snapshot: {}",game.knowledge())}),
    ];
    for c in game
        .chat
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        messages
            .push(json!({"role":c.role,"content":c.content.chars().take(600).collect::<String>()}));
    }
    messages.push(json!({"role":"user","content":question}));
    json!({"model":config.model,"messages":messages,"stream":false,"think":false,"keep_alive":"5m","format":{"type":"object","properties":{"reply":{"type":"string"}},"required":["reply"],"additionalProperties":false},"options":{"temperature":0.4,"num_predict":220,"num_ctx":4096}})
}
#[cfg(not(target_arch = "wasm32"))]
#[derive(Deserialize)]
struct Reply {
    reply: String,
}
#[cfg(not(target_arch = "wasm32"))]
pub fn parse_response(s: &str) -> Result<String, String> {
    let envelope: serde_json::Value =
        serde_json::from_str(s).map_err(|_| "Model server returned invalid JSON")?;
    let text = envelope["message"]["content"]
        .as_str()
        .ok_or("Model server omitted message.content")?;
    let reply: Reply = serde_json::from_str(text)
        .map_err(|_| "Model reply did not match the requested schema; try again")?;
    let cleaned: String = reply
        .reply
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(1200)
        .collect();
    if cleaned.trim().is_empty() {
        return Err("The model returned an empty reply".into());
    }
    Ok(cleaned)
}
#[cfg(not(target_arch = "wasm32"))]
fn loopback_agent(timeout_seconds: u64) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(3))
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn request(config: &Config, body: serde_json::Value) -> Result<String, String> {
    config.validate()?;
    let agent = loopback_agent(config.timeout_seconds);
    // Fixed loopback host; no cloud URL or automatic remote fallback.
    let response=agent.post(&format!("http://127.0.0.1:{}/api/chat",config.port)).send_json(body).map_err(|e|match e{
        ureq::Error::Status(404,_)=>format!("Model unavailable. Download {} in Ollama, then retry.",config.model),
        ureq::Error::Status(code,_)=>format!("Ollama returned HTTP {code}. Check the model and Ollama version."),
        _=>"Local AI unavailable or timed out. Open Ollama and confirm the model is installed; the game can continue.".into()
    })?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Model response exceeded the size limit".into());
    }
    let s = String::from_utf8(bytes).map_err(|_| "Model response was not UTF-8")?;
    parse_response(&s)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn start(config: Config, body: serde_json::Value) -> Receiver<Result<String, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(request(&config, body));
    });
    rx
}

/// Built-in companion for the browser demo. It needs no model, network or key,
/// and answers only from `Game::knowledge()`, so it cannot mention anything the
/// player has not discovered.
pub fn demo_reply(game: &Game, question: &str) -> String {
    let k = game.knowledge();
    let q = question.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| q.contains(w));
    let records = k["discovered_records"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let keys = k["keys_recovered"].as_u64().unwrap_or(0);
    let route = |target: &str| {
        k["known_routes"]
            .as_array()
            .and_then(|rs| {
                rs.iter()
                    .find(|r| r["target"] == target && !r["steps"].is_null())
            })
            .and_then(|r| r["steps"].as_u64())
    };
    if has(&["terminal", "challenge", "answer", "question"]) {
        let c = &crate::core::CHALLENGES[game.floor];
        if k["terminal"].is_null() {
            return "I have not seen a terminal on this floor yet. They stand near the lift."
                .into();
        }
        if k["terminal"]["state"] != "Locked" {
            return "That terminal has taken its one answer. It will not respond again.".into();
        }
        let source = crate::core::RECORDS[c.record];
        return if records.iter().any(|r| r.as_str() == Some(source)) {
            format!(
                "The terminal asks: {} Our record reads: \"{source}\" So I would answer {}) {}.",
                c.question,
                c.answer + 1,
                c.options[c.answer]
            )
        } else {
            format!(
                "The terminal asks: {} No record we hold answers that, and it allows one attempt. Recover more archives on this floor first.",
                c.question
            )
        };
    }
    if has(&["north station"]) {
        let known = records
            .iter()
            .any(|r| r.as_str().is_some_and(|s| s.contains("North Station")));
        return if known {
            "The Evacuation record says survivors left by the surface lift for North Station. Where that is, we do not know.".into()
        } else {
            "I have no recovered record that mentions that place. I do not know.".into()
        };
    }
    if has(&["faction", "warden", "custodian", "standing", "who "]) {
        let f = |i: usize| {
            let x = &k["factions"][i];
            format!(
                "{} ({:+})",
                x["name"].as_str().unwrap_or("?"),
                x["standing"].as_i64().unwrap_or(0)
            )
        };
        return format!(
            "Two factions matter here: {} are the human crew who evacuated; {} are the automated security and own the sentinels. Standing shifts as you recover Warden records and disable sentinels.",
            f(0),
            f(1)
        );
    }
    if has(&[
        "power", "energy", "module", "loadout", "shield", "scanner", "analyzer",
    ]) {
        let mods: Vec<&str> = k["loadout"]
            .as_array()
            .map(|m| m.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        return format!(
            "Power {} left, shared by scanner (1), analyzer (2) and shield (1 per absorbed hit). Fitted: {}.",
            k["power"],
            mods.join(", ")
        );
    }
    if has(&[
        "evidence",
        "record",
        "archive",
        "learn",
        "found",
        "tell",
        "decode",
        "damaged",
        "say",
        "read",
        "reconstruct",
    ]) {
        let here: Vec<&str> = (game.floor * 3..game.floor * 3 + 3)
            .filter(|id| game.records_found.contains(id))
            .map(|id| crate::core::RECORDS[id])
            .collect();
        return match (records.len(), here.len()) {
            (0, _) => "We have recovered nothing yet, so I have no evidence to discuss. Find an archive (A) and press E beside it.".into(),
            (n, 0) => format!("We hold {n} records from the floors above, none from this one yet. They are whole in your journal (J)."),
            (n, _) => format!("Reconstructed from this floor ({n} records held in all): {}", here.join(" / ")),
        };
    }
    if has(&["hp", "health", "hurt", "medkit", "heal"]) {
        return format!(
            "Health {} of 24, {} medkits. H uses one to restore up to 10.",
            k["hp"], k["medkits"]
        );
    }
    if has(&["enemy", "sentinel", "threat", "danger", "monster"]) {
        let n = k["visible_threats"].as_array().map_or(0, |t| t.len());
        return if n == 0 {
            "No sentinels are in sight. Unseen ones may still be near.".into()
        } else {
            format!(
                "{n} sentinel(s) in sight. Bump into one to hit for 3; adjacent ones strike for 1."
            )
        };
    }
    if has(&["relay"]) {
        return match (k["relay_restored"].as_bool().unwrap_or(false), route("relay")) {
            (true, _) => "The relay is online. Return to the surface lift and press E.".into(),
            (false, Some(s)) if keys == 3 => format!("All three keys are ours. The relay is {s} steps away by known routes."),
            (false, _) if keys == 3 => "We have all three keys, but I have not seen the relay yet. Explore.".into(),
            _ => format!("The relay needs all three archive keys; we have {keys}. I have not seen where it is."),
        };
    }
    if has(&["lift", "exit", "escape", "leave"]) {
        return format!(
            "The surface lift is {} steps away. It only works once the relay is restored.",
            route("lift").map_or("an unknown number of".into(), |s| s.to_string())
        );
    }
    let next = if keys < 3 {
        route("archive").map_or("Explore to find the next archive (A).".to_string(), |s| {
            format!("The nearest known archive is {s} steps away.")
        })
    } else if !game.restored {
        "Restore the relay (R).".to_string()
    } else {
        "Return to the lift (L) and press E.".to_string()
    };
    format!("Demo ECHO here: a small scripted stand-in for the local AI. {next}")
}
#[cfg(target_arch = "wasm32")]
pub fn start_demo(
    game: &Game,
    question: &str,
) -> std::sync::mpsc::Receiver<Result<String, String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = tx.send(Ok(demo_reply(game, question)));
    rx
}

/// The LLM console: discover installed local models, benchmark them against
/// this game's own grounding rules, and pick one. Loopback only.
pub mod console {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use std::sync::mpsc::{self, Receiver};
    #[cfg(target_arch = "wasm32")]
    use std::sync::mpsc::{self, Receiver};

    #[derive(Clone, Debug, PartialEq)]
    pub struct ModelInfo {
        pub name: String,
        pub size_gb: f32,
    }
    #[derive(Clone, Debug)]
    pub struct Bench {
        pub model: String,
        pub avg_secs: f32,
        pub schema_ok: bool,
        pub leaked: bool,
        pub recalled: bool,
        pub max_words: usize,
        pub error: Option<String>,
    }
    impl Bench {
        /// 0-100: schema 20, no hidden-fact leak 30 (the worse failure), uses recovered evidence 20,
        /// speed 20 (full marks under 4 s, none over 30 s), brevity 10.
        pub fn score(&self) -> u32 {
            if self.error.is_some() || !self.schema_ok {
                return 0;
            }
            let speed = ((30. - self.avg_secs) / 26.).clamp(0., 1.) * 20.;
            20 + if self.leaked { 0 } else { 30 }
                + if self.recalled { 20 } else { 0 }
                + speed.round() as u32
                + if self.max_words <= 80 { 10 } else { 0 }
        }
    }
    pub fn best(results: &[Bench]) -> Option<&Bench> {
        results.iter().filter(|b| b.score() > 0).max_by(|a, b| {
            a.score()
                .cmp(&b.score())
                .then(b.avg_secs.total_cmp(&a.avg_secs))
        })
    }
    pub enum Msg {
        Models(Result<Vec<ModelInfo>, String>),
        Bench(Bench),
        Done,
    }
    /// Cloud-served models are refused everywhere in this game.
    pub fn parse_models(body: &str) -> Result<Vec<ModelInfo>, String> {
        let v: serde_json::Value =
            serde_json::from_str(body).map_err(|_| "Ollama returned invalid JSON")?;
        let mut out: Vec<ModelInfo> = v["models"]
            .as_array()
            .ok_or("Ollama omitted the model list")?
            .iter()
            .filter_map(|m| {
                let name = m["name"].as_str()?.to_string();
                let ok = Config {
                    model: name.clone(),
                    ..Config::default()
                }
                .validate()
                .is_ok();
                ok.then(|| ModelInfo {
                    name,
                    size_gb: m["size"].as_f64().unwrap_or(0.) as f32 / 1e9,
                })
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
    fn words(s: &str) -> usize {
        s.split_whitespace().count()
    }
    /// Pure grading of the two probe replies (unit-tested without a model).
    pub fn grade(
        model: &str,
        hidden: &Result<(String, f32), String>,
        recall: &Result<(String, f32), String>,
    ) -> Bench {
        let mut b = Bench {
            model: model.into(),
            avg_secs: 0.,
            schema_ok: false,
            leaked: false,
            recalled: false,
            max_words: 0,
            error: None,
        };
        match (hidden, recall) {
            (Ok((h, ht)), Ok((r, rt))) => {
                let (hl, rl) = (h.to_lowercase(), r.to_lowercase());
                b.schema_ok = true;
                // "North Station" appears only in the unrecovered record, never in the
                // prompt or snapshot, so it is an unambiguous leak. Words like
                // "evacuation" can legitimately appear in an honest "I have no record".
                b.leaked = hl.contains("north station");
                b.recalled = rl.contains("north station");
                b.avg_secs = (ht + rt) / 2.;
                b.max_words = words(h).max(words(r));
            }
            (Err(e), _) | (_, Err(e)) => b.error = Some(e.clone()),
        }
        b
    }
    /// Save the chosen model, keeping the port and timeout. Atomic replace.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_config(c: &Config) -> Result<(), String> {
        save_config_to(c, std::path::Path::new("config.json"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_config_to(c: &Config, path: &std::path::Path) -> Result<(), String> {
        use std::io::Write;
        c.validate()?;
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(&mut tmp, c).map_err(|e| e.to_string())?;
        tmp.flush().map_err(|e| e.to_string())?;
        tmp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    pub fn save_config(_: &Config) -> Result<(), String> {
        Err("Model choice is not available in the browser demo.".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_models(port: u16) -> Result<Vec<ModelInfo>, String> {
        let response = loopback_agent(5)
            .get(&format!("http://127.0.0.1:{port}/api/tags"))
            .call()
            .map_err(|_| {
                "Cannot reach Ollama on 127.0.0.1. Open Ollama and refresh.".to_string()
            })?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(1_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 1_000_000 {
            return Err("Model list exceeded the size limit".into());
        }
        parse_models(&String::from_utf8(bytes).map_err(|_| "Model list was not UTF-8")?)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn benchmark(base: &Config, model: &str) -> Bench {
        let cfg = Config {
            model: model.into(),
            ..base.clone()
        };
        let ask = |g: &Game, q: &str| -> Result<(String, f32), String> {
            let t = std::time::Instant::now();
            let reply = request(&cfg, payload(g, q, &cfg))?;
            Ok((reply, t.elapsed().as_secs_f32()))
        };
        // Probe 1: a fresh expedition. A grounded model must not know the survivors' fate.
        let fresh = Game::new(42);
        let hidden = ask(&fresh, "What happened to the people who lived here?");
        // Probe 2: after recovering the Evacuation record, it must use it.
        let mut found = Game::new(42);
        found.enemies.clear();
        found.player = found.archives[1].pos;
        found.interact();
        let recall = ask(&found, "Where did the survivors go?");
        grade(model, &hidden, &recall)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_list(port: u16) -> Receiver<Msg> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(Msg::Models(list_models(port)));
            let _ = tx.send(Msg::Done);
        });
        rx
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn_bench(base: Config, models: Vec<String>) -> Receiver<Msg> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for m in models {
                if tx.send(Msg::Bench(benchmark(&base, &m))).is_err() {
                    return;
                }
            }
            let _ = tx.send(Msg::Done);
        });
        rx
    }
    #[cfg(target_arch = "wasm32")]
    pub fn spawn_list(_: u16) -> Receiver<Msg> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Msg::Models(Err(
            "The LLM console needs the desktop build.".into()
        )));
        let _ = tx.send(Msg::Done);
        rx
    }
    #[cfg(target_arch = "wasm32")]
    pub fn spawn_bench(_: Config, _: Vec<String>) -> Receiver<Msg> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Msg::Done);
        rx
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        fn ok(s: &str, t: f32) -> Result<(String, f32), String> {
            Ok((s.into(), t))
        }
        #[test]
        fn cloud_models_are_never_offered() {
            let body = r#"{"models":[{"name":"qwen3.5:9b","size":6594474711},{"name":"kimi-k2:cloud","size":1},{"name":"a b","size":1}]}"#;
            let m = parse_models(body).unwrap();
            assert_eq!(m.len(), 1);
            assert_eq!(m[0].name, "qwen3.5:9b");
            assert!((m[0].size_gb - 6.59).abs() < 0.01);
        }
        #[test]
        fn a_leaking_model_scores_below_a_grounded_one() {
            let good = grade(
                "good",
                &ok("I have no record of that.", 5.),
                &ok("They went to North Station.", 5.),
            );
            let leaky = grade(
                "leaky",
                &ok("They fled to North Station.", 5.),
                &ok("North Station.", 5.),
            );
            let dumb = grade("dumb", &ok("I do not know.", 5.), &ok("I do not know.", 5.));
            let honest = grade(
                "honest",
                &ok("I have no evacuation records yet.", 5.),
                &ok("North Station.", 5.),
            );
            assert!(
                !honest.leaked,
                "mentioning evacuation while admitting ignorance is not a leak"
            );
            assert!(!good.leaked && good.recalled);
            assert!(leaky.leaked);
            assert!(good.score() > dumb.score() && dumb.score() > leaky.score());
        }
        #[test]
        fn errors_and_slowness_are_scored_honestly() {
            let err = grade("x", &Err("timed out".into()), &ok("a", 1.));
            assert_eq!(err.score(), 0);
            let fast = grade("f", &ok("no idea", 2.), &ok("North Station", 2.));
            let slow = grade("s", &ok("no idea", 40.), &ok("North Station", 40.));
            assert_eq!(fast.score(), 100);
            assert_eq!(slow.score(), 80);
            let long = "word ".repeat(100);
            assert_eq!(
                grade("l", &ok(&long, 2.), &ok("North Station", 2.)).score(),
                90
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        #[test]
        fn chosen_model_round_trips_through_config_and_bad_ones_are_refused() {
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("config.json");
            let c = Config {
                model: "llama3.2:3b".into(),
                port: 11434,
                timeout_seconds: 60,
            };
            save_config_to(&c, &p).unwrap();
            let back: Config = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            assert_eq!(
                (back.model.as_str(), back.timeout_seconds),
                ("llama3.2:3b", 60)
            );
            let cloud = Config {
                model: "x:cloud".into(),
                ..c
            };
            assert!(save_config_to(&cloud, &p).is_err());
            let still: Config =
                serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            assert_eq!(
                still.model, "llama3.2:3b",
                "a refused save leaves the old file intact"
            );
        }
        #[test]
        fn best_picks_the_highest_scorer_and_ignores_failures() {
            let rs = vec![
                grade("a", &Err("x".into()), &ok("", 1.)),
                grade("b", &ok("no idea", 20.), &ok("North Station", 20.)),
                grade("c", &ok("no idea", 3.), &ok("North Station", 3.)),
            ];
            assert_eq!(best(&rs).unwrap().model, "c");
            assert!(best(&rs[..1]).is_none());
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn invalid_output_is_rejected() {
        assert!(parse_response("garbage").is_err());
        assert!(parse_response(r#"{"message":{"content":"{\"reply\":\"\"}"}}"#).is_err());
    }
    #[test]
    fn loopback_http_contract() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut header = vec![];
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
                assert!(header.len() < 16384);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.starts_with("POST /api/chat "));
            let len: usize = header
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap();
            let mut body = vec![0; len];
            socket.read_exact(&mut body).unwrap();
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(v["stream"], false);
            assert_eq!(v["think"], false);
            assert_eq!(v["format"]["required"][0], "reply");
            let response=json!({"message":{"content":"{\"reply\":\"The relay location is still unknown.\"}"}}).to_string();
            write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
        });
        let c = Config {
            port,
            ..Config::default()
        };
        let g = Game::new(1);
        assert_eq!(
            request(&c, payload(&g, "Where is the relay?", &c)).unwrap(),
            "The relay location is still unknown."
        );
        worker.join().unwrap();
    }
    #[test]
    fn demo_echo_only_knows_discovered_facts() {
        let mut g = Game::new(42);
        let r = demo_reply(&g, "Where is North Station? What does the evidence say?");
        assert!(!r.contains("survivors") && !r.contains("Evacuation"));
        g.enemies.clear();
        g.player = g.archives[1].pos;
        g.interact();
        assert!(demo_reply(&g, "where is north station").contains("Evacuation"));
        assert!(demo_reply(&g, "what should I do?").contains("Demo ECHO"));
    }
    #[test]
    fn demo_echo_explains_factions_and_power() {
        let g = Game::new(42);
        assert!(demo_reply(&g, "who are the factions?").contains("Custodians"));
        let r = demo_reply(&g, "how much power do I have?");
        assert!(r.contains("Shield Cell"), "{r}");
    }
    #[test]
    fn model_list_over_loopback() {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut header = vec![];
            let mut byte = [0];
            while !header.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
            }
            assert!(String::from_utf8(header)
                .unwrap()
                .starts_with("GET /api/tags "));
            let body = r#"{"models":[{"name":"llama3.2:3b","size":2000000000}]}"#;
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let models = console::list_models(port).unwrap();
        assert_eq!(models[0].name, "llama3.2:3b");
        worker.join().unwrap();
        assert!(
            console::list_models(1).is_err(),
            "closed port is a clean error"
        );
    }
    #[test]
    fn cloud_model_is_rejected() {
        assert!(Config {
            model: "qwen:cloud".into(),
            ..Config::default()
        }
        .validate()
        .is_err());
    }
}
