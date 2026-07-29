use crate::ha::types::HaError;
use crate::ha::{EntityState, HaConnectionConfig, HaEvent};
use crate::logger::LogType;
use crate::system_info::{SysinfoData, SystemInfo};
use crate::theme::loader::{available_themes, resolve_theme};
use crate::ui::platform::window_settings;
use crate::ui::settings::*;
use crate::update;
use crate::widget_size::{Priority, WidgetSize};
use crate::{ha, logger};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use iced::system::Information;
use iced::window;
use iced::{Element, Task};

use crate::config::{Config, WidgetPosition};
use crate::ha::token::{self, TokenPresence};
use crate::theme::{DEFAULT_THEME, ThemeDef, ThemeKind};

use super::window::{
    EntityWindowState, Expansion, WindowKind, WindowState, find_window_id, lift_needed,
};

#[derive(Debug, Clone)]
pub struct SettingsSensor {
    pub entity_id: String,
    pub friendly_name: String,
    pub search_key: String,
}

#[derive(Debug, Clone, Copy)]
pub enum FocusDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone)]
pub enum GalleryState {
    Idle,
    Loading,
    Loaded(Vec<ThemeDef>),
    Failed(String),
}

#[derive(Debug)]
pub struct Snapdash {
    pub config: Config,
    pub token_presence: TokenPresence,
    pub theme: ThemeDef,
    pub available_themes: Vec<ThemeDef>,
    pub theme_import_status: Option<Result<String, String>>,
    pub gallery: GalleryState,
    pub gallery_status: Option<Result<String, String>>,

    pub ha: ha::HaState,

    pub windows: HashMap<window::Id, WindowState>,

    pub theme_options: Vec<ThemeKind>,
    pub status: String,

    pub entity_windows: HashMap<String, window::Id>,
    pub entity_windows_opening: HashSet<String>,
    pub boot_open_done: bool,

    pub settings_sensors: Vec<SettingsSensor>,
    pub selected_widgets: HashSet<String>,
    pub active_settings_sensors: Vec<SettingsSensor>,

    pub entity_search_query: String,
    pub settings_page: SettingsPage,
    pub settings_search: String,

    pub update: update::UpdateStatus,

    pub last_widget_move_at: Option<std::time::Instant>,

    pub config_save_in_flight: bool,
    pub config_save_pending: bool,

    pub sys_data: Option<SysinfoData>,
    pub sys_info: Option<SystemInfo>,
    pub iced_sys_info: Option<iced::system::Information>,

    /// Locally-held values for entities the user is currently adjusting.
    /// Consulted before any `state_changed` is applied, so a throttled
    /// drag isn't fought by echoes that lag the user's finger.
    pub pending: super::PendingValues,
}

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    OpenSettings,
    OpenSettingsTo(SettingsPage),
    OpenEntity(String),
    OpenReleaseNotes,
    OpenUrl(String),
    OpenWidgetSettings(String),
    CloseWindow(window::Id),
    QuitApp,
    WindowClosed(window::Id),
    //  WindowOpened(window::Id, WindowKind),
    WindowOpened {
        id: window::Id,
        kind: WindowKind,
    },
    AnimationFrame(iced::time::Instant),

    ThemeSelected(String),
    ImportTheme,
    ThemeFilePicked(Option<std::path::PathBuf>),
    OpenThemeGallery,
    GalleryIndexFetched(Result<Vec<ThemeDef>, String>),
    InstallGalleryTheme(ThemeDef),
    SaveConfig,
    ToggleWidget(String),

    // Home Assistant
    ConnectHa,
    HaEvent(HaEvent),
    HaUrlChanged(String),
    HaTokenDelete,

    /// User tapped an actionable widget's header icon. Fires the action,
    /// or arms the widget when its confirmation gate is on. See #81.
    WidgetActionTriggered {
        entity_id: String,
        action: ha::ActionKind,
    },
    /// Resolves an armed widget. `false` disarms it without firing (#85).
    WidgetActionConfirmed {
        entity_id: String,
        confirmed: bool,
    },
    /// Per-widget toggle for the confirmation gate.
    WidgetRequireConfirmToggled(String, bool),

    /// User tapped the chevron. Collapses immediately, or asks the
    /// platform where the window is before expanding (#87).
    ToggleWidgetControls(String),
    /// The measurements the expansion needs, once the platform has
    /// answered: where the window sits and how tall its monitor is.
    /// Either may be `None` when the platform declines to say.
    WidgetControlsMeasured {
        entity_id: String,
        origin: Option<iced::Point>,
        monitor: Option<iced::Size>,
    },
    /// Slider moved. Records a pending value for that axis and sends
    /// only if the entity's throttle window has elapsed.
    ControlValueChanged {
        entity_id: String,
        axis: ha::AxisKind,
        value: f32,
    },
    /// Slider released. Always flushes the final value so an interaction
    /// never ends on a throttled-away intermediate, and starts the
    /// settle window.
    ControlReleased {
        entity_id: String,
        axis: ha::AxisKind,
    },
    /// The colour surface was dragged to a new point (#97). Its own
    /// message rather than a second component bolted onto
    /// [`Self::ControlValueChanged`]: an optional saturation would then
    /// ride along with every brightness and every setpoint, none of which
    /// can ever have one, and every handler would have to say so.
    ColorChanged {
        entity_id: String,
        hue: f32,
        saturation: f32,
    },
    /// The colour surface was let go. It names no axis because a colour
    /// is not dragged one axis at a time: both are flushed, and both go
    /// out in the single `hs_color` they share.
    ColorReleased {
        entity_id: String,
    },
    /// Ticks only while a pending value is outstanding, to retire the
    /// ones whose settle window ran out without a matching echo.
    PendingTick(iced::time::Instant),
    /// Ticks only while some widget is armed, to disarm the ones whose
    /// arm window ran out. A widget shows the prompt instead of its
    /// value, so nothing else would ever take it back.
    ArmedTick(iced::time::Instant),
    /// Result of the `WidgetActionTriggered` REST call. On `Ok` we
    /// silently rely on the follow-up `state_changed` WS event to refresh
    /// the widget; on `Err` we surface the reason in the status bar.
    WidgetActionResult {
        entity_id: String,
        result: Result<(), HaError>,
    },

    // UI events
    FocusMove {
        window_id: window::Id,
        direction: FocusDirection,
    },
    SavePressed,
    Saved,
    HaTokenDraftChanged(String),
    ConfigLoad(Result<Config, String>),
    StartDrag(window::Id),
    EntityHover {
        window: window::Id,
        on: bool,
    },

    EntitySearchChanged(String),
    SettingsPageSelected(SettingsPage),
    SettingsSearchChanged(String),
    CheckForUpdate,
    LastVersionChecked(Option<update::GitHubRelease>),

    WidgetMoved {
        id: window::Id,
        position: iced::Point,
    },
    PersistWidgetPositions,

    OpenConfigFile,
    OpenLogFile,
    TruncateLogFile,
    ResetConfig,

    WidgetSizeChanged(WidgetSize),
    WidgetPriorityChanged(String, Priority),
    WidgetNameChanged(String, String),

    WidgetVisibilityToggled(String, bool),
    WidgetVisibilityTriggerChanged(String, String),
    WidgetVisibilityConditionChanged(String, crate::widget_visibility::ConditionKind),
    WidgetVisibilityValueChanged(String, String),

    InstallUpdate,
    UpdateInstelled(Result<std::path::PathBuf, String>),
    RestartAfterUpdate(std::path::PathBuf),

    AutostartChanged(bool),

    CheckHaTokenPresence,
    HaTokenPresenceChecked(TokenPresence),
    RetryHaTokenPresence,

    AdaptiveFontChanged(bool),
    AdaptiveValueChanged(bool),
    ShowMeasurementInfoChanged(bool),

    SysInfoFetched(Information),
    SysExtrasFetched(SysinfoData),
    RefresSystemInfo,
    CopySystemInfo,
    CopySystemInfoMd,
}

impl Default for Snapdash {
    fn default() -> Self {
        Self::new()
    }
}

