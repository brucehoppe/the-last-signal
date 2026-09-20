use crate::core::Game;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
#[derive(Deserialize)]
struct Reply {
    reply: String,
}
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
pub fn start(config: Config, body: serde_json::Value) -> Receiver<Result<String, String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(request(&config, body));
    });
    rx
}

#[cfg(test)]
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
    fn cloud_model_is_rejected() {
        assert!(Config {
            model: "qwen:cloud".into(),
            ..Config::default()
        }
        .validate()
        .is_err());
    }
}
