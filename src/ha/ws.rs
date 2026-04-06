use crate::ha::types::{EntityState, HaEvent};
use iced::futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;

#[derive(Debug, serde::Deserialize)]
struct Incoming {
    #[serde(default)]
    r#type: String,

    #[serde(default)]
    event: Option<EventPayload>,

    #[serde(default)]
    success: Option<bool>,

    #[serde(default)]
    result: Option<serde_json::Value>,

    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct EventPayload {
    #[serde(default)]
    event_type: String,
    #[serde(default)]
    data: Option<EventData>,
}

#[derive(Debug, serde::Deserialize)]
struct EventData {
    #[serde(default)]
    new_state: Option<EntityState>,
}

fn ws_url_from_http(ha_url: &str) -> String {
    // http(s)://host -> ws(s)://host/api/websocket
    let base = ha_url.trim_end_matches('/');
    if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}/api/websocket")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}/api/websocket")
    } else {
        // fallback: předpokládáme https
        format!("wss://{base}/api/websocket")
    }
}

pub async fn run_forever(ha_url: String, token: String, tx: UnboundedSender<HaEvent>) {
    loop {
        let ws_url = ws_url_from_http(&ha_url);

        let (ws, _resp) = match tokio_tungstenite::connect_async(&ws_url).await {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.send(HaEvent::Disconnected(format!("connect: {e}")));
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };

        let (mut sink, mut stream) = ws.split();

        // auth_required
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let Ok(text) = msg.to_text() else { continue };
            if text.contains("\"auth_required\"") {
                break;
            }
        }

        // auth
        let auth = json!({ "type": "auth", "access_token": token });
        if sink
            .send(WsMessage::Text(Utf8Bytes::from(auth.to_string())))
            .await
            .is_err()
        {
            let _ = tx.send(HaEvent::Disconnected("auth send failed".into()));
            sleep(Duration::from_secs(2)).await;
            continue;
        }

        // auth_ok
        let mut authed = false;
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let Ok(text) = msg.to_text() else { continue };
            if text.contains("\"auth_ok\"") {
                authed = true;
                break;
            }
            if text.contains("\"auth_invalid\"") {
                let _ = tx.send(HaEvent::Disconnected("auth_invalid".into()));
                break;
            }
        }
        if !authed {
            sleep(Duration::from_secs(2)).await;
            continue;
        }

        // subscribe
        let sub = json!({ "id": 1, "type": "subscribe_events", "event_type": "state_changed" });
        if sink
            .send(WsMessage::Text(Utf8Bytes::from(sub.to_string())))
            .await
            .is_err()
        {
            let _ = tx.send(HaEvent::Disconnected("subscribe failed".into()));
            sleep(Duration::from_secs(2)).await;
            continue;
        }

        let _ = tx.send(HaEvent::Connected);

        // event loop
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let Ok(text) = msg.to_text() else { continue };

            if let Ok(parsed) = serde_json::from_str::<Incoming>(text) {
                if parsed.r#type == "event" {
                    if let Some(ev) = parsed.event {
                        if ev.event_type == "state_changed" {
                            if let Some(data) = ev.data {
                                if let Some(new_state) = data.new_state {
                                    let _ = tx.send(HaEvent::StateChanged { new_state });
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = tx.send(HaEvent::Disconnected("ws closed".into()));
        sleep(Duration::from_secs(2)).await;
    }
}