impl Snapdash {
    pub fn new() -> Self {
        let available_themes = available_themes();

        let theme = resolve_theme(DEFAULT_THEME, &available_themes)
            .or_else(|| available_themes.first())
            .cloned()
            .expect("at least bultin theme exists");

        Self {
            config: Config::default(),
            token_presence: TokenPresence::Unchecked,
            theme,
            available_themes,
            gallery: GalleryState::Idle,
            gallery_status: None,
            theme_import_status: None,
            ha: ha::HaState::default(),
            status: "-".into(),
            theme_options: vec![ThemeKind::MacLight, ThemeKind::MacDark],
            windows: HashMap::new(),
            entity_windows: HashMap::new(),
            entity_windows_opening: HashSet::new(),
            boot_open_done: false,
            settings_sensors: Vec::new(),
            selected_widgets: HashSet::new(),
            active_settings_sensors: Vec::new(),
            entity_search_query: String::new(),
            settings_page: SettingsPage::default(),
            settings_search: String::new(),
            update: update::UpdateStatus::default(),
            last_widget_move_at: None,
            config_save_in_flight: false,
            config_save_pending: false,
            sys_info: None,
            iced_sys_info: None,
            sys_data: None,
            pending: super::PendingValues::default(),
        }
    }

    fn try_combine_sysinfo(&mut self) {
        if let (Some(iced), Some(extras)) = (&self.iced_sys_info, &self.sys_data) {
            self.sys_info = Some(SystemInfo::from_parts(iced, extras));
            tracing::info!("system info ready");
        }
    }

    fn fetch_system_info(&mut self) -> Task<Message> {
        self.sys_info = None;
        self.sys_data = None;
        self.iced_sys_info = None;

        Task::batch([
            iced::system::information().map(Message::SysInfoFetched),
            Task::perform(
                async {
                    tokio::task::spawn_blocking(SysinfoData::collect)
                        .await
                        .unwrap_or_default()
                },
                Message::SysExtrasFetched,
            ),
        ])
    }

    fn rebuild_active_settings_sensors(&mut self) {
        self.active_settings_sensors = self
            .settings_sensors
            .iter()
            .filter(|sensor| self.selected_widgets.contains(&sensor.entity_id))
            .cloned()
            .collect();
    }

    pub fn rebuild_selected_widgets(&mut self) {
        self.selected_widgets = self.config.widgets.iter().cloned().collect();
        self.rebuild_active_settings_sensors();
    }

