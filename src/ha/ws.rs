use crate::ha::types::{EntityState, HaError, HaEvent};
use crate::ha::{HaConnectionConfig, rest};
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::stream;
use rand::RngExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;
use tokio::time::{Instant, interval, sleep};
use tokio_tungstenite::tungstenite::{Bytes, Message as WsMessage, Utf8Bytes};
use url::Url;

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = iced::futures::stream::SplitSink<Ws, WsMessage>;
type WsStream = iced::futures::stream::SplitStream<Ws>;

const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const BACKOFF_JITTER_MAX: Duration = Duration::from_secs(500);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const STALE_AFTER: Duration = Duration::from_secs(90);
const AUTH_RETRY_DELAY: Duration = Duration::from_secs(2);
const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const MAX_AUTH_FAILURES: u8 = 2;
const EVENT_CHANNEL_CAPACITY: usize = 100;
const SUBSCRIBE_ID: u64 = 1;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

fn jitter() -> Duration {
    let max_ms = BACKOFF_JITTER_MAX.as_millis() as u64;
    let ms = rand::rng().random_range(0..=max_ms);
    Duration::from_millis(ms)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[expect(
    dead_code,
    reason = "fields kept for Debug output and wire-contract symmetry"
)]
enum ServerMsg {
    AuthRequired {
        #[serde(default)]
        ha_version: String,
    },
    AuthOk {
        #[serde(default)]
        ha_version: String,
    },
    AuthInvalid {
        #[serde(default)]
        message: String,
    },
    Event {
        #[serde(default)]
        id: Option<u64>,
        event: EventPayload,
    },
    Result {
        id: u64,
        success: bool,
        #[serde(default)]
        error: Option<serde_json::Value>,
    },
    Pong {
        id: u64,
    },

    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg<'a> {
    Auth { access_token: &'a str },
    SubscribeEvents { id: u64, event_type: &'a str },
}

#[derive(Debug, Deserialize)]
struct EventPayload {
    event_type: String,
    #[serde(default)]
    data: EventData,
}

#[derive(Debug, Deserialize, Default)]
struct EventData {
    #[serde(default)]
    new_state: Option<EntityState>,
}

fn ws_url_from_http(ha_url: &str) -> Result<Url, HaError> {
    let mut url = Url::parse(ha_url).map_err(|e| HaError::Protocol(format!("invalid URL: {e}")))?;

    let ws_scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => {
            return Err(HaError::Protocol(format!(
                "expected http(s):// URL, got scheme {other:?}"
            )));
        }
    };

    url.set_scheme(ws_scheme)
        .map_err(|()| HaError::Protocol("failed to switch URL scheme to WebSocket".into()))?;

    let base = url.path().trim_matches('/');
    let new_path = if base.is_empty() {
        "/api/websocket".to_owned()
    } else {
        format!("/{base}/api/websocket")
    };

    url.set_path(&new_path);
    Ok(url)
}

#[tracing::instrument(skip_all, fields(url = %ws_url))]
async fn open_session(token: &str, ws_url: &Url) -> Result<(WsSink, WsStream), HaError> {
    let (ws, _) = tokio_tungstenite::connect_async(ws_url.as_str())
        .await
        .map_err(|e| HaError::Connect(e.to_string()))?;

    let (mut sink, mut stream) = ws.split();

    handshake(&mut sink, &mut stream, token).await?;
    subscribe(&mut sink).await?;

    Ok((sink, stream))
}

async fn send_msg(
    sink: &mut WsSink,
    msg: &ClientMsg<'_>,
    what: &'static str,
) -> Result<(), HaError> {
    let json = serde_json::to_string(msg).map_err(|e| HaError::Protocol(e.to_string()))?;

    // do not log auth token!
    if !matches!(msg, ClientMsg::Auth { .. }) {
        tracing::trace!(what, payload = %json, "→ send");
    } else {
        tracing::trace!(what, "→ send (payload redacted)");
    }
    sink.send(WsMessage::Text(Utf8Bytes::from(json)))
        .await
        .map_err(|_| HaError::SendFailed { what })
}

async fn read_msg<T: serde::de::DeserializeOwned>(stream: &mut WsStream) -> Result<T, HaError> {
    loop {
        match stream.next().await {
            Some(Err(e)) => return Err(HaError::Protocol(e.to_string())),
            Some(Ok(WsMessage::Close(_))) | None => return Err(HaError::Closed),
            Some(Ok(WsMessage::Text(text))) => {
                tracing::trace!(raw = text.as_str(), "← recv");
                return serde_json::from_str(text.as_str())
                    .map_err(|e| HaError::Protocol(e.to_string()));
            }
            // binary, ping, pong -- tungsteinite will respond on his own
            Some(Ok(_)) => {}
        }
    }
}

async fn expect<T: DeserializeOwned>(
    stream: &mut WsStream,
    timeout_dur: Duration,
    what: &'static str,
) -> Result<T, HaError> {
    tokio::time::timeout(timeout_dur, read_msg(stream))
        .await
        .map_err(|_| HaError::Timeout { what })?
}

#[tracing::instrument(skip_all)]
async fn handshake(sink: &mut WsSink, stream: &mut WsStream, token: &str) -> Result<(), HaError> {
    // expect auth_required
    tracing::debug!("waiting for auth_required");
    match expect::<ServerMsg>(stream, HANDSHAKE_TIMEOUT, "auth_required").await? {
        ServerMsg::AuthRequired { ha_version } => {
            tracing::info!(%ha_version, "authenticating")
        }
        other => {
            return Err(HaError::Protocol(format!(
                "expected auth_required, got {other:?}"
            )));
        }
    }

    // send auth
    tracing::debug!("sending auth");
    send_msg(
        sink,
        &ClientMsg::Auth {
            access_token: token,
        },
        "auth",
    )
    .await?;

    // expect auth_ok / auth_invalid

    tracing::debug!("waiting for auth result");
    match expect::<ServerMsg>(stream, HANDSHAKE_TIMEOUT, "auth_result").await? {
        ServerMsg::AuthOk { .. } => {
            tracing::debug!("auth succeeded");
            Ok(())
        }
        ServerMsg::AuthInvalid { message } => Err(HaError::AuthInvalid(message)),
        other => Err(HaError::Protocol(format!(
            "unexpected during auth: {other:?}"
        ))),
    }
}

#[tracing::instrument(skip_all)]
async fn subscribe(sink: &mut WsSink) -> Result<(), HaError> {
    send_msg(
        sink,
        &ClientMsg::SubscribeEvents {
            id: SUBSCRIBE_ID,
            event_type: "state_changed",
        },
        "subscribe",
    )
    .await
}

#[tracing::instrument(skip_all)]
async fn pump_events(
    sink: &mut WsSink,
    stream: &mut WsStream,
    out: &mut iced::futures::channel::mpsc::Sender<HaEvent>,
) -> HaError {
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    let mut last_received = Instant::now();

    loop {
        tokio::select! {
            // we have a payload
            frame = stream.next() => {
                let frame = match frame {
                    None => return HaError::Closed,
                    Some(Err(e)) => return HaError::Protocol(e.to_string()),
                    Some(Ok(f)) => f,
                };

                last_received = Instant::now();

                let text = match frame {
                    WsMessage::Text(t) => t,
                    WsMessage::Close(_) => return HaError::Closed,
                    _ => continue,
                };

                let msg: ServerMsg = match serde_json::from_str(text.as_str()) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(error = %e, raw = text.as_str(), "failed to decode ServerMsg - skipping");
                        continue;
                    }
                };

                if let ServerMsg::Event { event, .. } = msg
                    && event.event_type == "state_changed"
                    && let Some(new_state) = event.data.new_state
                {
                    tracing::trace!(
                        entity_id = %new_state.entity_id,
                        state = %new_state.state,
                        "state_changed"
                    );
                    let _ = out.send(HaEvent::StateChanged { new_state }).await;
                }
            }

            _ = heartbeat.tick() => {
                if last_received.elapsed() > STALE_AFTER {
                    return HaError::Stale { elapsed: last_received.elapsed() };
                }
                tracing::trace!("sending heartbeat ping");
                if sink.send(WsMessage::Ping(Bytes::new())).await.is_err() {
                    return HaError::SendFailed { what: "ping" };
                }
            }
        }
    }
}

