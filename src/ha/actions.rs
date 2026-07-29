//! Outbound service calls to Home Assistant.
//!
//! Actionable widgets (issue #81) let the user act on a widget without
//! opening Home Assistant. This module owns two separate concerns that
//! phase 1 was able to conflate and phase 2 cannot:
//!
//! - **Discovery** - [`Capabilities`], "what can this entity do?".
//!   Answered from the entity's *attributes*, because the domain alone
//!   does not know it: not every `light` is dimmable and not every
//!   `cover` can be positioned.
//! - **Invocation** - [`ActionKind`], "what do I send?". A concrete
//!   service call, optionally carrying a value.
//!
//! ## Why REST and not the existing WebSocket?
//!
//! Recorded in `docs/adr/0003-service-calls-stay-on-rest.md`. In short:
//! sends are throttled, so the peak is ~5 calls/sec and only during a
//! drag, and the per-call cost that motivated the idea was a fresh
//! `reqwest::Client` per call rather than HTTP itself. The migration
//! would put outbound sender plumbing, request-id correlation and
//! reconnect re-attach into the live state feed, which is the one
//! subsystem that must not break.

use serde_json::Value;

use crate::ha::types::{EntityState, HaError};
use crate::ui::format::domain;

/// Domains that may be pinned as a widget.
///
/// Candidacy is deliberately domain-based, not capability-based: an
/// entity is a valid widget if its state is worth *displaying*, whether
/// or not it can also be acted on. A non-dimmable light and a
/// read-only sensor are both perfectly good widgets.
const WIDGET_DOMAINS: &[&str] = &[
    "sensor",
    "binary_sensor",
    "switch",
    "light",
    "scene",
    "script",
    "input_boolean",
    "climate",
    "cover",
];

/// One process-wide HTTP client, so service calls reuse a pooled
/// keep-alive connection.
///
/// Previously every call constructed `reqwest::Client::new()`, which
/// builds a fresh connection pool and therefore paid for a new TCP and
/// TLS handshake per tap. That per-call cost - not HTTP itself - was
/// what made a WebSocket migration look necessary for phase 2. `Client`
/// is internally `Arc`'d and explicitly designed to be reused.
static HTTP: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
}

/// A concrete Home Assistant service call.
///
/// Phase 1 variants are zero-argument: HA decides the resulting state.
/// Phase 2 variants carry the value being set. The enum stays `Copy`:
/// a payload is a scalar, or - where the service takes a colour - a
/// fixed-size group of them, and nothing here owns an allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionKind {
    /// `switch.toggle` - HA flips it based on current state.
    ToggleSwitch,
    /// `light.toggle` - on/off only, no brightness or color.
    ToggleLight,
    /// `scene.turn_on` - a scene is stateless: "activate this scene".
    TriggerScene,
    /// `script.turn_on` - starts the script from the top.
    TriggerScript,
    /// `input_boolean.toggle` - HA's user-defined boolean helper.
    ToggleInputBoolean,
    /// `light.turn_on` with `brightness` (0-255).
    SetBrightness(u8),
    /// `light.turn_on` with `color_temp_kelvin`.
    SetColorTemp(u32),
    /// `light.turn_on` with `hs_color`.
    ///
    /// Hue and saturation travel together because the service parameter
    /// is one two-element array: there is no way to name one of them on
    /// the wire without also naming the other.
    ///
    /// `hs_color` is a universal input rather than a mode-specific one.
    /// Home Assistant's `process_turn_on_params` converts it into
    /// whatever space the light actually supports - RGB, RGBW, RGBWW, XY,
    /// or as a last resort back to a colour temperature - so Snapdash
    /// sends hue and saturation whatever the bulb is, and never has to
    /// branch on the device's native colour mode.
    SetHs { hue: u16, saturation: u8 },
    /// `climate.set_temperature` with `temperature`.
    SetTemperature(f32),
    /// `cover.set_cover_position` with `position` (0-100).
    SetPosition(u8),
}

/// The complete wire mapping for one action: the endpoint plus any
/// parameters beyond `entity_id`.
///
/// Kept as a single value so the whole mapping lives in one auditable
/// place - `wire_mapping_is_stable` asserts against it, and any change
/// here changes what Snapdash puts on the wire.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceCall {
    pub domain: &'static str,
    pub service: &'static str,
    /// Extra body fields, merged alongside `entity_id`.
    pub params: Vec<(&'static str, Value)>,
}

impl ActionKind {
    /// The `(domain, service)` endpoint and parameters to POST.
    pub fn service_call(self) -> ServiceCall {
        let (domain, service, params) = match self {
            Self::ToggleSwitch => ("switch", "toggle", vec![]),
            Self::ToggleLight => ("light", "toggle", vec![]),
            Self::TriggerScene => ("scene", "turn_on", vec![]),
            Self::TriggerScript => ("script", "turn_on", vec![]),
            Self::ToggleInputBoolean => ("input_boolean", "toggle", vec![]),
            Self::SetBrightness(v) => ("light", "turn_on", vec![("brightness", Value::from(v))]),
            Self::SetColorTemp(v) => (
                "light",
                "turn_on",
                vec![("color_temp_kelvin", Value::from(v))],
            ),
            Self::SetHs { hue, saturation } => (
                "light",
                "turn_on",
                vec![(
                    "hs_color",
                    Value::Array(vec![Value::from(hue), Value::from(saturation)]),
                )],
            ),
            Self::SetTemperature(v) => (
                "climate",
                "set_temperature",
                vec![("temperature", Value::from(v))],
            ),
            Self::SetPosition(v) => (
                "cover",
                "set_cover_position",
                vec![("position", Value::from(v))],
            ),
        };

        ServiceCall {
            domain,
            service,
            params,
        }
    }

    /// The zero-argument action a widget fires from its header icon, or
    /// `None` for entities that only display.
    ///
    /// Derivable from the entity id alone, unlike an [`Axis`], because
    /// every primary action is a plain domain-level toggle or trigger.
    pub fn primary_for_entity(entity_id: &str) -> Option<Self> {
        match domain(entity_id) {
            "switch" => Some(Self::ToggleSwitch),
            "light" => Some(Self::ToggleLight),
            "scene" => Some(Self::TriggerScene),
            "script" => Some(Self::TriggerScript),
            "input_boolean" => Some(Self::ToggleInputBoolean),
            _ => None,
        }
    }
}

/// Which dimension an [`Axis`] adjusts. Determines the action built
/// on release and how the value is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisKind {
    Brightness,
    /// White colour temperature of a light, in kelvin.
    ColorTemp,
    /// Position on the colour wheel, in degrees.
    Hue,
    /// How much colour, as a percentage, with 0 being white.
    Saturation,
    Temperature,
    Position,
}

