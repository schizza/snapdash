use crate::ha::types::{EntityState, HaEvent};
use crate::ha::{HaConnectionConfig, rest};
use iced::futures::{SinkExt, StreamExt};
use iced::stream;
use serde_json::json;
use std::time::Duration;
use tokio::time::{Instant, interval, sleep};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::Utf8Bytes;

#[derive(Debug, serde::Deserialize)]
struct Incoming {
    #[serde(default)]
    r#type: String,

    #[serde(default)]
    event: Option<EventPayload>,
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
        // fallback: assume https
        format!("wss://{base}/api/websocket")
    }
}

pub fn connect(config: &HaConnectionConfig) -> iced::futures::stream::BoxStream<'static, HaEvent> {
    let ha_url = config.url.clone();
    let token = config.token.clone();

    stream::channel(100, async move |mut output| {
        loop {
            let ws_url = ws_url_from_http(&ha_url);

            let (ws, _resp) = match tokio_tungstenite::connect_async(&ws_url).await {
                Ok(v) => v,
                Err(e) => {
                    let _ = output
                        .send(HaEvent::Disconnected(format!("connect: {e}")))
                        .await;
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }
            };

            let (mut sink, mut stream) = ws.split();

            while let Some(msg) = stream.next().await {
                let Ok(msg) = msg else { continue };
                let Ok(text) = msg.to_text() else { continue };

                if text.contains("\"auth_required\"") {
                    break;
                }
            }

            let auth = json!({"type": "auth", "access_token": token});

            if sink
                .send(WsMessage::Text(Utf8Bytes::from(auth.to_string())))
                .await
                .is_err()
            {
                let _ = output
                    .send(HaEvent::Disconnected("auth send failed".into()))
                    .await;
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            let mut authed = false;

            while let Some(msg) = stream.next().await {
                let Ok(msg) = msg else { continue };
                let Ok(text) = msg.to_text() else { continue };

                if text.contains("\"auth_ok\"") {
                    authed = true;
                    break;
                }

                if text.contains("\"auth_invalid\"") {
                    let _ = output
                        .send(HaEvent::Disconnected("auth_invalid".into()))
                        .await;
                    break;
                }
            }

            if !authed {
                sleep(Duration::from_secs(3)).await;
                continue;
            }

            let sub = json!({
                "id": 1,
                "type": "subscribe_events",
                "event_type": "state_changed"
            });

            if sink
                .send(WsMessage::Text(Utf8Bytes::from(sub.to_string())))
                .await
                .is_err()
            {
                let _ = output
                    .send(HaEvent::Disconnected("subscribe failed".into()))
                    .await;
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            let _ = output.send(HaEvent::Connected).await;

            let initial_states = rest::fetch_all_states(ha_url.clone(), token.clone()).await;
            let _ = output.send(HaEvent::InitialState(initial_states)).await;

            let mut heartbeat = interval(Duration::from_secs(30));
            let mut last_received = Instant::now();

            loop {
                tokio::select! {
                    msg = stream.next() => {
                        let Some(msg) = msg else { break };
                        let Ok(msg) = msg else { continue };

                        last_received = Instant::now();

                        let Ok(text) = msg.to_text() else { continue };

                        if let Ok(parsed) = serde_json::from_str::<Incoming>(text)
                        && parsed.r#type == "event"
                        && let Some(ev) = parsed.event
                        && ev.event_type == "state_changed"
                        && let Some(data) = ev.data
                        && let Some(new_state) = data.new_state
                        {
                            let _ = output.send(HaEvent::StateChanged { new_state}).await;
                        }
                    }
                    _ = heartbeat.tick() => {
                            if last_received.elapsed() > Duration::from_secs(90) {
                                break;
                            }
                            if sink.send(WsMessage::Ping(vec![].into())).await.is_err()
                            { break; }
                        }
                }
            }

            // while let Some(msg) = stream.next().await {
            //     let Ok(msg) = msg else { continue };
            //     let Ok(text) = msg.to_text() else { continue };
            //
            //     if let Ok(parsed) = serde_json::from_str::<Incoming>(text)
            //         && parsed.r#type == "event"
            //         && let Some(ev) = parsed.event
            //         && ev.event_type == "state_changed"
            //         && let Some(data) = ev.data
            //         && let Some(new_state) = data.new_state
            //     {
            //         let _ = output.send(HaEvent::StateChanged { new_state }).await;
            //     }
            // }

            let _ = output.send(HaEvent::Disconnected("ws closed".into())).await;
            sleep(Duration::from_secs(2)).await;
        }
    })
    .boxed()
}
