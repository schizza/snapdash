//! Seam S2: the application driven by messages, against a fake Home
//! Assistant speaking real HTTP.
//!
//! The unit seams below this one can say what a `Capabilities` holds or
//! what a `PendingValues` returns, but they cannot say what the user
//! ends up looking at or what Snapdash ends up putting on the wire. This
//! one can: it drives [`Snapdash::update`] with the same messages the
//! runtime delivers, and points the connection at a `wiremock` server so
//! a service call is a real POST whose body can be read back.
//!
//! Two things make that possible without launching a window system.
//! `Snapdash::new` takes no arguments and its fields are public, so the
//! connection can be pointed anywhere. And `call_service` builds its URL
//! from that connection rather than from configuration, so `127.0.0.1`
//! is as valid a Home Assistant as any.
//!
//! Attribute blobs are copied from what a real Home Assistant sends
//! rather than reduced to the keys a given test happens to read. The
//! whole class of defect this seam exists to catch is Snapdash
//! misreading something Home Assistant actually says, and a blob trimmed
//! to what the code already handles cannot catch that.

use serde_json::{Value, json};
use snapdash::app::{Message, Snapdash};
use snapdash::ha::{EntityState, HaConnectionConfig, HaEvent};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use iced_winit::futures::futures::StreamExt;
use iced_winit::runtime::{Action, task};

/// Home Assistant's `state_changed` payload for one entity, built from a
/// real attribute blob.
pub fn entity_state(entity_id: &str, state: &str, attributes: Value) -> EntityState {
    EntityState {
        entity_id: entity_id.to_owned(),
        state: state.to_owned(),
        attributes: serde_json::from_value(attributes).expect("attributes are a JSON object"),
        last_changed: None,
        last_updated: None,
    }
}

/// A `Snapdash` whose Home Assistant is a local HTTP server.
pub struct Harness {
    pub app: Snapdash,
    server: MockServer,
}

impl Harness {
    /// Boot the application connected to a fake Home Assistant that
    /// accepts every service call.
    ///
    /// Home Assistant answers `POST /api/services/{domain}/{service}`
    /// with the list of states the call changed. Snapdash ignores the
    /// body and waits for the `state_changed` broadcast instead, so an
    /// empty list is a faithful enough reply.
    pub async fn new() -> Self {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"^/api/services/[^/]+/[^/]+$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let mut app = Snapdash::new();
        app.ha.connection = Some(HaConnectionConfig {
            url: server.uri(),
            token: "test-token".to_owned(),
        });
        app.ha.connected = true;

        Self { app, server }
    }

    /// Deliver one message and run whatever it hands back to the
    /// runtime, returning the messages that came out.
    ///
    /// Running the task is the point: a `Task::perform` is an inert
    /// stream until something polls it, so without this the HTTP request
    /// inside a service call would never leave. The messages it produces
    /// are returned rather than fed back in, so a test stays in charge
    /// of how far a single interaction is allowed to cascade.
    pub async fn send(&mut self, message: Message) -> Vec<Message> {
        let Some(stream) = task::into_stream(self.app.update(message)) else {
            return Vec::new();
        };

        stream
            .filter_map(|action| async move {
                match action {
                    Action::Output(message) => Some(message),
                    _ => None,
                }
            })
            .collect()
            .await
    }

    /// Feed the `state_changed` Home Assistant broadcasts for an entity,
    /// exactly as the WebSocket feed does.
    pub async fn state_changed(&mut self, entity_id: &str, state: &str, attributes: Value) {
        self.send(Message::HaEvent(HaEvent::StateChanged {
            new_state: entity_state(entity_id, state, attributes),
        }))
        .await;
    }

    /// Every service call posted so far, as `(path, body)`.
    pub async fn service_calls(&self) -> Vec<(String, Value)> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|request: &Request| {
                (
                    request.url.path().to_owned(),
                    serde_json::from_slice(&request.body).expect("a JSON body"),
                )
            })
            .collect()
    }
}

/// What Home Assistant reports for a colour-temperature bulb that is
/// **off**.
///
/// Every axis is `null`: no brightness, no colour temperature, no colour
/// mode. The static capability attributes stay, which is why the widget
/// still knows the bulb is dimmable while it is off.
pub fn light_off_attributes() -> Value {
    json!({
        "min_color_temp_kelvin": 2202,
        "max_color_temp_kelvin": 6535,
        "min_mireds": 153,
        "max_mireds": 454,
        "effect_list": ["None", "candle"],
        "supported_color_modes": ["color_temp", "hs"],
        "color_mode": null,
        "brightness": null,
        "color_temp_kelvin": null,
        "color_temp": null,
        "hs_color": null,
        "rgb_color": null,
        "xy_color": null,
        "effect": null,
        "friendly_name": "Living room",
        "supported_features": 44
    })
}

/// What Home Assistant reports for a bulb whose native colour mode is
/// **rgb**, on and showing a colour.
///
/// The two arguments have to be given together because they are the same
/// fact stated twice. `rgb_color` is what the light actually stores, and
/// Home Assistant *derives* `hs_color` and `xy_color` from it on every
/// state build. A hue that goes out as an integer therefore comes back
/// through 8-bit RGB, and not as the number Snapdash sent - which is the
/// whole reason this seam needs an rgb light and not only the `hs` one
/// above. A template light backed by `input_number` helpers stores hue
/// verbatim and would confirm any tolerance at all.
///
/// Note the absence of the `color_temp` and `mireds` keys: Home
/// Assistant only reports those for a light that advertises the
/// `color_temp` mode, and this one does not.
pub fn rgb_light_attributes(rgb_color: [u8; 3], hs_color: [f64; 2], xy_color: [f64; 2]) -> Value {
    json!({
        "supported_color_modes": ["rgb"],
        "color_mode": "rgb",
        "brightness": 199,
        "hs_color": hs_color,
        "rgb_color": rgb_color,
        "xy_color": xy_color,
        "friendly_name": "Desk lamp",
        "supported_features": 0
    })
}

/// The same bulb **on**, sitting in `color_temp` mode.
pub fn light_on_attributes() -> Value {
    json!({
        "min_color_temp_kelvin": 2202,
        "max_color_temp_kelvin": 6535,
        "min_mireds": 153,
        "max_mireds": 454,
        "effect_list": ["None", "candle"],
        "supported_color_modes": ["color_temp", "hs"],
        "color_mode": "color_temp",
        "brightness": 172,
        "color_temp_kelvin": 2703,
        "color_temp": 370,
        "hs_color": [28.391, 65.659],
        "rgb_color": [255, 167, 87],
        "xy_color": [0.524, 0.387],
        "effect": null,
        "friendly_name": "Living room",
        "supported_features": 44
    })
}