impl AxisKind {
    /// How far an echo may sit from the value we sent and still count as
    /// confirmation of it, for the axes that are confirmed on their own.
    ///
    /// Not one constant, because the round trip through Home Assistant
    /// is lossy by different amounts per axis. Brightness, position and
    /// a setpoint are stored in the units we send, so anything past
    /// float representation noise is a genuinely different value.
    ///
    /// Colour temperature is not: most integrations store mireds and
    /// round-trip kelvin through `round(1_000_000 / kelvin)` and back,
    /// which does not return what we sent. Over the 50 K grid Snapdash
    /// actually sends, the worst case is 19 K (6350 K comes back as
    /// 6369 K). 25 K clears that comfortably while staying at half the
    /// step, so it can never confirm a neighbouring stop.
    ///
    /// Hue and saturation have no answer here, and cannot have one. How
    /// far a colour may have drifted is not a question either axis can
    /// be asked alone: a light storing 8-bit RGB returns a hue roughly
    /// `20 / saturation` degrees from the one it was given, which is
    /// 0.24 degrees at full saturation and 19.9 at saturation 1, where
    /// three bytes barely determine a hue at all. No fixed number on the
    /// hue axis absorbs that without also confirming colours the user
    /// did not pick. They are confirmed jointly instead, by
    /// [`Control::Color`], and in RGB - see
    /// `docs/adr/0004-controls-and-axes.md`.
    fn echo_tolerance(self) -> Option<f32> {
        match self {
            // A setpoint round-trips through HA as a float and may come
            // back with a different representation than we sent, which
            // is all this needs to absorb.
            Self::Brightness | Self::Temperature | Self::Position => Some(0.01),
            Self::ColorTemp => Some(25.0),
            Self::Hue | Self::Saturation => None,
        }
    }
}

/// A numeric dimension of an entity that is *set* rather than toggled,
/// together with the range needed to render a control for it.
///
/// Ranges come from the entity's attributes rather than being hardcoded,
/// because HA reports per-device limits (a thermostat's `min_temp` and
/// `max_temp` vary by device and by unit system).
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub kind: AxisKind,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// Current value as reported by HA, when it reports one.
    pub current: Option<f32>,
}

impl Axis {
    /// `value` brought inside the range Home Assistant reported for this
    /// dimension.
    fn clamped(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

/// One thing the user grabs in an expanded widget.
///
/// An [`Axis`] is a numeric dimension of the entity; a `Control` is the
/// affordance that drives one or more of them, and it owns three things:
/// how it draws, what it puts on the wire ([`Control::action`]), and how
/// it recognises its own echo ([`Control::reconciles`]).
///
/// They were 1:1 until the colour surface arrived, which is why they were
/// the same type, but they are not the same concept: a colour surface is
/// one control setting two axes with one gesture and one service call.
///
/// The enum exists so that grouping is a fact of the type rather than a
/// convention. A flat list with a "these belong together" marker would
/// permit a hue with no saturation beside it, and nothing would catch
/// it. Recorded in `docs/adr/0004-controls-and-axes.md`.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// A single axis, dragged on a slider of its own.
    Value(Axis),
    /// Hue and saturation, dragged as one point on a two-dimensional
    /// field.
    Color { hue: Axis, saturation: Axis },
}

/// How far two colours may differ, per eight-bit channel, and still
/// count as the same colour.
///
/// One level, which is the least a comparison can allow and still be a
/// tolerance at all. It has to be at least that, because Home Assistant
/// changed `color_hsv_to_RGB` from truncating to rounding between the
/// 2024.7 and 2025.1 releases: the two differ by exactly one level, and
/// Snapdash cannot know which of them the house is running. One level is
/// also enough. Two whole degrees of hue are four levels apart at full
/// saturation and two at half, so no neighbouring colour the user could
/// have meant to pick instead can slip through - and below about
/// saturation 20 neighbouring degrees are the *same* three bytes, which
/// is the point: they are the same colour.
const ECHO_CHANNEL_TOLERANCE: u8 = 1;

/// A colour as the three eight-bit channels a light actually stores.
type Rgb8 = [u8; 3];

/// Hue and saturation as the eight-bit colour Home Assistant makes of
/// them.
///
/// A transcription of `color_hs_to_RGB` from `homeassistant/util/color.py`,
/// which is `color_hsv_to_RGB(hue, saturation, 100)`, which is
/// `colorsys.hsv_to_rgb(hue / 360, saturation / 100, 1)` scaled to eight
/// bits. It is deliberately Home Assistant's arithmetic and not a tidier
/// equivalent, because the whole purpose is to land on the same three
/// bytes the bulb was given.
///
/// In `f64` rather than `f32` for the same reason: Python computes this
/// in doubles, and a component that lands within float noise of a
/// half-level would otherwise round the other way and shift a channel.
///
/// Note that this is *not* [`crate::ui::colour_texture`]'s pixel
/// function, even though both turn an HSV triple into bytes. That one
/// paints what the user sees and rounds so the field does not darken
/// towards its saturated edge; this one has to agree with a third party
/// to the level. Making them one function would mean one of the two jobs
/// silently getting the other's answer.
fn hs_to_rgb8(hue: f32, saturation: f32) -> Rgb8 {
    let hue = f64::from(hue) / 360.0;
    let saturation = f64::from(saturation) / 100.0;

    // `colorsys.hsv_to_rgb` takes this branch verbatim, and it matters
    // for more than speed: with S = 0 the sector arithmetic below is
    // undefined-ish at the seam, and white is white at every hue.
    let (red, green, blue) = if saturation == 0.0 {
        (1.0, 1.0, 1.0)
    } else {
        let sector = hue * 6.0;
        // Python's `int()`, which truncates towards zero rather than
        // rounding. `rem_euclid` then folds hue 360 back onto the red
        // the wheel starts at, exactly as `i % 6` does there.
        let index = sector as i64;
        let offset = sector - index as f64;

        // The value axis is pinned to 1, so `v` drops out of all three
        // of Python's `p`, `q` and `t`.
        let p = 1.0 - saturation;
        let q = 1.0 - saturation * offset;
        let t = 1.0 - saturation * (1.0 - offset);

        match index.rem_euclid(6) {
            0 => (1.0, t, p),
            1 => (q, 1.0, p),
            2 => (p, 1.0, t),
            3 => (p, q, 1.0),
            4 => (t, p, 1.0),
            _ => (1.0, p, q),
        }
    };

    [level(red), level(green), level(blue)]
}

/// A 0..=1 colour component as the eight-bit level Python's `round`
/// makes of it.
///
/// Ties go to even, which is what `round` does in Python 3 and what
/// `f64::round` does not: `round(178.5)` is 178 there and 179 here.
/// Half a level is not visible, but agreeing to the level is the entire
/// job of this conversion, so it uses the same rule.
fn level(component: f64) -> u8 {
    (component * 255.0).round_ties_even() as u8
}

fn same_colour(sent: Rgb8, echoed: Rgb8) -> bool {
    sent.iter()
        .zip(echoed)
        .all(|(sent, echoed)| sent.abs_diff(echoed) <= ECHO_CHANNEL_TOLERANCE)
}

impl Control {
    /// Every axis this control drives, in the order it presents them.
    pub fn axes(&self) -> impl Iterator<Item = &Axis> {
        let (first, second) = match self {
            Self::Value(axis) => (axis, None),
            Self::Color { hue, saturation } => (hue, Some(saturation)),
        };

        std::iter::once(first).chain(second)
    }

