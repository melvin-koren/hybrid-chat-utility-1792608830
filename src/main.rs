use std::io::{self, BufRead};
use std::process::Command;
use std::time::Duration;
use tokio::net::{TcpListener, UdpSocket};
use tokio::time::timeout;
use base64::{engine::general_purpose, Engine as _};

const TARGET_IP: &str = "127.0.0.1";
const HTTP_PORT: u16 = 8080;
const SIGNAL_PORT: u16 = 8889;
const ACK_TIMEOUT: Duration = Duration::from_secs(3);
const TG_CHANNEL_NAME: &str = "synthpower_covert_test";
const TG_BOT_TOKEN: &str = "7324892472:AAElFakeTokenForDemoPurposeOnly12345";

static mut MY_ID: &str = "CLIENT_A";
static mut THEIR_ID: &str = "CLIENT_B";

fn covert_encode(sender: &str, text: &str) -> String {
    let raw_str = format!("{}:{}", sender, text);
    general_purpose::STANDARD.encode(raw_str)
}

fn covert_decode(encoded_str: &str) -> Option<(String, String)> {
    if let Ok(decoded_bytes) = general_purpose::STANDARD.decode(encoded_str) {
        if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
            if let Some((sender, text)) = decoded_str.split_once(':') {
                return Some((sender.to_string(), text.to_string()));
            }
        }
    }
    None
}

async fn start_http_listener() {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", HTTP_PORT)).await { Ok(l) => l, Err(_) => return, };
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0; 2048];
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            if let Ok(n) = stream.read(&mut buf).await {
                let request = String::from_utf8_lossy(&buf[..n]);
                for line in request.lines() {
                    if line.starts_with("User-Agent: Covert-Agent: ") {
                        let encoded = line.replace("User-Agent: Covert-Agent: ", "");
                        if let Some((sender, text)) = covert_decode(encoded.trim()) {
                            unsafe { if sender == THEIR_ID { println!("\n[📩 HTTP] {}: {}", sender, text); } }
                        }
                    }
                }
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
            }
        }
    }
}

async fn start_icmp_signal_listener() {
    let socket = match UdpSocket::bind(format!("0.0.0.0:{}", SIGNAL_PORT)).await { Ok(s) => s, Err(_) => return, };
    let mut buf = [0; 1024];
    loop {
        if let Ok((len, addr)) = socket.recv_from(&mut buf).await {
            let payload = String::from_utf8_lossy(&buf[..len]);
            if payload.starts_with("COVERT:") {
                let encoded = payload.replace("COVERT:", "");
                if let Some((sender, text)) = covert_decode(encoded.trim()) {
                    unsafe {
                        if sender == THEIR_ID {
                            println!("\n[📩 ICMP/Ping] {}: {}", sender, text);
                            let _ = socket.send_to(b"COVERT_ACK", addr).await;
                        }
                    }
                }
            }
        }
    }
}

async fn start_telegram_listener() {
    let client = reqwest::Client::new();
    let url = format!("https://t.me{}", TG_CHANNEL_NAME);
    let mut seen_messages = std::collections::HashSet::new();
    loop {
        if let Ok(res) = client.get(&url).timeout(Duration::from_secs(4)).send().await {
            if res.status().is_success() {
                if let Ok(html) = res.text().await {
                    let marker = "TG_COVERT_";
                    let mut current_pos = 0;
                    while let Some(start_idx) = html[current_pos..].find(marker) {
                        let absolute_start = current_pos + start_idx + marker.len();
                        let end_idx = html[absolute_start..].find(|c: char| !c.is_alphanumeric() && c != '+' && c != '/' && c != '=');
                        if let Some(len) = end_idx {
                            let encoded = &html[absolute_start..absolute_start + len];
                            if !seen_messages.contains(encoded) {
                                seen_messages.insert(encoded.to_string());
                                if let Some((sender, text)) = covert_decode(encoded) {
                                    unsafe { if sender == THEIR_ID { println!("\n[📩 TG Dead Drop] {}: {}", sender, text); } }
                                }
                            }
                            current_pos = absolute_start + len;
                        } else { break; }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn try_http(encoded: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://{}:{}/", TARGET_IP, HTTP_PORT);
    let res = client.get(&url).header("User-Agent", format!("Covert-Agent: {}", encoded)).timeout(ACK_TIMEOUT).send().await;
    res.is_ok() && res.unwrap().status().is_success()
}

async fn try_icmp(encoded: &str) -> bool {
    let is_windows = cfg!(target_os = "windows");
    let mut cmd = Command::new("ping");
    if is_windows { cmd.args(["-n", "1", "-l", "64", TARGET_IP]); } else { cmd.args(["-c", "1", "-s", "64", TARGET_IP]); }
    let _ = cmd.output();
    let socket = match UdpSocket::bind("0.0.0.0:0").await { Ok(s) => s, Err(_) => return false, };
    if socket.send_to(format!("COVERT:{}", encoded).as_bytes(), format!("{}:{}", TARGET_IP, SIGNAL_PORT)).await.is_err() { return false; }
    let mut buf = [0; 64];
    match timeout(ACK_TIMEOUT, socket.recv_from(&mut buf)).await {
        Ok(Ok((len, _))) => &buf[..len] == b"COVERT_ACK",
        _ => false,
    }
}

async fn try_telegram(encoded: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("https://telegram.org{}/sendMessage", TG_BOT_TOKEN);
    let params = serde_json::json!({"chat_id": format!("@{}", TG_CHANNEL_NAME), "text": format!("TG_COVERT_{}", encoded)});
    let res = client.post(&url).json(&params).timeout(Duration::from_secs(5)).send().await;
    res.is_ok() && res.unwrap().status().is_success()
}

async fn send_message_auto(message: &str) {
    let encoded = unsafe { covert_encode(MY_ID, message) };
    println!("[⚙️] Поиск рабочего канала...");
    if try_http(&encoded).await { println!(" ➔ [🟢 УСПЕХ] Доставлено по HTTP."); return; }
    if try_icmp(&encoded).await { println!(" ➔ [🟢 УСПЕХ] Доставлено по ICMP."); return; }
    if try_telegram(&encoded).await { println!(" ➔ [🟢 УСПЕХ] Доставлено в Telegram."); return; }
    println!("[❌ ОШИБКА] Все каналы связи заблокированы.");
}

#[tokio::main]
async fn main() {
    println!("=== ГИБРИДНЫЙ RUST МЕССЕНДЖЕР ===");
    println!("Выберите профиль:\n1 - Клиент А\n2 - Клиент Б");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();
    if choice.trim() == "2" { unsafe { MY_ID = "CLIENT_B"; THEIR_ID = "CLIENT_A"; } }
    tokio::spawn(start_http_listener());
    tokio::spawn(start_icmp_signal_listener());
    tokio::spawn(start_telegram_listener());
    println!("[🔥 СИСТЕМА ГОТОВА] Пишите сообщения:");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(text) = line {
            if text.trim().to_lowercase() == "exit" { break; }
            if !text.trim().is_empty() { send_message_auto(&text).await; }
        }
    }
}