    pub fn rebuild_settings_sensors(&mut self) {
        let mut sensors: Vec<SettingsSensor> = self
            .ha
            .entities
            .values()
            .filter(|e| crate::ha::actions::is_widget_candidate(&e.entity_id))
            .map(|e| {
                let friendly_name = e
                    .attributes
                    .get("friendly_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&e.entity_id)
                    .to_string();

                let search_key = format!(
                    "{} {}",
                    friendly_name.to_lowercase(),
                    e.entity_id.to_lowercase()
                );

                SettingsSensor {
                    entity_id: e.entity_id.clone(),
                    friendly_name,
                    search_key,
                }
            })
            .collect();

        sensors.sort_by(|a, b| {
            a.friendly_name
                .cmp(&b.friendly_name)
                .then_with(|| a.entity_id.cmp(&b.entity_id))
        });

        self.settings_sensors = sensors;
        self.rebuild_active_settings_sensors();
    }

    fn set_window_entity_state(&mut self, entity_id: &str, new_state: &EntityState, pulse: bool) {
        let Some(window_id) = self.entity_windows.get(entity_id).copied() else {
            return;
        };

        let Some(window) = self.windows.get_mut(&window_id) else {
            self.entity_windows.remove(entity_id);
            return;
        };

        window.entity.last = Some(new_state.clone());

        if pulse {
            window.entity.pulse.trigger();
        }
    }

    fn apply_initial_states(&mut self, states: Vec<EntityState>) {
        self.ha.entities.clear();

        for state in states {
            let entity_id = state.entity_id.clone();

            self.set_window_entity_state(&entity_id, &state, false);
            self.ha.entities.insert(entity_id, state.clone());
        }
        self.rebuild_settings_sensors();
    }

    fn display_signature(&self, st: &EntityState) -> (String, Option<String>) {
        let formatted = crate::ui::format::format_entity_value(st);
        (formatted.main, formatted.detail)
    }

    fn should_pulse(&self, old: Option<&EntityState>, new: &EntityState) -> bool {
        match old {
            Some(old) => self.display_signature(old) != self.display_signature(new),
            None => true,
        }
    }

    fn apply_entity_state(&mut self, new_state: EntityState) {
        let entity_id = new_state.entity_id.clone();

        // Reconcile before anything renders. While an interaction is
        // outstanding the card must keep showing the pending value:
        // echoes lag the user's finger, so applying them here is exactly
        // what makes a slider fight the drag. The entity store is still
        // updated, only the *display* is held back, so the moment the
        // pending value resolves or expires there is real state to fall
        // back to. See `docs/adr/0002-pending-values-and-settle-window.md`.
        //
        // The echo goes in as the controls it built, not as loose values:
        // each control carries what Home Assistant just reported for
        // every axis it drives, which is exactly what it needs to
        // recognise its own echo (`docs/adr/0004-controls-and-axes.md`).
        let ha_driven = self.pending.reconcile(
            &entity_id,
            &ha::Capabilities::from_state(&new_state).controls,
        );

        let (pulse, should_refresh_settings) = match self.ha.entities.get(&entity_id) {
            None => (true, true),
            Some(old) => {
                let old_is_widget_candidate =
                    crate::ha::actions::is_widget_candidate(&old.entity_id);
                let new_is_widget_candidate =
                    crate::ha::actions::is_widget_candidate(&new_state.entity_id);

                let old_name = old.attributes.get("friendly_name").and_then(|v| v.as_str());
                let new_name = new_state
                    .attributes
                    .get("friendly_name")
                    .and_then(|v| v.as_str());

                (
                    self.should_pulse(Some(old), &new_state),
                    old_is_widget_candidate != new_is_widget_candidate || old_name != new_name,
                )
            }
        };

        if ha_driven {
            self.set_window_entity_state(&entity_id, &new_state, pulse);
        }
        self.ha.entities.insert(entity_id, new_state);

        if should_refresh_settings {
            self.rebuild_settings_sensors();
        }
    }

    /// Optimistic UI: pulse the widget immediately so the user sees
    /// "yes, I got your tap" instead of a 50-100 ms dead interval before
    /// HA's `state_changed` event arrives.
    fn pulse_widget(&mut self, entity_id: &str) {
        if let Some(&window_id) = self.entity_windows.get(entity_id)
            && let Some(window) = self.windows.get_mut(&window_id)
        {
            window.entity.pulse.trigger();
        }
    }

    fn is_armed(&self, entity_id: &str) -> bool {
        self.entity_windows
            .get(entity_id)
            .and_then(|id| self.windows.get(id))
            .is_some_and(|win| win.entity.armed_at.is_some())
    }

    fn set_armed(&mut self, entity_id: &str, armed: bool) {
        if let Some(&id) = self.entity_windows.get(entity_id)
            && let Some(win) = self.windows.get_mut(&id)
        {
            win.entity.armed_at = armed.then(std::time::Instant::now);
        }
    }

    /// The HA connection, but only while the socket is actually up.
    ///
    /// Both flags matter. `self.ha.connection` is `Some` for the entire
    /// reconnect cycle (the WS Subscription is keyed on it and stays set
    /// even while the socket is down); `HaEvent::Disconnected` only
    /// clears `.connected`. Guarding on the config alone would let a tap
    /// during a WS outage fire a REST request and quietly succeed, so the
    /// UI would say "disconnected" while the action still ran.
    fn live_connection(&self) -> Option<HaConnectionConfig> {
        match (self.ha.connected, self.ha.connection.clone()) {
            (true, Some(cfg)) => Some(cfg),
            _ => None,
        }
    }

    /// Every control an entity currently offers, in display order.
    fn controls_for(&self, entity_id: &str) -> Vec<ha::Control> {
        self.ha
            .entities
            .get(entity_id)
            .map(ha::Capabilities::from_state)
            .map(|caps| caps.controls)
            .unwrap_or_default()
    }

    /// Every control an entity offers, in display order, each carrying
    /// the locally-held value of the axes it drives.
    ///
    /// This is what the expanded card is built from, and it is the only
    /// place that answers "what does this widget offer for this axis
    /// right now?". Kept on `Snapdash` rather than in the view so the
    /// answer can be asserted on without rendering anything.
    pub fn control_views(&self, entity_id: &str) -> Vec<crate::ui::entity_window::ControlView> {
        self.controls_for(entity_id)
            .into_iter()
            .map(|control| crate::ui::entity_window::ControlView {
                pending: control
                    .axes()
                    .map(|axis| self.pending.shown(entity_id, axis.kind))
                    .collect(),
                control,
            })
            .collect()
    }

    /// Put the values one gesture produced on the wire, as the service
    /// calls the controls owning those axes build from them.
    ///
    /// Written over controls rather than over axes because the call is
    /// the control's to build (`docs/adr/0004-controls-and-axes.md`). A
    /// colour is one `light.turn_on` carrying both of its axes, and
    /// dispatching per axis would make it two calls and therefore two
    /// colours, the first of them one the user never pointed at.
    ///
    /// A control none of whose axes the gesture named stays silent, which
    /// is what keeps a brightness drag on a colour bulb from also
    /// restating the colour.
    fn send_gesture(
        &self,
        entity_id: &str,
        values: &[(ha::AxisKind, f32)],
        connection: HaConnectionConfig,
    ) -> Task<Message> {
        let actions: Vec<ha::ActionKind> = self
            .controls_for(entity_id)
            .iter()
            .filter_map(|control| {
                control.action(|kind| {
                    values
                        .iter()
                        .find(|(moved, _)| *moved == kind)
                        .map(|(_, value)| *value)
                })
            })
            .collect();

        Task::batch(
            actions.into_iter().map(|action| {
                self.dispatch_action(entity_id.to_owned(), action, connection.clone())
            }),
        )
    }

    /// Put an expanded widget back at its preset size and, when it had
    /// been lifted to make room, back where it started.
    fn collapse_widget(&mut self, entity_id: &str) -> Task<Message> {
        let Some(&id) = self.entity_windows.get(entity_id) else {
            return Task::none();
        };
        let Some(win) = self.windows.get_mut(&id) else {
            return Task::none();
        };
        let Some(expansion) = win.entity.expansion.take() else {
            return Task::none();
        };

        // The pending value belongs to the interaction, and the
        // interaction just ended. Keeping it would leave the card showing
        // a number HA never confirmed with no control left to correct it.
        self.pending.clear(entity_id);

        // Echoes that arrived mid-interaction updated the entity store but
        // deliberately did not reach the card, so `last` is now older than
        // what we know. Clearing also stops the expiry tick, so nothing
        // else is coming: push the latest truth across by hand, exactly as
        // `PendingTick` does for a value that timed out. Without this a
        // quiet entity leaves the collapsed card on its pre-drag value.
        if let Some(state) = self.ha.entities.get(entity_id).cloned() {
            self.set_window_entity_state(entity_id, &state, false);
        }

        // Through the platform helper, so Linux keeps its SHADOW_MARGIN
        // inflation. Resizing with the raw card size would clip the
        // drawn shadow, the same trap `WidgetSizeChanged` documents.
        let base = self.config.widget_settings.widget_size.window_size();
        let resize = iced::window::resize::<Message>(
            id,
            crate::ui::platform::window_size(base.width, base.height),
        );

        if expansion.lifted_by <= 0.0 {
            return resize;
        }

        // Give the lift back relative to wherever the widget is *now*,
        // not where it was when it expanded: the user may have dragged it
        // in the meantime, and that drag is theirs to keep.
        let lifted_by = expansion.lifted_by;
        resize.chain(iced::window::position(id).then(move |origin| match origin {
            Some(origin) => iced::window::move_to::<Message>(
                id,
                iced::Point::new(origin.x, origin.y + lifted_by),
            ),
            None => Task::none(),
        }))
    }

    /// Fire-and-forget `call_service`. The widget refreshes via the
    /// follow-up `state_changed`, so only the failure path is handled.
    fn dispatch_action(
        &self,
        entity_id: String,
        action: ha::ActionKind,
        connection: HaConnectionConfig,
    ) -> Task<Message> {
        let entity_for_result = entity_id.clone();
        Task::perform(
            async move {
                crate::ha::actions::call_service(
                    &connection.url,
                    &connection.token,
                    action,
                    &entity_id,
                )
                .await
            },
            move |result| Message::WidgetActionResult {
                entity_id: entity_for_result.clone(),
                result,
            },
        )
    }

    fn ha_error_status(error: &HaError) -> (String, LogType) {
        match error {
            HaError::AuthInvalid(msg) => {
                (format!("Authentication rejected: {msg}"), LogType::Error)
            }
            HaError::AuthExhausted { attempts } => (
                format!("Auth failed after {attempts} attempts - check your token"),
                LogType::Error,
            ),
            HaError::Protocol(_) => (format!("HA protocol error: {error}"), LogType::Error),
            HaError::Stale { elapsed } => (
                format!(
                    "HA connection stale ({}s), reconnecting...",
                    elapsed.as_secs()
                ),
                LogType::Warn,
            ),
            HaError::Connect(_) => (format!("Cannoct reach HA: {error}"), LogType::Warn),
            HaError::Timeout { what } => (
                format!("HA timeout waiting for {what}, reconnecting..."),
                LogType::Warn,
            ),
            HaError::SendFailed { what } => (
                format!("Failed to send {what} to HA, reconnecting..."),
                LogType::Warn,
            ),
            HaError::Closed => ("HA disconnected, reconnecting...".to_owned(), LogType::Info),
            // ServiceCall errors are user-driven (issue #81 actionable
            // widgets) and never flow through the connection health path
            // — they're surfaced directly by the `WidgetActionResult`
            // handler. Kept here for exhaustiveness only.
            HaError::ServiceCall { .. } => (format!("{error}"), LogType::Error),
        }
    }

    fn handle_ha_event(&mut self, ev: HaEvent) -> Task<Message> {
        match ev {
            HaEvent::Connected => {
                self.ha.connected = true;
                self.set_status("HA Connected", LogType::Info);
                self.ha.auth_failed = false;
                Task::none()
            }
            HaEvent::Disconnected(error) => {
                self.ha.connected = false;
                let (msg, severity) = Snapdash::ha_error_status(&error);
                self.set_status(msg, severity);
                self.ha.auth_failed = false;
                Task::none()
            }
            HaEvent::InitialState(states) => {
                self.apply_initial_states(states);

                // we can evaluate any rules whose triggers we previously didn't know.
                self.update_widget_visibility()
            }
            HaEvent::StateChanged { new_state } => {
                self.apply_entity_state(new_state);
                self.update_widget_visibility()
            }
            HaEvent::AuthFailed(error) => {
                self.ha.connected = false;
                self.ha.connection = None;
                self.ha.auth_failed = true;

                let (msg, _) = Snapdash::ha_error_status(&error);
                self.set_status(msg, LogType::Error);
                self.save_config()
            }
        }
    }

    // Set status for status bar and log this message to log.
    pub fn set_status(&mut self, msg: impl Into<String>, error_type: LogType) {
        let msg = msg.into();
        match error_type {
            LogType::Info => tracing::info!(target: "snapdash::status", "{msg}"),
            LogType::Warn => tracing::warn!(target: "snapdash::status", "{msg}"),
            LogType::Error => tracing::error!(target: "snapdash::status", "{msg}"),
            LogType::DoNotLog => (),
        }
        self.status = msg;
    }

    fn save_config(&mut self) -> Task<Message> {
        if self.config_save_in_flight {
            // newer state will be picked up when the in-flight save completes.

            self.config_save_pending = true;
            return Task::none();
        }

        self.config_save_in_flight = true;
        let cfg = self.config.clone();
        Task::perform(async move { cfg.save_async().await }, |_| {
            Message::SaveConfig
        })
    }

    fn is_entity_window_open(&self, entity_id: &str) -> bool {
        self.entity_windows.contains_key(entity_id)
            || self.entity_windows_opening.contains(entity_id)
    }

    /// Resolved title for a widget: custom override (if any) → HA
    /// friendly_name → bare entity_id without the domain prefix.
    pub fn display_name(&self, entity_id: &str) -> String {
        if let Some(name) = self.config.name_override(entity_id) {
            return name.to_owned();
        }

        if let Some(name) = self
            .ha
            .entities
            .get(entity_id)
            .and_then(|s| s.attributes.get("friendly_name").and_then(|v| v.as_str()))
        {
            return name.to_owned();
        }

        entity_id.split('.').nth(1).unwrap_or(entity_id).to_owned()
    }

    /// `true` when a widget should currently be shown. No rule → always
    /// visible. Rule → delegate to its evaluator against current HA
    /// entities.
    pub fn is_widget_visible(&self, entity_id: &str) -> bool {
        match self.config.visibility(entity_id) {
            None => true,
            Some(rule) => rule.evaluate(&self.ha.entities),
        }
    }

    /// Walk every rule-gated widget and emit OpenEntity / CloseWindow
    /// tasks for the deltas (newly-visible-but-not-open, or
    /// currently-open-but-now-hidden). Widgets WITHOUT a rule are not
    /// touched — they're under user control, otherwise closing one
    /// manually would have it bounce right back open.
    fn update_widget_visibility(&mut self) -> Task<Message> {
        let mut tasks: Vec<Task<Message>> = Vec::new();

        for (entity_id, rule) in self.config.visibility_rules() {
            if !self.config.widgets.contains(entity_id) {
                continue;
            }

            let visible = rule.evaluate(&self.ha.entities);

            if visible {
                if !self.is_entity_window_open(entity_id) {
                    tracing::debug!(entity = %entity_id, "visibility rule: opening widget");
                    tasks.push(Task::done(Message::OpenEntity(entity_id.clone())));
                }
            } else if let Some(&id) = self.entity_windows.get(entity_id) {
                tracing::debug!(entity = %entity_id, "visibility rule: closing widget");
                tasks.push(Task::done(Message::CloseWindow(id)));
            }
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => Task::none(),

            Message::ImportTheme => {
                // Open native file picker.
                // rfd returns FileHandle -> map to path

                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Theme", &["json"])
                            .set_title("Import Snapdash theme")
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::ThemeFilePicked,
                )
            }

            Message::ThemeFilePicked(None) => Task::none(),

            Message::ThemeFilePicked(Some(path)) => {
                match crate::theme::import_theme_file(&path) {
                    Ok(name) => {
                        // Re-scan
                        self.available_themes = crate::theme::available_themes();
                        self.theme_import_status = Some(Ok(format!("Imported `{name}`")));
                        self.set_status(format!("Theme '{name}' imported"), LogType::Info);
                    }
                    Err(e) => {
                        self.theme_import_status = Some(Err(e.clone()));
                        self.set_status(format!("Theme import failed: {e}"), LogType::Error);
                    }
                }
                Task::none()
            }

            Message::OpenThemeGallery => {
                self.gallery_status = None;

                // Refetch on every open (incl. the in-window Retry button)
                let fetch = if matches!(self.gallery, GalleryState::Loading) {
                    Task::none()
                } else {
                    self.gallery = GalleryState::Loading;

                    Task::perform(
                        async {
                            tokio::task::spawn_blocking(crate::theme::fetch_index)
                                .await
                                .unwrap_or_else(|e| Err(format!("join error: {e}")))
                        },
                        Message::GalleryIndexFetched,
                    )
                };

                match find_window_id(&self.windows, WindowKind::ThemeGallery, None) {
                    Some(opened) => iced::window::gain_focus(opened).chain(fetch),
                    None => {
                        let win = window_settings(iced::Size::new(640.0, 720.0), true);
                        let (id, task_id) = window::open(win);
                        task_id
                            .map(move |_| Message::WindowOpened {
                                id,
                                kind: WindowKind::ThemeGallery,
                            })
                            .chain(fetch)
                    }
                }
            }

            Message::GalleryIndexFetched(Ok(themes)) => {
                tracing::info!(count = themes.len(), "theme gallery loaded");
                self.gallery = GalleryState::Loaded(themes);
                Task::none()
            }

            Message::GalleryIndexFetched(Err(e)) => {
                tracing::warn!(error = %e, "theme gallery fetch failed");
                self.gallery = GalleryState::Failed(e);
                Task::none()
            }

            Message::InstallGalleryTheme(theme) => {
                match crate::theme::install_theme(&theme) {
                    Ok(name) => {
                        self.available_themes = crate::theme::available_themes();
                        self.gallery_status = Some(Ok(format!("Installed `{name}`")));
                        self.set_status(format!("Theme '{name}' installed"), LogType::Info);
                    }
                    Err(e) => {
                        self.gallery_status = Some(Err(e.clone()));
                        self.set_status(format!("Theme install failed: {e}"), LogType::Error);
                    }
                }
                Task::none()
            }

            Message::CopySystemInfo => {
                if let Some(snapshoot) = &self.sys_info {
                    let text = snapshoot.to_clipboard_string();
                    self.set_status("System info copied", LogType::DoNotLog);
                    return iced::clipboard::write(text);
                }
                Task::none()
            }

            Message::CopySystemInfoMd => {
                if let Some(snapshoot) = &self.sys_info {
                    let text = snapshoot.to_md_string();
                    self.set_status("System info copied as Markdown", LogType::DoNotLog);
                    return iced::clipboard::write(text);
                }
                Task::none()
            }

            Message::SysInfoFetched(i) => {
                self.iced_sys_info = Some(i);
                self.try_combine_sysinfo();
                Task::none()
            }

            Message::SysExtrasFetched(data) => {
                self.sys_data = Some(data);
                self.try_combine_sysinfo();
                Task::none()
            }

            Message::RefresSystemInfo => self.fetch_system_info(),

            Message::AutostartChanged(want) => {
                let previous = self.config.autostart;
                self.config.autostart = want;

                let result = if want {
                    crate::autostart::enable()
                } else {
                    crate::autostart::disable()
                };

                if let Err(e) = result {
                    self.config.autostart = previous;
                    tracing::error!(error = %e, want, "autostart update failed");
                    self.set_status(format!("Autostart change failed: {e}"), LogType::Error);
                    return Task::none();
                }

                let action = if want { "enabled" } else { "disabled" };
                self.set_status(format!("Autostart {action}"), LogType::Info);
                self.save_config()
            }
            Message::InstallUpdate => {
                // Guard
                if !matches!(
                    self.update.install,
                    crate::update::InstallProgress::Idle
                        | crate::update::InstallProgress::Failed(_)
                ) {
                    return Task::none();
                }

                self.update.install = crate::update::InstallProgress::Installing;
                self.set_status("Installing update...", LogType::Info);

                Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| -> anyhow::Result<std::path::PathBuf> {
                            use crate::update::installer;

                            let release = installer::fetch_latest_release()?;
                            let asset = installer::pick_asset(&release)?;

                            let temp = tempfile::tempdir()?;
                            let archive = installer::download_to(
                                &asset.archive_url,
                                temp.path(),
                                &asset.archive_name,
                            )?;
                            let checksum = installer::download_to(
                                &asset.checksum_url,
                                temp.path(),
                                &format!("{}.sha256", asset.archive_name),
                            )?;

                            installer::verify_checksum(&archive, &checksum)?;
                            installer::install_archive(&archive)
                        })
                        .await
                        .unwrap_or_else(|join_err| {
                            Err(anyhow::anyhow!("task join failed: {join_err}"))
                        })
                        .map_err(|e| format!("{e:#}"))
                    },
                    Message::UpdateInstelled,
                )
            }

            Message::UpdateInstelled(Ok(new_exec)) => {
                self.update.install = crate::update::InstallProgress::ReadyToRestart(new_exec);
                self.set_status("Update installed - restart to apply.", LogType::Info);
                Task::none()
            }

            Message::UpdateInstelled(Err(e)) => {
                tracing::error!(error = %e, "update install failed");
                self.set_status(format!("Update failed: {e}"), LogType::Error);
                self.update.install = crate::update::InstallProgress::Failed(e);
                Task::none()
            }

            Message::RestartAfterUpdate(exec) => {
                if let Err(e) = std::process::Command::new(&exec).spawn() {
                    tracing::error!(error = %e, exec = %exec.display(), "failed to spawn new exec");
                    self.update.install = crate::update::InstallProgress::Failed(format!(
                        "Failed to launch new version: {e}"
                    ));
                    return Task::none();
                }
                // Exit daemon - new install will take-over
                std::process::exit(0);
            }

            Message::CheckHaTokenPresence | Message::RetryHaTokenPresence => {
                self.token_presence = TokenPresence::Checking;

                Task::perform(
                    async {
                        tokio::task::spawn_blocking(token::presence)
                            .await
                            .unwrap_or_else(|e| TokenPresence::AccessFailed(e.to_string()))
                    },
                    Message::HaTokenPresenceChecked,
                )
            }

            Message::HaTokenPresenceChecked(token_status) => match token_status {
                TokenPresence::AccessFailed(e) => {
                    self.set_status(format!("Token auth failed: {e}"), LogType::Error);
                    self.token_presence = TokenPresence::AccessFailed(e);
                    Task::none()
                }
                TokenPresence::Missing => {
                    self.token_presence = TokenPresence::Missing;
                    self.config.ha_token_present = false;
                    self.save_config()
                }
                TokenPresence::Present => {
                    self.token_presence = TokenPresence::Present;
                    self.config.ha_token_present = true;
                    let mut tasks: Vec<Task<Message>> = vec![self.save_config()];

                    if !self.config.ha_url.trim().is_empty() {
                        let ha_connect = Task::perform(async {}, |_| Message::ConnectHa);
                        tasks.push(ha_connect);
                    }
                    Task::batch(tasks)
                }
                TokenPresence::Checking => Task::none(),
                TokenPresence::Unchecked => Task::none(),
            },

            Message::WidgetPriorityChanged(entity_id, priority) => {
                // Normal is the default and is skipped on serialize, so
                // storing it unconditionally still keeps config lean -
                // an entry that ends up all-default is pruned on save.
                self.config.widget_mut(&entity_id).priority = priority;
                self.save_config()
            }

            Message::WidgetSizeChanged(size) => {
                self.config.widget_settings.widget_size = size;

                // Route the card size through the platform helper so Linux gets its
                // SHADOW_MARGIN inflation (composited.rs:28) — same path as
                // window_settings() uses at window creation time. Without this,
                // resizing on Linux drops the shadow margin and clips drawn shadows.
                let card = size.window_size();
                let surface = crate::ui::platform::window_size(card.width, card.height);

                // Collapse anything currently expanded first. The resize
                // below would otherwise silently overwrite its grown
                // height, leaving the widget at preset size with its
                // controls still rendered and its lift never given back.
                let expanded: Vec<String> = self
                    .windows
                    .values()
                    .filter(|win| win.entity.is_expanded())
                    .map(|win| win.entity.entity_id.clone())
                    .collect();

                let mut tasks: Vec<Task<Message>> = expanded
                    .iter()
                    .map(|entity_id| self.collapse_widget(entity_id))
                    .collect();

                tasks.extend(
                    self.windows
                        .iter()
                        .filter(|(_id, win)| matches!(win.kind, WindowKind::Entity { .. }))
                        .map(|(id, _win)| iced::window::resize::<Message>(*id, surface)),
                );

                tasks.push(self.save_config());
                Task::batch(tasks)
            }

            Message::OpenConfigFile => {
                match Config::config_path() {
                    Ok(path) => {
                        if let Err(e) = open::that(&path) {
                            tracing::warn!(path = %path.display(), error = %e, "failed to open config file")
                        }
                    }
                    Err(e) => tracing::warn!(error = %e, "no config path available"),
                }
                Task::none()
            }

            Message::OpenLogFile => {
                match crate::logger::log_path() {
                    Ok(path) => {
                        #[cfg(target_os = "windows")]
                        {
                            let target = path.parent().map(|p| p.to_path_buf()).unwrap_or(path);
                            if let Err(e) = open::that(&target) {
                                tracing::warn!(path = %target.display(), error = %e, "failed to open log dir");
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            if let Err(e) = open::that(&path) {
                                tracing::warn!(path = %path.display(), error = %e, "failed to open log file")
                            }
                        }
                    }
                    Err(e) => tracing::warn!(error = %e, "no log path available"),
                }
                Task::none()
            }

            Message::ResetConfig => {
                self.config = Config::default();
                self.ha.connection = None;
                self.ha.connected = false;
                self.selected_widgets.clear();
                self.rebuild_active_settings_sensors();
                self.set_status("Configuration rest to defaults", LogType::Info);
                self.save_config()
            }

            Message::TruncateLogFile => match logger::clear_log() {
                Ok(_) => {
                    self.set_status("Log has been truncated.", LogType::DoNotLog);
                    Task::none()
                }
                Err(e) => {
                    self.set_status(format!("Can not clear log. {e}"), LogType::Warn);
                    Task::none()
                }
            },

            Message::SettingsPageSelected(page) => {
                self.theme_import_status = None;
                self.settings_page = page;

                // Refresh sysinfo page on selection
                if matches!(page, SettingsPage::Sysinfo) {
                    self.fetch_system_info()
                } else {
                    Task::none()
                }
            }

            Message::SettingsSearchChanged(value) => {
                self.settings_search = value;
                Task::none()
            }

            Message::EntitySearchChanged(value) => {
                self.entity_search_query = value;
                Task::none()
            }

            Message::HaTokenDelete => match token::delete_raw() {
                Ok(_) => {
                    self.config.ha_token_present = false;
                    self.ha.connection = None;
                    self.ha.connected = false;
                    self.token_presence = TokenPresence::Missing;
                    self.set_status("Token deleted from key-chain", LogType::Info);
                    tracing::warn!("HA disconected due to erased token.");
                    self.save_config()
                }
                Err(keyring::Error::NoEntry) => {
                    self.config.ha_token_present = false;
                    self.ha.connection = None;
                    self.ha.connected = false;
                    self.token_presence = TokenPresence::Missing;
                    self.set_status("Token deleted from key-chain", LogType::Info);
                    tracing::warn!("HA disconected due to erased token.");
                    self.save_config()
                }
                Err(e) => {
                    self.set_status(
                        format!("Could not delete token from key-chain {e}"),
                        LogType::Error,
                    );
                    self.token_presence = TokenPresence::AccessFailed(e.to_string());
                    Task::none()
                }
            },

            Message::ConfigLoad(res) => {
                match res {
                    Ok(cfg) => {
                        let mut tasks: Vec<Task<Message>> = Vec::new();

                        self.config = cfg;
                        crate::autostart::validate_state(self.config.autostart);

                        // Resolve the persisted theme name against the catalog.
                        // Falls back to first builtin theme if name is unknown
                        if let Some(theme) =
                            resolve_theme(&self.config.theme, &self.available_themes)
                        {
                            self.theme = theme.clone();
                        } else {
                            tracing::warn!(name = %self.config.theme, "unknown theme, using defualt");
                            self.theme = self.available_themes[0].clone();
                        }

                        self.rebuild_selected_widgets();
                        self.set_status("Config loaded", LogType::Info);

                        if !self.boot_open_done {
                            let open_task = if self.config.widgets.is_empty() {
                                Task::perform(async {}, |_| Message::OpenSettings)
                            } else {
                                Task::perform(async {}, |_| Message::OpenEntity("".to_string()))
                            };

                            self.boot_open_done = true;
                            tasks.push(open_task);
                        }

                        let key_chain_task = Task::perform(
                            async {
                                tokio::task::spawn_blocking(token::presence)
                                    .await
                                    .unwrap_or_else(|e| TokenPresence::AccessFailed(e.to_string()))
                            },
                            Message::HaTokenPresenceChecked,
                        );

                        return if tasks.is_empty() {
                            Task::none()
                        } else {
                            // Lazily create key-chain task. The task should be the last on boot
                            // while it is blocking spawn.
                            Task::batch(tasks).chain(key_chain_task)
                        };
                    }
                    Err(e) => {
                        self.set_status(format!("Config load failed: {e}"), LogType::Error);
                    }
                }
                Task::none()
            }

            Message::StartDrag(id) => iced::window::drag(id),

            Message::CloseWindow(id) => iced::window::close(id),

            Message::QuitApp => iced::exit(),

            Message::EntityHover { window, on } => {
                if let Some(w) = self.windows.get_mut(&window) {
                    w.entity.hovered = on;
                    // Leaving the widget answers the prompt by walking
                    // away. The confirm buttons live inside the card, so
                    // moving between them never crosses this boundary.
                    if !on {
                        w.entity.armed_at = None;
                    }
                }
                Task::none()
            }

            Message::ToggleWidget(entity_id) => {
                let mut task: Vec<Task<Message>> = Vec::new();

                if let Some(widget) = self.config.widgets.iter().position(|e| e == &entity_id) {
                    // close widget
                    self.config.widgets.remove(widget);

                    if let Some(id) = self.entity_windows.remove(&entity_id) {
                        self.windows.remove(&id);
                        task.push(iced::window::close(id));
                    }
                } else {
                    // append new widget
                    self.config.widgets.push(entity_id.clone());
                    task.push(Task::perform(async {}, move |_| {
                        Message::OpenEntity(entity_id.clone())
                    }));
                }

                self.rebuild_selected_widgets();
                // save widget configuration
                task.push(self.save_config());

                Task::batch(task)
            }

            Message::HaTokenDraftChanged(val) => {
                self.ha.token_draft = val;
                Task::none()
            }

            Message::WindowOpened { id, kind } => {
                // Lazy-fetch system info once we have at least one window -
                // the compositor (and therefore grapichs_adapter info).
                // Graphics info is only available after the first windows open.
                // If we trigger sys_info on the boot() the request races compositor init.

                let mut entity = EntityWindowState::default();

                if let WindowKind::Entity { entity_id } = &kind {
                    entity.entity_id = entity_id.clone();

                    if let Some(st) = self.ha.entities.get(&entity.entity_id) {
                        entity.last = Some(st.clone());
                    }

                    self.entity_windows.insert(entity_id.clone(), id);
                    self.entity_windows_opening.remove(entity_id);
                }

                self.windows.insert(id, WindowState { kind, entity });

                Task::none()
            }

            Message::WindowClosed(id) => {
                if let Some(window) = self.windows.remove(&id)
                    && let WindowKind::Entity { entity_id } = window.kind
                {
                    self.entity_windows.remove(&entity_id);
                    self.entity_windows_opening.remove(&entity_id);
                    // No card left to render the pending value, and no
                    // control left to resolve it. Leaving it behind would
                    // keep the settle-window subscription awake and make
                    // the widget reopen showing a number HA never
                    // confirmed.
                    self.pending.clear(&entity_id);
                }

                if self.windows.is_empty() {
                    // Stay alive if any configured widget has a
                    // visibility rule — the next HA state change can
                    // bring it back via update_widget_visibility. Without
                    // this guard a rule that flips false for the last
                    // visible widget kills the process and there's
                    // nothing left to reopen it. Users without rules
                    // get the original "close everything → exit"
                    // behaviour; rule users quit via Settings → Quit.
                    let has_rule_gated_widget = self
                        .config
                        .widgets
                        .iter()
                        .any(|w| self.config.visibility(w).is_some());

                    if has_rule_gated_widget {
                        Task::none()
                    } else {
                        iced::exit()
                    }
                } else {
                    Task::none()
                }
            }

            Message::HaUrlChanged(val) => {
                self.config.ha_url = val;
                Task::none()
            }

            Message::SavePressed => {
                self.set_status("Saving...", LogType::DoNotLog);
                if !self.ha.token_draft.trim().is_empty() {
                    match token::set(self.ha.token_draft.trim()) {
                        Ok(()) => {
                            self.config.ha_token_present = true;
                            self.token_presence = TokenPresence::Present;
                            self.ha.token_draft.clear();
                            self.set_status("Token saved into keychain.", LogType::Info);
                        }
                        Err(e) => {
                            self.token_presence = TokenPresence::AccessFailed(e.to_string());
                            self.set_status(format!("Keychain error: {e}"), LogType::Error);
                            return Task::none();
                        }
                    }
                }
                self.save_config().chain(Task::done(Message::Saved))
            }

            Message::Saved => {
                self.set_status("Saved", LogType::DoNotLog);
                Task::perform(async {}, |_| Message::ConnectHa)
            }
            Message::OpenSettingsTo(page) => {
                self.settings_page = page;
                self.update(Message::OpenSettings)
            }

            Message::OpenSettings => {
                // if Settings window is opened, give focus
                //

                if let Some(settings_id) = find_window_id(&self.windows, WindowKind::Settings, None)
                {
                    return iced::window::gain_focus::<Message>(settings_id);
                }

                let sysinfo_task = if self.sys_info.is_none() {
                    self.fetch_system_info()
                } else {
                    Task::none()
                };

                // The platform helper adds a transparent shadow margin on
                // Linux (where we render our own shader shadow) and is a
                // no-op on macOS/Windows (where the OS clips + draws its
                // own shadow). See `ui::platform` module doc.
                let settings = window_settings(iced::Size::new(920.0, 640.0), true);
                let (id, task_id) = window::open(settings);
                task_id
                    .map(move |_| Message::WindowOpened {
                        id,
                        kind: WindowKind::Settings,
                    })
                    .chain(sysinfo_task)
            }

            Message::OpenEntity(entity_id) => {
                // Bulk boot-open: skip widgets currently hidden by their rule.
                // At boot HA is not connected, so an gated widget eveluates to "trigger unkonwn"
                // and will open later once HA is connected
                let widgets: Vec<String> = if entity_id.is_empty() {
                    self.config
                        .widgets
                        .iter()
                        .filter(|id| self.is_widget_visible(id))
                        .cloned()
                        .collect()
                } else {
                    vec![entity_id]
                };

                // Platform helper: adds shadow margin on Linux, pass-through
                // on macOS/Windows. See `ui::platform` module doc.
                let mut task = Vec::new();

                for widget in widgets {
                    if self.is_entity_window_open(&widget) {
                        continue;
                    }

                    // Reserve slot immediatelly, so another event fired before our
                    // WindowOpened arrives sees the entity as already-being-opened and skips it.
                    self.entity_windows_opening.insert(widget.clone());

                    let mut win_settings = window_settings(
                        self.config.widget_settings.widget_size.window_size(),
                        false,
                    );
                    if let Some(saved) = self.config.position(&widget) {
                        win_settings.position =
                            window::Position::Specific(iced::Point::new(saved.x, saved.y));
                    }

                    let (id, task_id) = window::open(win_settings);
                    task.push(task_id.map(move |_| Message::WindowOpened {
                        id,
                        kind: WindowKind::Entity {
                            entity_id: widget.clone(),
                        },
                    }));
                }

                if task.is_empty() {
                    Task::none()
                } else {
                    Task::batch(task)
                }
            }

            Message::ThemeSelected(name) => {
                if let Some(theme) = self
                    .available_themes
                    .iter()
                    .find(|t| t.name == name)
                    .cloned()
                {
                    self.theme = theme;
                    self.config.theme = name;
                    self.theme_import_status = None;
                    self.save_config()
                } else {
                    Task::none()
                }
            }

            Message::SaveConfig => {
                self.config_save_in_flight = false;
                if self.config_save_pending {
                    self.config_save_pending = false;
                    return self.save_config();
                }
                Task::none()
            }

            Message::ConnectHa => {
                if self.config.ha_url.trim().is_empty() {
                    self.ha.connected = false;
                    self.ha.connection = None;
                    self.set_status("HA not enabled - URL", LogType::Error);
                    return Task::none();
                }

                match &self.token_presence {
                    TokenPresence::Present => {}
                    TokenPresence::Missing => {
                        self.ha.connected = false;
                        self.ha.connection = None;
                        self.set_status("HA not enabled - missing token", LogType::Error);
                        return Task::none();
                    }
                    TokenPresence::AccessFailed(e) => {
                        self.ha.connected = false;
                        self.ha.connection = None;
                        self.set_status(format!("Keychain access needed: {e}"), LogType::Warn);
                        return Task::none();
                    }
                    TokenPresence::Checking => {
                        self.set_status("Checking token in keychain...", LogType::DoNotLog);
                        return Task::none();
                    }
                    TokenPresence::Unchecked => {
                        return Task::done(Message::CheckHaTokenPresence);
                    }
                }
                let stored_token = match token::get_raw() {
                    Ok(t) => t,
                    Err(e) => {
                        match e {
                            keyring::Error::NoEntry => {
                                self.set_status(
                                    format!("Missing token in key-chain {e}"),
                                    LogType::Error,
                                );
                                self.config.ha_token_present = false;
                                self.token_presence = TokenPresence::Missing;
                            }
                            _ => {
                                self.set_status(
                                    format!("Access to key-chain was denied. {e}"),
                                    LogType::Warn,
                                );
                                self.token_presence = TokenPresence::AccessFailed(e.to_string());
                            }
                        }
                        self.ha.connected = false;
                        self.ha.connection = None;

                        return self.save_config().chain(Task::done(Message::Noop));
                    }
                };

                let next_connection = HaConnectionConfig {
                    url: self.config.ha_url.clone(),
                    token: stored_token,
                };

                if self.ha.connection.as_ref() != Some(&next_connection) {
                    self.ha.connected = false;
                    self.set_status("Connecting to HA ...", LogType::Info);
                    self.ha.connection = Some(next_connection);
                }

                Task::none()
            }

            Message::HaEvent(ev) => self.handle_ha_event(ev),

            Message::WidgetActionTriggered { entity_id, action } => {
                let Some(connection) = self.live_connection() else {
                    self.set_status(
                        format!("Cannot toggle {entity_id}: not connected to Home Assistant"),
                        LogType::Warn,
                    );
                    return Task::none();
                };

                // Gated widgets take the first tap as "arm", not "fire".
                // Nothing reaches the wire until the prompt is answered.
                if self.config.require_confirm(&entity_id) && !self.is_armed(&entity_id) {
                    self.set_armed(&entity_id, true);
                    return Task::none();
                }

                self.pulse_widget(&entity_id);
                self.dispatch_action(entity_id, action, connection)
            }

            Message::WidgetActionConfirmed {
                entity_id,
                confirmed,
            } => {
                self.set_armed(&entity_id, false);

                if !confirmed {
                    return Task::none();
                }

                // Re-derive rather than carrying the action through the
                // prompt: it is a plain domain-level toggle or trigger, so
                // the entity id is the whole input, and the connection has
                // to be re-checked anyway now that time has passed.
                let Some(action) = ha::ActionKind::primary_for_entity(&entity_id) else {
                    return Task::none();
                };
                let Some(connection) = self.live_connection() else {
                    self.set_status(
                        format!("Cannot toggle {entity_id}: not connected to Home Assistant"),
                        LogType::Warn,
                    );
                    return Task::none();
                };

                self.pulse_widget(&entity_id);
                self.dispatch_action(entity_id, action, connection)
            }

            Message::WidgetRequireConfirmToggled(entity_id, on) => {
                self.config.widget_mut(&entity_id).require_confirm = on;
                // Turning the gate off must not leave a widget sitting
                // armed with no prompt left to resolve it.
                if !on {
                    self.set_armed(&entity_id, false);
                }
                self.save_config()
            }

            Message::ToggleWidgetControls(entity_id) => {
                if self
                    .entity_windows
                    .get(&entity_id)
                    .and_then(|id| self.windows.get(id))
                    .is_some_and(|win| win.entity.is_expanded())
                {
                    return self.collapse_widget(&entity_id);
                }

                let Some(&id) = self.entity_windows.get(&entity_id) else {
                    return Task::none();
                };

                // Where the window sits and how tall its monitor is are
                // both platform questions, so the expansion cannot be
                // decided synchronously. Ask, then act on the answers.
                iced::window::position(id).then(move |origin| {
                    let entity_id = entity_id.clone();
                    iced::window::monitor_size(id).map(move |monitor| {
                        Message::WidgetControlsMeasured {
                            entity_id: entity_id.clone(),
                            origin,
                            monitor,
                        }
                    })
                })
            }

            Message::WidgetControlsMeasured {
                entity_id,
                origin,
                monitor,
            } => {
                let Some(&id) = self.entity_windows.get(&entity_id) else {
                    return Task::none();
                };
                // Nothing to reveal means nothing to grow into.
                let controls = self.controls_for(&entity_id);
                if controls.is_empty() {
                    return Task::none();
                }

                let size = self.config.widget_settings.widget_size;
                let base = size.window_size();
                let grown_by = size.controls_height(&controls);

                // Without a reported position there is no way to tell
                // whether the widget is near an edge, so it grows
                // downwards. Same fallback as an unknown monitor.
                let lifted_by = origin.map_or(0.0, |origin| {
                    lift_needed(origin.y, base.height, grown_by, monitor.map(|m| m.height))
                });

                let Some(win) = self.windows.get_mut(&id) else {
                    return Task::none();
                };
                win.entity.expansion = Some(Expansion {
                    grown_by,
                    lifted_by,
                });

                let grow = iced::window::resize::<Message>(
                    id,
                    crate::ui::platform::window_size(base.width, base.height + grown_by),
                );

                match (lifted_by > 0.0, origin) {
                    (true, Some(origin)) => iced::window::move_to::<Message>(
                        id,
                        iced::Point::new(origin.x, origin.y - lifted_by),
                    )
                    .chain(grow),
                    _ => grow,
                }
            }

            Message::ControlValueChanged {
                entity_id,
                axis,
                value,
            } => {
                let Some(connection) = self.live_connection() else {
                    self.set_status(
                        format!("Cannot adjust {entity_id}: not connected to Home Assistant"),
                        LogType::Warn,
                    );
                    return Task::none();
                };

                // `set` says no when the entity's throttle window swallows
                // this move. The value is not lost: it stays in `shown`,
                // and `ControlReleased` flushes the final one.
                if !self
                    .pending
                    .set(&entity_id, &[(axis, value)], std::time::Instant::now())
                {
                    return Task::none();
                }

                self.send_gesture(&entity_id, &[(axis, value)], connection)
            }

            Message::ControlReleased { entity_id, axis } => {
                // The bookkeeping happens whether or not HA is reachable,
                // and before the connection is checked. `release` is what
                // starts the settle window, and a pending value that never
                // got one can never expire: the card would keep showing a
                // number the house never confirmed, with nothing left to
                // correct it. Only the service call needs a live socket.
                let Some(flushed) =
                    self.pending
                        .release(&entity_id, &[axis], std::time::Instant::now())
                else {
                    return Task::none();
                };

                let Some(connection) = self.live_connection() else {
                    return Task::none();
                };

                // Written over whatever the release hands back rather than
                // over the axis this message names, so that a control
                // which grows a second axis cannot quietly leave that axis
                // unsent.
                self.send_gesture(&entity_id, &flushed, connection)
            }

            Message::ColorChanged {
                entity_id,
                hue,
                saturation,
            } => {
                let Some(connection) = self.live_connection() else {
                    self.set_status(
                        format!("Cannot adjust {entity_id}: not connected to Home Assistant"),
                        LogType::Warn,
                    );
                    return Task::none();
                };

                // One batch, so one throttle window for the whole gesture
                // and a `last_sent` recorded on both axes when it goes
                // out. Two `set` calls would spend the window on the hue
                // and leave the saturation unable to ever recognise its
                // echo (#96).
                let moved = [
                    (ha::AxisKind::Hue, hue),
                    (ha::AxisKind::Saturation, saturation),
                ];

                if !self
                    .pending
                    .set(&entity_id, &moved, std::time::Instant::now())
                {
                    return Task::none();
                }

                self.send_gesture(&entity_id, &moved, connection)
            }

            Message::ColorReleased { entity_id } => {
                let Some(flushed) = self.pending.release(
                    &entity_id,
                    &[ha::AxisKind::Hue, ha::AxisKind::Saturation],
                    std::time::Instant::now(),
                ) else {
                    return Task::none();
                };

                let Some(connection) = self.live_connection() else {
                    return Task::none();
                };

                // Both axes come back from one release and go out as one
                // `SetHs`, not as two calls: `send_gesture` asks the
                // control what to send, and a colour surface answers with
                // a single `hs_color`.
                self.send_gesture(&entity_id, &flushed, connection)
            }

            Message::PendingTick(now) => {
                // A widget that just lost its pending value is still
                // showing that local number, and nothing else will
                // correct it: in exactly the case the timeout exists for,
                // HA has gone quiet. So push the last known truth back
                // onto each retired widget by hand.
                for entity_id in self.pending.expire(now) {
                    if let Some(state) = self.ha.entities.get(&entity_id).cloned() {
                        self.set_window_entity_state(&entity_id, &state, false);
                    }
                }
                Task::none()
            }

            Message::ArmedTick(now) => {
                for win in self.windows.values_mut() {
                    if win.entity.arm_expired(now) {
                        win.entity.armed_at = None;
                    }
                }
                Task::none()
            }

            Message::WidgetActionResult { entity_id, result } => {
                match result {
                    Ok(()) => {
                        // The follow-up state_changed WS event will
                        // refresh the widget shortly. Nothing to do here.
                        tracing::debug!(%entity_id, "widget action acknowledged");
                    }
                    Err(err) => {
                        self.set_status(
                            format!("Failed to toggle {entity_id}: {err}"),
                            LogType::Error,
                        );
                    }
                }
                Task::none()
            }

            Message::FocusMove {
                window_id,
                direction,
            } => {
                let is_sesttings = self
                    .windows
                    .get(&window_id)
                    .is_some_and(|w| matches!(w.kind, WindowKind::Settings));

                if !is_sesttings {
                    return Task::none();
                }

                match direction {
                    FocusDirection::Next => iced::widget::operation::focus_next(),
                    FocusDirection::Previous => iced::widget::operation::focus_previous(),
                }
            }

            Message::CheckForUpdate => {
                Task::perform(update::get_latest_version(), Message::LastVersionChecked)
            }

            Message::LastVersionChecked(release) => {
                self.update.record_check(release);
                Task::none()
            }
            Message::OpenReleaseNotes => {
                if let Some(opened) = find_window_id(&self.windows, WindowKind::ReleaseNotes, None)
                {
                    return iced::window::gain_focus::<Message>(opened);
                }

                let settings = window_settings(iced::Size::new(560.0, 640.0), false);
                let (id, task_id) = window::open(settings);
                task_id.map(move |_| Message::WindowOpened {
                    id,
                    kind: WindowKind::ReleaseNotes,
                })
            }
            Message::OpenWidgetSettings(entity_id) => {
                let kind = WindowKind::WidgetSettings { entity_id };

                if let Some(opened) = find_window_id(&self.windows, kind.clone(), None) {
                    return iced::window::gain_focus::<Message>(opened);
                }

                let settings = window_settings(iced::Size::new(540.0, 540.0), true);
                let (id, task_id) = window::open(settings);
                task_id.map(move |_| Message::WindowOpened {
                    id,
                    kind: kind.clone(),
                })
            }
            Message::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    tracing::warn!(url, error = %e, "failed to open URL");
                }
                Task::none()
            }

            Message::WidgetMoved { id, position } => {
                let Some(window) = self.windows.get_mut(&id) else {
                    return Task::none();
                };

                // Expanding near the bottom edge lifts the window out of
                // the place it belongs, so what gets persisted is where
                // the collapsed card lives rather than where the window
                // happens to be right now (#87).
                let resting = window.entity.resting_position(position);

                let WindowKind::Entity { entity_id } = &window.kind else {
                    return Task::none();
                };

                let entity_id = entity_id.clone();
                let new_position = WidgetPosition {
                    x: resting.x,
                    y: resting.y,
                };

                // Filter: if position is not moved - do nothing. Without filter we will fire
                // debounce timer with every programatic move (ex. window manager snap-to-grid on borders).

                if self.config.position(&entity_id) == Some(new_position) {
                    return Task::none();
                }

                self.config.widget_mut(&entity_id).position = Some(new_position);
                self.last_widget_move_at = Some(std::time::Instant::now());

                Task::perform(
                    async {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    },
                    |_| Message::PersistWidgetPositions,
                )
            }

            Message::WidgetNameChanged(entity_id, raw) => {
                // Empty/whitespace clears the override so the widget falls
                // back to HA's friendly_name. Storing `None` rather than
                // removing a key is enough: a widget whose every field is
                // back at its default is pruned on save.
                self.config.widget_mut(&entity_id).name = (!raw.trim().is_empty()).then_some(raw);
                self.save_config()
            }

            Message::WidgetVisibilityToggled(entity_id, on) => {
                let post_task = if on {
                    // Sensible default: self-trigger + IsAvailable —
                    // covers "show this sensor only while it reports
                    // something real" without forcing the user to pick a
                    // trigger entity upfront.
                    self.config.widget_mut(&entity_id).visibility =
                        Some(crate::widget_visibility::VisibilityRule {
                            trigger: entity_id.clone(),
                            condition: crate::widget_visibility::VisibilityCondition::IsAvailable,
                        });
                    self.update_widget_visibility()
                } else {
                    self.config.widget_mut(&entity_id).visibility = None;
                    if self.is_entity_window_open(&entity_id) {
                        Task::none()
                    } else {
                        Task::done(Message::OpenEntity(entity_id))
                    }
                };

                self.save_config().chain(post_task)
            }

            Message::WidgetVisibilityTriggerChanged(entity_id, trigger) => {
                if let Some(rule) = self.config.visibility_mut(&entity_id) {
                    rule.trigger = trigger;
                }
                self.save_config().chain(self.update_widget_visibility())
            }

            Message::WidgetVisibilityConditionChanged(entity_id, kind) => {
                if let Some(rule) = self.config.visibility_mut(&entity_id) {
                    let raw = rule
                        .condition
                        .raw_value()
                        .map(String::from)
                        .unwrap_or_default();
                    rule.condition = kind.with_value(raw);
                }
                self.save_config().chain(self.update_widget_visibility())
            }

            Message::WidgetVisibilityValueChanged(entity_id, raw) => {
                if let Some(rule) = self.config.visibility_mut(&entity_id) {
                    let kind =
                        crate::widget_visibility::ConditionKind::from_condition(&rule.condition);
                    rule.condition = kind.with_value(raw);
                }
                self.save_config().chain(self.update_widget_visibility())
            }

            Message::PersistWidgetPositions => {
                let Some(last) = self.last_widget_move_at else {
                    return Task::none();
                };

                // Trailing-edge debounce: if last move < 500ms, widget has moved once more
                // delay write

                if last.elapsed() < Duration::from_millis(500) {
                    return Task::none();
                }
                self.last_widget_move_at = None;
                self.save_config()
            }

            Message::AdaptiveFontChanged(b) => {
                self.config.widget_settings.adaptive.adaptive_font = b;
                self.save_config()
            }
            Message::AdaptiveValueChanged(b) => {
                self.config.widget_settings.adaptive.adaptive_value = b;
                self.save_config()
            }

            Message::ShowMeasurementInfoChanged(b) => {
                self.config.widget_settings.show_measurement_info = b;
                self.save_config()
            }

            Message::AnimationFrame(now) => {
                for window in self.windows.values_mut() {
                    if let WindowKind::Entity { .. } = window.kind {
                        window.entity.pulse.tick(now);
                    }
                }

                Task::none()
            }
        }
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        let Some(win) = self.windows.get(&id) else {
            return iced::widget::text("Loading...").into();
        };

        let inner = crate::ui::chrome::window_content(self, win, id);

        let inner = match win.kind {
            WindowKind::Entity { .. } => {
                let with_gear = crate::ui::chrome::with_gear_overlay(self, inner, win);
                crate::ui::chrome::with_mouse_area(with_gear, id, win)
            }
            WindowKind::Settings => inner,
            WindowKind::ReleaseNotes => inner,
            WindowKind::ThemeGallery => inner,
            WindowKind::WidgetSettings { .. } => inner,
        };

        // Platform-specific outer wrapping:
        // - Linux: transparent `SHADOW_MARGIN` padding around the card so
        //   the iced-wgpu shader shadow has room to fade inside the surface.
        // - macOS/Windows: pass-through. Platform-specific window settings clip the
        //   window to a rounded shape and the OS draws the drop shadow.
        crate::ui::platform::wrap_outer(inner)
    }

    /// Surface clear color. **Linux-only**: set to `Color::TRANSPARENT` so the
    /// cleared pixels in the shadow margin actually composite as transparent
    /// (the default theme background would show as an opaque color there).
    ///
    /// On macOS/Windows we intentionally do NOT install this callback (see
    /// `main.rs`) — the OS-level rounded-corner/shadow path clips the window to
    /// a rounded shape before any cleared pixel is visible, so the default
    /// opaque theme background is never seen and matches the pre-shadow-margin
    /// behavior users reported looked "nice and optimal".
    #[cfg(target_os = "linux")]
    pub fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: self.theme.palette.text_primary,
        }
    }
}