    /// The one service call that sets this control to the values a
    /// gesture produced, or `None` when the gesture did not name
    /// everything the call needs.
    ///
    /// One call per control and never one per axis. `hs_color` is a
    /// single two-element parameter, so there is no way to name hue on
    /// the wire without also naming saturation, and a colour sent as two
    /// calls would be two colours - the first of them one the user never
    /// pointed at.
    ///
    /// `value` answers the value of an axis this gesture moved, or `None`
    /// for one it did not, which is how a brightness drag on a colour
    /// bulb leaves the colour surface silent.
    pub fn action(&self, value: impl Fn(AxisKind) -> Option<f32>) -> Option<ActionKind> {
        match self {
            Self::Value(axis) => {
                let clamped = axis.clamped(value(axis.kind)?);

                Some(match axis.kind {
                    AxisKind::Brightness => ActionKind::SetBrightness(clamped as u8),
                    AxisKind::ColorTemp => ActionKind::SetColorTemp(clamped as u32),
                    AxisKind::Temperature => ActionKind::SetTemperature(clamped),
                    AxisKind::Position => ActionKind::SetPosition(clamped as u8),
                    // Unreachable by construction: discovery only ever
                    // puts these two inside `Control::Color`, which is
                    // what makes an orphan hue unrepresentable. Silence
                    // rather than a guess, because the guess would be a
                    // saturation the user never chose.
                    AxisKind::Hue | AxisKind::Saturation => return None,
                })
            }

            Self::Color { hue, saturation } => Some(ActionKind::SetHs {
                hue: hue.clamped(value(AxisKind::Hue)?) as u16,
                saturation: saturation.clamped(value(AxisKind::Saturation)?) as u8,
            }),
        }
    }

    /// Whether this echo confirms what was put on the wire.
    ///
    /// `self` is the control as Home Assistant has just reported it, so
    /// each [`Axis::current`] is the echoed value. `sent` answers the
    /// last value sent for an axis, or `None` when that axis has nothing
    /// outstanding.
    ///
    /// The judgement is the control's whole gesture at once and never one
    /// axis at a time, which is what a colour needs: half a confirmed
    /// colour is not a thing, and the caller releases every axis this
    /// control drives or none of them.
    ///
    /// A control with nothing outstanding is vacuously confirmed - there
    /// is nothing for the echo to disagree with, and nothing to release.
    pub fn reconciles(&self, sent: impl Fn(AxisKind) -> Option<f32>) -> bool {
        match self {
            Self::Value(axis) => {
                let Some(sent) = sent(axis.kind) else {
                    return true;
                };

                // An echo that carries no value for the axis is not
                // confirmation of one, and an axis with no tolerance is
                // not confirmable alone at all.
                match (axis.current, axis.kind.echo_tolerance()) {
                    (Some(echoed), Some(tolerance)) => (sent - echoed).abs() <= tolerance,
                    _ => false,
                }
            }

            Self::Color { hue, saturation } => {
                let (hue_sent, saturation_sent) = (sent(AxisKind::Hue), sent(AxisKind::Saturation));

                if hue_sent.is_none() && saturation_sent.is_none() {
                    return true;
                }

                // Half a colour is not a thing in either direction. One
                // component outstanding without the other cannot be
                // judged, because eight-bit RGB needs both; and an echo
                // carrying no colour at all - the light went off, or back
                // into a white mode - confirms nothing.
                let (Some(hue_sent), Some(saturation_sent)) = (hue_sent, saturation_sent) else {
                    return false;
                };
                let (Some(hue_echoed), Some(saturation_echoed)) = (hue.current, saturation.current)
                else {
                    return false;
                };

                // Compared as colours rather than as the coordinates they
                // happen to be written in. A light working in RGB or XY
                // returns a hue and a saturation that are not the ones it
                // was given, by an amount that grows without bound as
                // saturation falls, but the three bytes it is showing are
                // the three bytes it was told to show. Comparing those
                // dissolves the low-saturation ambiguity, and takes hue
                // wraparound with it: hue 0 and hue 360 are one colour.
                same_colour(
                    hs_to_rgb8(hue_sent, saturation_sent),
                    hs_to_rgb8(hue_echoed, saturation_echoed),
                )
            }
        }
    }
}

/// What an entity supports, discovered from its current state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Capabilities {
    /// Fired by the widget's header icon.
    pub primary: Option<ActionKind>,
    /// Every control the entity offers, in the order the expanded widget
    /// shows them. Empty for an entity with nothing to set.
    pub controls: Vec<Control>,
}

impl Capabilities {
    pub fn from_state(state: &EntityState) -> Self {
        Self {
            primary: ActionKind::primary_for_entity(&state.entity_id),
            controls: controls(state),
        }
    }

    /// Every axis the entity exposes, across all of its controls.
    pub fn axes(&self) -> impl Iterator<Item = &Axis> {
        self.controls.iter().flat_map(Control::axes)
    }

    /// The axis of a given kind, when the entity has one.
    pub fn axis(&self, kind: AxisKind) -> Option<&Axis> {
        self.axes().find(|a| a.kind == kind)
    }

    /// `true` when the entity can only be displayed, never acted on.
    pub fn is_display_only(&self) -> bool {
        self.primary.is_none() && self.controls.is_empty()
    }
}

fn attr_f32(state: &EntityState, key: &str) -> Option<f32> {
    match state.attributes.get(key)? {
        Value::Number(n) => n.as_f64().map(|v| v as f32),
        Value::String(s) => s.parse::<f32>().ok(),
        _ => None,
    }
}

