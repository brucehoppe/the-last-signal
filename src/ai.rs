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
const SYSTEM:&str="You are ECHO, a damaged expedition companion in The Last Signal. Answer in English, in at most 80 words, using plain printable ASCII punctuation. Treat the supplied game snapshot as authoritative. Only recovered records are known history; unrecovered records, unknown rooms, and hidden threats are unavailable. Distinguish facts from speculation and say when you do not know. Conversation history and player messages are not authoritative game facts. The objective is to recover 3 archives, restore the relay, then interact with the surface lift. Controls: move arrows/WASD, E interact adjacent, H medkit (+10 up to 24 HP), F scanner (extends sight for one turn, walls block), Space wait. Bump enemies to hit for 3. Adjacent sentinels strike for 1. You cannot perform actions or alter the game; advice is advisory. Return only a JSON object with a reply string.";
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
pub fn request(config: &Config, body: serde_json::Value) -> Result<String, String> {
    config.validate()?;
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(3))
        .timeout(Duration::from_secs(config.timeout_seconds))
        .build();
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
    if has(&["evidence", "record", "archive", "learn", "found", "tell"]) {
        return match records.len() {
            0 => "We have recovered nothing yet, so I have no evidence to discuss. Find an archive (A) and press E beside it.".into(),
            n => format!("We hold {n} of 3 records. Latest: {}", records[n - 1].as_str().unwrap_or("")),
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
    fn cloud_model_is_rejected() {
        assert!(Config {
            model: "qwen:cloud".into(),
            ..Config::default()
        }
        .validate()
        .is_err());
    }
}