pub fn connect(config: &HaConnectionConfig) -> BoxStream<'static, HaEvent> {
    let cfg = config.clone();

    stream::channel(EVENT_CHANNEL_CAPACITY, async move |mut out| {
        let ws_url = match ws_url_from_http(&cfg.url) {
            Ok(u) => u,
            Err(e) => {
                tracing::error!(url= %cfg.url, error = %e, "invalid HA URL - stream ending");
                let _ = out.send(HaEvent::Disconnected(e)).await;
                return;
            }
        };

        let mut auth_failures: u8 = 0;
        let mut backoff = INITIAL_BACKOFF;

        loop {
            let (mut sink, mut stream) = match open_session(&cfg.token, &ws_url).await {
                Ok(ws) => {
                    tracing::info!("connected and subscribed to state_changed events");
                    auth_failures = 0;
                    backoff = INITIAL_BACKOFF;
                    ws
                }
                Err(e @ HaError::AuthInvalid(_)) => {
                    auth_failures += 1;
                    let _ = out.send(HaEvent::Disconnected(e)).await;

                    if auth_failures >= MAX_AUTH_FAILURES {
                        tracing::error!(
                            attempts = auth_failures,
                            "authentication exhausted - giving up"
                        );
                        let _ = out
                            .send(HaEvent::AuthFailed(HaError::AuthExhausted {
                                attempts: auth_failures,
                            }))
                            .await;
                        return; // stream ended
                    }
                    tracing::warn!(
                        attempt = auth_failures,
                        max = MAX_AUTH_FAILURES,
                        "authentication rejected - retrying"
                    );
                    sleep(AUTH_RETRY_DELAY).await;
                    continue;
                }
                Err(e) => {
                    let jittered = backoff + jitter();
                    tracing::warn!(
                        error = %e, 
                        base_s = backoff.as_secs(), 
                        jittered_ms = jittered.as_millis(),
                        "reconnecting"
                    );
                    let _ = out.send(HaEvent::Disconnected(e)).await;
                    sleep(jittered).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }
            };

            // Autenticated + subscribed
            let _ = out.send(HaEvent::Connected).await;
            let initial = rest::fetch_all_states(&cfg.url, &cfg.token).await;

            tracing::debug!(count = initial.len(), "received initial states");

            let _ = out.send(HaEvent::InitialState(initial)).await;

            // run Forrest, run!
            let reason = pump_events(&mut sink, &mut stream, &mut out).await;
            let _ = out.send(HaEvent::Disconnected(reason)).await;
            sleep(RECONNECT_DELAY).await;
        }
    })
    .boxed()
}

// TESTS

#[cfg(test)]
mod tests;