/// One element of the `hs_color` Home Assistant is currently reporting:
/// index 0 is the hue in degrees, index 1 the saturation in percent.
///
/// Its own reader because `hs_color` is the one attribute Snapdash takes
/// a value from that is not a scalar, and [`attr_f32`] would answer
/// `None` for it.
///
/// The attribute says what colour the light is showing and nothing about
/// which mode it is in: Home Assistant *derives* an `hs_color` from the
/// kelvin value whenever the active mode is `color_temp`, so it is
/// present for a bulb sitting in plain white. Discovery is keyed on
/// `supported_color_modes` for that reason, never on this.
fn attr_hs(state: &EntityState, index: usize) -> Option<f32> {
    match state.attributes.get("hs_color")? {
        Value::Array(hs) => hs.get(index)?.as_f64().map(|value| value as f32),
        _ => None,
    }
}

fn supported_features(state: &EntityState) -> u64 {
    state
        .attributes
        .get("supported_features")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// The colour modes a light advertises as supported.
///
/// Every light capability Snapdash discovers is read from this one list,
/// and from the *supported* list rather than the active `color_mode`.
/// `color_mode` is `None` whenever the light is off, so keying on it
/// would make controls appear and disappear as the light is switched.
fn color_modes(state: &EntityState) -> impl Iterator<Item = &str> {
    match state.attributes.get("supported_color_modes") {
        Some(Value::Array(modes)) => modes.as_slice(),
        _ => &[],
    }
    .iter()
    .filter_map(Value::as_str)
}

/// The colour modes that carry a full colour, `COLOR_MODES_COLOR` in
/// Home Assistant's `light/const.py`.
///
/// A light in any one of these can be given a hue. `color_temp` and
/// `white` are deliberately absent: they are modes for producing white
/// light and a hue means nothing in them.
const COLOR_MODES_COLOR: &[&str] = &["hs", "rgb", "rgbw", "rgbww", "xy"];

/// A light is dimmable when it advertises any color mode other than
/// plain on/off. `onoff` and `unknown` are the two modes that carry no
/// brightness channel; everything else (`brightness`, `color_temp`,
/// `hs`, `xy`, `rgb`, `rgbw`, `rgbww`, `white`) does.
fn light_is_dimmable(state: &EntityState) -> bool {
    color_modes(state).any(|m| !matches!(m, "onoff" | "unknown"))
}

/// A light can be given a colour when it advertises any of the colour
/// modes, not `hs` alone.
///
/// The distinction matters because `hs_color` is a universal input:
/// Home Assistant converts it into whichever space the device natively
/// speaks. An `rgb`-only bulb takes hue and saturation perfectly well
/// and would be left with no colour control by a narrower predicate.
fn light_has_colour(state: &EntityState) -> bool {
    color_modes(state).any(|m| COLOR_MODES_COLOR.contains(&m))
}

/// A light supports white colour temperature when it advertises the
/// `color_temp` colour mode.
///
/// `color_temp_kelvin` is `null` whenever the light is currently in some
/// other mode, so the axis is offered on the strength of what the device
/// *supports* rather than of what it happens to be doing. Until the
/// light is put into that mode the axis has no value, and the control
/// renders as absent (`docs/adr/0006-a-null-axis-renders-as-absent.md`).
fn light_has_color_temp(state: &EntityState) -> bool {
    color_modes(state).any(|m| m == "color_temp")
}

/// Every control an entity offers, in the order they should be shown.
///
/// A light can carry more than one, which is why this returns a list.
/// Each axis reconciles independently, but they share one send throttle
/// per entity, so the peak call rate does not grow with the axis count
/// (`docs/adr/0003-service-calls-stay-on-rest.md`).
fn controls(state: &EntityState) -> Vec<Control> {
    let mut controls = Vec::new();

    match domain(&state.entity_id) {
        "light" => {
            if light_is_dimmable(state) {
                controls.push(Control::Value(Axis {
                    kind: AxisKind::Brightness,
                    min: 0.0,
                    max: 255.0,
                    step: 1.0,
                    current: attr_f32(state, "brightness"),
                }));
            }

            if light_has_color_temp(state) {
                // Ranges are per-device. The fallbacks are HA's own
                // defaults for a light that reports the mode but not its
                // limits, which some integrations do.
                controls.push(Control::Value(Axis {
                    kind: AxisKind::ColorTemp,
                    min: attr_f32(state, "min_color_temp_kelvin").unwrap_or(2000.0),
                    max: attr_f32(state, "max_color_temp_kelvin").unwrap_or(6535.0),
                    // Kelvin spans thousands, so a 1 K step would be a
                    // slider with several thousand stops and no visible
                    // difference between neighbours.
                    step: 50.0,
                    current: attr_f32(state, "color_temp_kelvin"),
                }));
            }

            if light_has_colour(state) {
                controls.push(Control::Color {
                    hue: Axis {
                        kind: AxisKind::Hue,
                        min: 0.0,
                        // Capped at 359 rather than 360. The two ends of
                        // the wheel are the same red, so a 360th stop
                        // would be a second name for the value at 0 and
                        // every piece of state that compares hues would
                        // have to know it. Red still appears at both ends
                        // of the field, as it does in every colour picker.
                        max: 359.0,
                        step: 1.0,
                        current: attr_hs(state, 0),
                    },
                    saturation: Axis {
                        kind: AxisKind::Saturation,
                        min: 0.0,
                        // The range `hs_color` itself uses, so nothing is
                        // rescaled on the way out or on the way back.
                        max: 100.0,
                        step: 1.0,
                        current: attr_hs(state, 1),
                    },
                });
            }
        }

        // ClimateEntityFeature.TARGET_TEMPERATURE == 1. Without it the
        // entity has no single setpoint to drag (it may be a range-only
        // thermostat, which is not covered).
        "climate" if supported_features(state) & 1 != 0 => {
            controls.push(Control::Value(Axis {
                kind: AxisKind::Temperature,
                min: attr_f32(state, "min_temp").unwrap_or(7.0),
                max: attr_f32(state, "max_temp").unwrap_or(35.0),
                step: attr_f32(state, "target_temp_step").unwrap_or(0.5),
                current: attr_f32(state, "temperature"),
            }));
        }

        // CoverEntityFeature.SET_POSITION == 4. Open/close-only covers
        // report 1|2 but not 4 and get no slider.
        "cover" if supported_features(state) & 4 != 0 => {
            controls.push(Control::Value(Axis {
                kind: AxisKind::Position,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                current: attr_f32(state, "current_position"),
            }));
        }

        _ => {}
    }

    controls
}

/// Whether an entity is eligible to appear in the Settings widget
/// picker. See [`WIDGET_DOMAINS`] for why this is domain-based.
pub fn is_widget_candidate(entity_id: &str) -> bool {
    WIDGET_DOMAINS.contains(&domain(entity_id))
}

/// POST `/api/services/{domain}/{service}` with `{"entity_id": …}` plus
/// any action parameters.
///
/// Returns `Ok(())` when HA acknowledges with a 2xx. Any transport or
/// server-side failure lands in `HaError::ServiceCall` with enough
/// context for the status bar. The widget UI is refreshed via the
/// normal `state_changed` WS event that follows a successful call,
/// so this function is fire-and-forget from the caller's perspective.
pub async fn call_service(
    ha_url: &str,
    token: &str,
    action: ActionKind,
    entity_id: &str,
) -> Result<(), HaError> {
    let call = action.service_call();
    let base = ha_url.trim_end_matches('/');
    let url = format!("{base}/api/services/{}/{}", call.domain, call.service);

    let mut body = serde_json::Map::new();
    body.insert("entity_id".into(), Value::from(entity_id));
    for (key, value) in call.params {
        body.insert(key.into(), value);
    }

    let resp = http()
        .post(&url)
        .bearer_auth(token)
        .json(&Value::Object(body))
        .send()
        .await
        .map_err(|e| HaError::ServiceCall {
            entity_id: entity_id.to_owned(),
            status: None,
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(HaError::ServiceCall {
            entity_id: entity_id.to_owned(),
            status: Some(resp.status().as_u16()),
            message: resp
                .status()
                .canonical_reason()
                .unwrap_or("error")
                .to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(entity_id: &str, attrs: Value) -> EntityState {
        EntityState {
            entity_id: entity_id.to_owned(),
            state: "on".into(),
            attributes: serde_json::from_value(attrs).unwrap(),
            last_changed: None,
            last_updated: None,
        }
    }

    #[test]
    fn maps_supported_domains_to_primary_actions() {
        assert_eq!(
            ActionKind::primary_for_entity("switch.kitchen"),
            Some(ActionKind::ToggleSwitch)
        );
        assert_eq!(
            ActionKind::primary_for_entity("light.living_room"),
            Some(ActionKind::ToggleLight)
        );
        assert_eq!(
            ActionKind::primary_for_entity("scene.evening"),
            Some(ActionKind::TriggerScene)
        );
        assert_eq!(
            ActionKind::primary_for_entity("script.morning_routine"),
            Some(ActionKind::TriggerScript)
        );
        assert_eq!(
            ActionKind::primary_for_entity("input_boolean.guest_mode"),
            Some(ActionKind::ToggleInputBoolean)
        );
    }

    #[test]
    fn rejects_unsupported_domains() {
        assert_eq!(ActionKind::primary_for_entity("sensor.outdoor_temp"), None);
        assert_eq!(ActionKind::primary_for_entity("climate.thermostat"), None);
        assert_eq!(
            ActionKind::primary_for_entity("media_player.living_tv"),
            None
        );
        assert_eq!(ActionKind::primary_for_entity("cover.blinds"), None);
        assert_eq!(ActionKind::primary_for_entity("lock.front_door"), None);
    }

    #[test]
    fn rejects_malformed_entity_ids() {
        // Missing domain (no dot).
        assert_eq!(ActionKind::primary_for_entity("just_a_name"), None);
        assert_eq!(ActionKind::primary_for_entity(""), None);
        // Wrong-case domain - HA IDs are always lowercase, we don't try to normalize.
        assert_eq!(ActionKind::primary_for_entity("Switch.kitchen"), None);
    }

    #[test]
    fn widget_candidates_span_display_and_actions() {
        // Read-only display candidates.
        assert!(is_widget_candidate("sensor.outdoor_temp"));
        assert!(is_widget_candidate("binary_sensor.front_door"));
        // Actionable ones.
        assert!(is_widget_candidate("switch.kitchen"));
        assert!(is_widget_candidate("light.living_room"));
        assert!(is_widget_candidate("scene.evening"));
        assert!(is_widget_candidate("script.morning"));
        assert!(is_widget_candidate("input_boolean.guest_mode"));
        // Domains that carry a control (#87). Candidates regardless of
        // whether the individual device supports a setpoint or position -
        // their state is worth displaying either way.
        assert!(is_widget_candidate("climate.thermostat"));
        assert!(is_widget_candidate("cover.blinds"));
        // Still out of scope - no phase 2 control surface for media.
        assert!(!is_widget_candidate("media_player.living_tv"));
        // Malformed.
        assert!(!is_widget_candidate("no_dot_here"));
        assert!(!is_widget_candidate(""));
    }

    #[test]
    fn wire_mapping_is_stable() {
        // Any change here means the on-wire call to HA changes shape -
        // bump deliberately.
        let call = |a: ActionKind| {
            let c = a.service_call();
            (c.domain, c.service, c.params)
        };

        assert_eq!(call(ActionKind::ToggleSwitch), ("switch", "toggle", vec![]));
        assert_eq!(call(ActionKind::ToggleLight), ("light", "toggle", vec![]));
        assert_eq!(call(ActionKind::TriggerScene), ("scene", "turn_on", vec![]));
        assert_eq!(
            call(ActionKind::TriggerScript),
            ("script", "turn_on", vec![])
        );
        assert_eq!(
            call(ActionKind::ToggleInputBoolean),
            ("input_boolean", "toggle", vec![])
        );
        assert_eq!(
            call(ActionKind::SetBrightness(128)),
            ("light", "turn_on", vec![("brightness", json!(128))])
        );
        assert_eq!(
            call(ActionKind::SetTemperature(21.5)),
            (
                "climate",
                "set_temperature",
                vec![("temperature", json!(21.5))]
            )
        );
        assert_eq!(
            call(ActionKind::SetPosition(60)),
            ("cover", "set_cover_position", vec![("position", json!(60))])
        );
        // Kelvin, not mireds. HA accepts both but `color_temp` (mireds)
        // is deprecated, and the two are inverse scales, so sending the
        // wrong key would drive the slider backwards.
        assert_eq!(
            call(ActionKind::SetColorTemp(3000)),
            ("light", "turn_on", vec![("color_temp_kelvin", json!(3000))])
        );
        // One parameter, carrying both numbers. Brightness is deliberately
        // not sent with it: `light.turn_on` without `brightness` leaves
        // the light's brightness where it was, so one control sets one
        // thing and the colour cannot dim the lamp behind the user's back.
        assert_eq!(
            call(ActionKind::SetHs {
                hue: 200,
                saturation: 100
            }),
            ("light", "turn_on", vec![("hs_color", json!([200, 100]))])
        );
    }

    #[test]
    fn dimmable_light_reports_brightness_control() {
        let s = state(
            "light.living_room",
            json!({ "supported_color_modes": ["brightness"], "brightness": 128 }),
        );
        let caps = Capabilities::from_state(&s);

        assert_eq!(caps.primary, Some(ActionKind::ToggleLight));
        let c = caps
            .axis(AxisKind::Brightness)
            .expect("dimmable light has a brightness axis");
        assert_eq!((c.min, c.max), (0.0, 255.0));
        assert_eq!(c.current, Some(128.0));
        // Brightness alone: this bulb advertises no colour temperature.
        assert_eq!(caps.controls.len(), 1);
    }

    /// A light can expose more than one axis, which is the whole reason
    /// capabilities are a list rather than a single control.
    #[test]
    fn a_light_can_report_brightness_and_colour_temperature() {
        let s = state(
            "light.kitchen",
            json!({
                "supported_color_modes": ["color_temp"],
                "brightness": 200,
                "color_temp_kelvin": 3000,
                "min_color_temp_kelvin": 2202,
                "max_color_temp_kelvin": 6535
            }),
        );
        let caps = Capabilities::from_state(&s);

        assert_eq!(caps.controls.len(), 2);
        // Brightness first: it is the axis people reach for.
        assert_eq!(
            caps.axes().map(|a| a.kind).collect::<Vec<_>>(),
            vec![AxisKind::Brightness, AxisKind::ColorTemp]
        );

        let temp = caps
            .axis(AxisKind::ColorTemp)
            .expect("color_temp mode means a white axis");
        assert_eq!((temp.min, temp.max), (2202.0, 6535.0));
        assert_eq!(temp.current, Some(3000.0));
    }

    /// `color_temp_kelvin` is null whenever the light sits in another
    /// colour mode. The axis is still offered, because the device
    /// supports it, and it carries no value until the light is in that
    /// mode.
    #[test]
    fn colour_temperature_survives_the_light_being_in_another_mode() {
        let s = state(
            "light.kitchen",
            json!({
                "supported_color_modes": ["color_temp", "hs"],
                "color_mode": "hs",
                "color_temp_kelvin": null
            }),
        );
        let temp = Capabilities::from_state(&s)
            .axis(AxisKind::ColorTemp)
            .cloned()
            .expect("still supported, just not active");

        assert_eq!(temp.current, None);
    }

    /// The predicate is `COLOR_MODES_COLOR` and not `hs` alone. Hue and
    /// saturation are a universal input that Home Assistant converts into
    /// whatever the device natively speaks, so every one of these bulbs
    /// takes a colour, and an `hs`-only check would leave the most common
    /// of them - a plain RGB strip - with no colour control at all.
    #[test]
    fn every_colour_mode_offers_a_colour_surface() {
        for mode in ["hs", "rgb", "rgbw", "rgbww", "xy"] {
            let s = state(
                "light.strip",
                json!({ "supported_color_modes": [mode], "hs_color": [120.0, 90.0] }),
            );
            let caps = Capabilities::from_state(&s);

            let hue = caps
                .axis(AxisKind::Hue)
                .unwrap_or_else(|| panic!("{mode} is a colour mode"));
            assert_eq!(hue.current, Some(120.0), "{mode}");
            assert_eq!(
                caps.axis(AxisKind::Saturation).and_then(|a| a.current),
                Some(90.0),
                "{mode}"
            );
        }
    }

    /// The negatives, and the reason the check cannot simply be "not
    /// on/off": `brightness`, `color_temp` and `white` are all modes a
    /// dimmable light reports, and none of them can show a colour.
    #[test]
    fn a_light_with_no_colour_mode_offers_no_colour_surface() {
        for mode in ["onoff", "brightness", "color_temp", "white"] {
            let s = state(
                "light.lamp",
                json!({
                    "supported_color_modes": [mode],
                    // Given on purpose to every one of these, because
                    // Home Assistant really does report an `hs_color`
                    // for a light in `color_temp` mode: the attribute's
                    // presence is never evidence of a colour capability,
                    // and discovery must not be tempted to read it.
                    "hs_color": [28.391, 65.659]
                }),
            );

            let caps = Capabilities::from_state(&s);

            assert!(
                !caps
                    .controls
                    .iter()
                    .any(|control| matches!(control, Control::Color { .. })),
                "{mode} is not a colour mode"
            );
            assert!(caps.axis(AxisKind::Hue).is_none(), "{mode}");
            assert!(caps.axis(AxisKind::Saturation).is_none(), "{mode}");
        }
    }

    /// The wheel is `[0, 359]` in whole degrees. Capping at 359 rather
    /// than 360 keeps one value per colour: the two ends are the same
    /// red, and a 360th stop would be a second name for 0 that every
    /// comparison of hues would then have to know about.
    #[test]
    fn hue_spans_the_wheel_capped_at_359() {
        let s = state("light.strip", json!({ "supported_color_modes": ["rgb"] }));
        let caps = Capabilities::from_state(&s);
        let hue = caps.axis(AxisKind::Hue).expect("an rgb light has a hue");

        assert_eq!((hue.min, hue.max, hue.step), (0.0, 359.0, 1.0));
        // No `hs_color` at all, which is what an off light reports. The
        // axis is offered because the device supports colour, and carries
        // no value until it is showing one (#94).
        assert_eq!(hue.current, None);
    }

    /// Home Assistant reports the colour as one two-element array, so
    /// both axes of the surface are read out of the same attribute.
    #[test]
    fn the_colour_surface_reads_both_elements_of_hs_color() {
        let s = state(
            "light.strip",
            json!({ "supported_color_modes": ["rgb"], "hs_color": [199.765, 62.5] }),
        );
        let caps = Capabilities::from_state(&s);

        assert_eq!(caps.axis(AxisKind::Hue).unwrap().current, Some(199.765));
        assert_eq!(caps.axis(AxisKind::Saturation).unwrap().current, Some(62.5));
    }

    /// Saturation is a full axis of its own, and one the user drives, so
    /// it carries the range `hs_color` is stated in rather than a
    /// rescaling of it.
    #[test]
    fn saturation_spans_zero_to_a_hundred_in_whole_steps() {
        let s = state("light.strip", json!({ "supported_color_modes": ["rgb"] }));
        let caps = Capabilities::from_state(&s);
        let saturation = caps
            .axis(AxisKind::Saturation)
            .expect("an rgb light has a saturation");

        assert_eq!(
            (saturation.min, saturation.max, saturation.step),
            (0.0, 100.0, 1.0)
        );
        // No `hs_color` at all, which is what an off light reports.
        assert_eq!(saturation.current, None);
    }

    /// The two axes are one control, not two beside each other. Nothing
    /// downstream has to be told they belong together, and nothing can
    /// render the hue on a slider of its own.
    #[test]
    fn hue_and_saturation_arrive_as_one_control() {
        let s = state(
            "light.strip",
            json!({ "supported_color_modes": ["rgb"], "hs_color": [199.765, 62.5] }),
        );
        let caps = Capabilities::from_state(&s);
        let colour = caps
            .controls
            .iter()
            .find(|control| matches!(control, Control::Color { .. }))
            .expect("an rgb light has a colour surface");

        // Brightness beside it, and nothing else: two controls, three
        // axes.
        assert_eq!(caps.controls.len(), 2);
        assert_eq!(
            colour.axes().map(|axis| axis.kind).collect::<Vec<_>>(),
            vec![AxisKind::Hue, AxisKind::Saturation],
            "hue across, saturation down, in the order the field shows them"
        );
    }

    #[test]
    fn onoff_only_light_has_no_axes() {
        let s = state("light.porch", json!({ "supported_color_modes": ["onoff"] }));
        let caps = Capabilities::from_state(&s);

        // Still tappable, just not adjustable. This is exactly the case
        // an entity-id-only check gets wrong.
        assert_eq!(caps.primary, Some(ActionKind::ToggleLight));
        assert!(caps.controls.is_empty());
    }

    #[test]
    fn light_without_color_modes_has_no_axes() {
        let s = state("light.mystery", json!({}));
        assert!(Capabilities::from_state(&s).controls.is_empty());
    }

    #[test]
    fn climate_setpoint_uses_reported_range() {
        let s = state(
            "climate.hall",
            json!({
                "supported_features": 1,
                "min_temp": 10.0,
                "max_temp": 28.0,
                "target_temp_step": 0.5,
                "temperature": 21.0
            }),
        );
        let caps = Capabilities::from_state(&s);
        let c = caps
            .axis(AxisKind::Temperature)
            .expect("target temperature supported");

        assert_eq!((c.min, c.max, c.step), (10.0, 28.0, 0.5));
        assert_eq!(c.current, Some(21.0));
    }

    #[test]
    fn climate_without_target_temperature_feature_has_no_control() {
        let s = state("climate.hall", json!({ "supported_features": 0 }));
        assert!(Capabilities::from_state(&s).controls.is_empty());
    }

    #[test]
    fn cover_needs_set_position_feature() {
        // OPEN|CLOSE|STOP but no SET_POSITION (4).
        let positionless = state("cover.garage", json!({ "supported_features": 11 }));
        assert!(Capabilities::from_state(&positionless).controls.is_empty());

        let positionable = state(
            "cover.blinds",
            json!({ "supported_features": 15, "current_position": 40 }),
        );
        let caps = Capabilities::from_state(&positionable);
        let c = caps
            .axis(AxisKind::Position)
            .expect("SET_POSITION means a position axis");
        assert_eq!(c.current, Some(40.0));
    }

    #[test]
    fn sensors_are_display_only() {
        let s = state(
            "sensor.outdoor_temp",
            json!({ "unit_of_measurement": "°C" }),
        );
        assert!(Capabilities::from_state(&s).is_display_only());
    }

    /// Every value in this module's tests reaches an action through the
    /// same door a gesture does: a lookup by axis.
    fn at(values: &[(AxisKind, f32)]) -> impl Fn(AxisKind) -> Option<f32> + use<'_> {
        move |kind| {
            values
                .iter()
                .find(|(k, _)| *k == kind)
                .map(|(_, value)| *value)
        }
    }

    fn brightness() -> Control {
        Control::Value(Axis {
            kind: AxisKind::Brightness,
            min: 0.0,
            max: 255.0,
            step: 1.0,
            current: None,
        })
    }

    /// A colour that went out and the colour that came back, each as
    /// `(hue, saturation)`.
    type RoundTrip = ((f32, f32), (f32, f32));

    /// A colour surface built by hand, with whatever the echo reports.
    fn colour(current: Option<(f32, f32)>) -> Control {
        Control::Color {
            hue: Axis {
                kind: AxisKind::Hue,
                min: 0.0,
                max: 359.0,
                step: 1.0,
                current: current.map(|(hue, _)| hue),
            },
            saturation: Axis {
                kind: AxisKind::Saturation,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                current: current.map(|(_, saturation)| saturation),
            },
        }
    }

    #[test]
    fn a_control_clamps_before_building_an_action() {
        let control = brightness();

        assert_eq!(
            control.action(at(&[(AxisKind::Brightness, 300.0)])),
            Some(ActionKind::SetBrightness(255))
        );
        assert_eq!(
            control.action(at(&[(AxisKind::Brightness, -20.0)])),
            Some(ActionKind::SetBrightness(0))
        );
        assert_eq!(
            control.action(at(&[(AxisKind::Brightness, 128.0)])),
            Some(ActionKind::SetBrightness(128))
        );
    }

    /// A gesture that did not touch this control has nothing for it to
    /// send. That is what keeps a brightness drag on a colour bulb from
    /// also restating the colour.
    #[test]
    fn a_control_the_gesture_did_not_move_sends_nothing() {
        assert_eq!(brightness().action(at(&[(AxisKind::Hue, 200.0)])), None);
        assert_eq!(
            colour(None).action(at(&[(AxisKind::Brightness, 128.0)])),
            None
        );
    }

    /// Both components in one call, and the clamp is what keeps the
    /// wheel single-valued: a hue past the top lands on 359 rather than
    /// wrapping to 0, so nothing downstream has to reason about which
    /// end of the field a value came from.
    #[test]
    fn a_colour_surface_sends_both_components_and_never_leaves_the_wheel() {
        let field = colour(None);
        let hs = |hue: u16, saturation: u8| Some(ActionKind::SetHs { hue, saturation });

        assert_eq!(
            field.action(at(&[(AxisKind::Hue, 212.0), (AxisKind::Saturation, 85.0)])),
            hs(212, 85)
        );
        assert_eq!(
            field.action(at(&[(AxisKind::Hue, 400.0), (AxisKind::Saturation, 140.0)])),
            hs(359, 100)
        );
        assert_eq!(
            field.action(at(&[(AxisKind::Hue, -1.0), (AxisKind::Saturation, -1.0)])),
            hs(0, 0)
        );
    }

    /// Half a colour cannot go on the wire, because `hs_color` is one
    /// parameter carrying two numbers: there is no way to name the hue
    /// without also naming a saturation, and inventing one would send a
    /// colour the user did not pick.
    #[test]
    fn a_colour_surface_sends_nothing_for_half_a_gesture() {
        let field = colour(Some((40.0, 100.0)));

        assert_eq!(field.action(at(&[(AxisKind::Hue, 212.0)])), None);
        assert_eq!(field.action(at(&[(AxisKind::Saturation, 85.0)])), None);
    }

    /// Every `(sent, echoed)` pair below was produced by running Home
    /// Assistant's own `homeassistant.util.color`, not by any arithmetic
    /// of Snapdash's:
    ///
    /// ```text
    /// rgb = color_hs_to_RGB(hue, saturation)
    /// echoed = color_RGB_to_hs(*rgb)
    /// ```
    ///
    /// That matters more than it looks. An expected value recomputed the
    /// way the code computes it can never disagree with the code, so a
    /// table generated by this module's `hs_to_rgb8` would assert
    /// nothing at all.
    ///
    /// Both eras are here because both are running in people's houses.
    /// Home Assistant changed `color_hsv_to_RGB` from truncating to
    /// rounding between 2025.1 and the 2024.7 before it, which moves
    /// whole rows: hue 59 at saturation 1 comes back as 60 from one and
    /// as 40 from the other. Every row of both must confirm.
    const GOLDEN_ECHOES: &[RoundTrip] = &[
        // Home Assistant 2025.1 and later, which rounds.
        ((0.0, 100.0), (0.0, 100.0)),       // rgb(255, 0, 0)
        ((59.0, 1.0), (60.0, 1.176)),       // rgb(255, 255, 252)
        ((0.0, 80.0), (0.0, 80.0)),         // rgb(255, 51, 51)
        ((132.0, 100.0), (132.0, 100.0)),   // rgb(0, 255, 51)
        ((200.0, 100.0), (200.0, 100.0)),   // rgb(0, 170, 255)
        ((212.0, 85.0), (212.074, 85.098)), // rgb(38, 139, 255)
        ((240.0, 3.0), (240.0, 3.137)),     // rgb(247, 247, 255)
        ((300.0, 50.0), (300.0, 49.804)),   // rgb(255, 128, 255)
        ((359.0, 100.0), (359.059, 100.0)), // rgb(255, 0, 4)
        ((359.0, 1.0), (0.0, 1.176)),       // rgb(255, 252, 252)
        ((45.0, 2.0), (48.0, 1.961)),       // rgb(255, 254, 250)
        ((180.0, 7.0), (180.0, 7.059)),     // rgb(237, 255, 255)
        ((96.0, 33.0), (95.714, 32.941)),   // rgb(205, 255, 171)
        // Home Assistant 2024.7 and earlier, which truncates.
        ((59.0, 1.0), (40.0, 1.176)),       // rgb(255, 254, 252)
        ((0.0, 80.0), (0.0, 80.392)),       // rgb(255, 50, 50)
        ((132.0, 100.0), (131.765, 100.0)), // rgb(0, 255, 50)
        ((200.0, 100.0), (200.235, 100.0)), // rgb(0, 169, 255)
        ((300.0, 50.0), (300.0, 50.196)),   // rgb(255, 127, 255)
        ((45.0, 2.0), (40.0, 2.353)),       // rgb(255, 253, 249)
        ((96.0, 33.0), (96.0, 33.333)),     // rgb(204, 255, 170)
    ];

    /// The tolerance, stated as the round trips it has to survive.
    ///
    /// Read the low-saturation rows first: hue 59 at saturation 1 comes
    /// back as 40 or as 60 depending on the release, nineteen degrees
    /// out and past any tolerance a hue axis could carry without also
    /// confirming a colour the user did not pick. As three bytes they
    /// are the same colour, which is the whole argument for comparing
    /// there.
    #[test]
    fn an_echo_from_an_rgb_light_confirms_at_any_saturation() {
        for &((hue_sent, saturation_sent), (hue_echoed, saturation_echoed)) in GOLDEN_ECHOES {
            let control = colour(Some((hue_echoed, saturation_echoed)));

            assert!(
                control.reconciles(at(&[
                    (AxisKind::Hue, hue_sent),
                    (AxisKind::Saturation, saturation_sent),
                ])),
                "({hue_sent}, {saturation_sent}) came back as \
                 ({hue_echoed}, {saturation_echoed}) and must confirm"
            );
        }
    }

    /// And no wider than that. Each pair here is two colours a person
    /// would call different, so the light answering with the second is
    /// the light not having done what it was told - the case the settle
    /// window exists for, and one the widget must not paper over by
    /// declaring itself confirmed.
    ///
    /// The eight-bit distances are the reason each one is here: ten
    /// degrees of hue at full saturation is 42 levels, one degree is 4,
    /// one point of saturation at full hue is 3, and three points of
    /// saturation near white is 7.
    #[test]
    fn a_genuinely_different_colour_does_not_confirm() {
        const DIFFERENT: &[RoundTrip] = &[
            ((200.0, 100.0), (210.0, 100.0)),
            ((120.0, 100.0), (121.0, 100.0)),
            ((200.0, 100.0), (200.0, 99.0)),
            ((0.0, 50.0), (180.0, 50.0)),
            ((0.0, 100.0), (359.0, 100.0)),
            ((59.0, 1.0), (59.0, 4.0)),
            ((200.0, 100.0), (200.0, 90.0)),
        ];

        for &((hue_sent, saturation_sent), (hue_echoed, saturation_echoed)) in DIFFERENT {
            let control = colour(Some((hue_echoed, saturation_echoed)));

            assert!(
                !control.reconciles(at(&[
                    (AxisKind::Hue, hue_sent),
                    (AxisKind::Saturation, saturation_sent),
                ])),
                "({hue_sent}, {saturation_sent}) must not be confirmed by \
                 ({hue_echoed}, {saturation_echoed})"
            );
        }
    }

    /// A light that went off, or back into a white mode, reports no
    /// colour at all. That is not confirmation of the one it was given,
    /// and treating it as one would hand the surface back to Home
    /// Assistant while the house is showing something else entirely.
    #[test]
    fn an_echo_carrying_no_colour_confirms_nothing() {
        assert!(
            !colour(None).reconciles(at(&[(AxisKind::Hue, 212.0), (AxisKind::Saturation, 85.0),]))
        );
    }

    /// Nothing outstanding, nothing to confirm. The caller uses this to
    /// decide what to release, so a control with no pending axis has to
    /// answer yes and release nothing rather than answer no and hold an
    /// entity that is not waiting on anything.
    #[test]
    fn a_control_with_nothing_in_flight_is_vacuously_confirmed() {
        assert!(colour(Some((40.0, 100.0))).reconciles(at(&[])));
        assert!(brightness().reconciles(at(&[])));
    }

    /// Half a colour cannot be judged either. Eight-bit RGB needs both
    /// components, so a control holding one of them and not the other
    /// keeps waiting rather than guessing.
    #[test]
    fn half_a_colour_in_flight_is_not_confirmed() {
        let control = colour(Some((212.074, 85.098)));

        assert!(!control.reconciles(at(&[(AxisKind::Hue, 212.0)])));
        assert!(!control.reconciles(at(&[(AxisKind::Saturation, 85.0)])));
    }
}
