use crate::ha::{EntityState, HaConnectionConfig, HaEvent};
use crate::logger::LogType;
use crate::{logger as log, update};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use iced::{Element, Task};
use iced::{Subscription, window};

use crate::config::Config;
use crate::secrets;
use crate::theme::ThemeKind;

#[derive(Debug, Clone, PartialEq)]
pub enum WindowKind {
    Settings,
    Entity { entity_id: String },
}
#[derive(Debug)]
pub struct WindowState {
    pub kind: WindowKind,
    pub entity: EntityWindowState,
    pub debug: WindowDebugState,
}

#[derive(Debug, Default, Clone)]
pub struct EntityWindowState {
    pub entity_id: String,
    pub last: Option<EntityState>,
    pub pulse: f32, // TODO: Replace with Animation/spring. Currently just easy "animation paramter" (0..1), později nahradit Animation/spring
    pub hovered: bool,
}

#[derive(Debug, Default, Clone)]
pub struct WindowDebugState {
    pub redraw_total: u64,
    pub redraws_last_second: usize,
    pub last_redraw_at: Option<Instant>,
    pub last_redraw_delta_ms: Option<f32>,
    recent_redraws: VecDeque<Instant>,
}

impl WindowDebugState {
    fn record_redraw(&mut self, now: Instant) {
        if let Some(previous) = self.last_redraw_at {
            self.last_redraw_delta_ms =
                Some(now.saturating_duration_since(previous).as_secs_f32() * 1000.0);
        }

        self.last_redraw_at = Some(now);
        self.redraw_total = self.redraw_total.saturating_add(1);
        self.recent_redraws.push_back(now);
        self.prune(now);
    }

    fn prune(&mut self, now: Instant) {
        while matches!(
            self.recent_redraws.front(),
            Some(at) if now.saturating_duration_since(*at) > Duration::from_secs(1)
        ) {
            self.recent_redraws.pop_front();
        }

        self.redraws_last_second = self.recent_redraws.len();
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Unknown,
    UptoDate,
    UpdateAvailable,
}

#[derive(Debug)]
pub struct Snapdash {
    pub config: Config,
    pub theme: ThemeKind,

    pub ha_connected: bool,
    pub ha_connection: Option<HaConnectionConfig>,
    pub ha_token_draft: String,

    pub windows: HashMap<window::Id, WindowState>,
    pub pending_opens: VecDeque<WindowKind>,

    pub theme_options: Vec<ThemeKind>,
    pub status: String,

    pub entities_by_id: HashMap<String, EntityState>,
    pub entity_windows: HashMap<String, window::Id>,
    pub boot_open_done: bool,

    pub settings_sensors: Vec<SettingsSensor>,
    pub selected_widgets: HashSet<String>,
    pub active_settings_sensors: Vec<SettingsSensor>,

    pub entity_search_query: String,
    pub debug_now: Instant,
    pub update_state: UpdateState,
}

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    OpenSettings,
    OpenEntity(String),
    CloseWindow(window::Id),
    QuitApp,
    WindowClosed(window::Id),
    //  WindowOpened(window::Id, WindowKind),
    WindowActuallyOpened(window::Id),

    ThemeSelected(ThemeKind),
    SaveConfig,
    ToggleWidget(String),
    #[cfg(feature = "diagnostics")]
    DebugOverlayToggled(bool),

    // Home Assistant
    ConnectHa,
    HaEvent(HaEvent),
    HaUrlChanged(String),
    HaTokenDelete,

    // UI events
    FocusMove {
        window_id: window::Id,
        direction: FocusDirection,
    },
    SavePressed,
    Saved,
    HaTokenDraftChanged(String),
    WindowRedraw(window::Id, Instant),
    ConfigLoad(Result<Config, String>),
    StartDrag(window::Id),
    EntityHover {
        window: window::Id,
        on: bool,
    },

    EntitySearchChanged(String),
    CheckForUpdate,
    LastVersionChecked(Option<update::GitHubRelease>),
}

impl Snapdash {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            theme: ThemeKind::default(),
            ha_connected: false,
            ha_connection: None,
            ha_token_draft: String::new(),
            status: "-".into(),
            theme_options: vec![ThemeKind::MacLight, ThemeKind::MacDark],
            windows: HashMap::new(),
            pending_opens: VecDeque::new(),
            entities_by_id: HashMap::new(),
            entity_windows: HashMap::new(),
            boot_open_done: false,
            settings_sensors: Vec::new(),
            selected_widgets: HashSet::new(),
            active_settings_sensors: Vec::new(),
            entity_search_query: String::new(),
            debug_now: Instant::now(),
            update_state: UpdateState::Unknown,
        }
    }
    pub fn boot() -> (Self, Task<Message>) {
        let state = Self::new();

        let load_task = Task::perform(
            async { Config::load().await.map_err(|e| e.to_string()) },
            Message::ConfigLoad,
        );

        let check_update = Task::perform(update::get_latest_version(), Message::LastVersionChecked);

        let tasks = Task::batch([load_task, check_update]);

        (state, tasks)
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
            .entities_by_id
            .values()
            .filter(|e| e.entity_id.starts_with("sensor."))
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

    pub fn subscription(&self) -> Subscription<Message> {
        let redraw_events_needed = {
            #[cfg(feature = "diagnostics")]
            {
                self.config.debug_overlay || self.windows.values().any(|win| win.entity.pulse > 0.0)
            }

            #[cfg(not(feature = "diagnostics"))]
            {
                self.windows.values().any(|win| win.entity.pulse > 0.0)
            }
        };

        let redraw_events = if redraw_events_needed {
            iced::event::listen_raw(|event, _status, id| match event {
                iced::Event::Window(window::Event::RedrawRequested(now)) => {
                    Some(Message::WindowRedraw(id, now))
                }
                _ => None,
            })
        } else {
            Subscription::none()
        };

        let ha = if let Some(connection) = &self.ha_connection {
            Subscription::run_with(connection.clone(), crate::ha::ws::connect).map(Message::HaEvent)
        } else {
            Subscription::none()
        };

        let keyboard_events = iced::event::listen_raw(|event, _status, id| {
            let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                modifiers,
                ..
            }) = event
            else {
                return None;
            };

            let direction = if modifiers.shift() {
                FocusDirection::Previous
            } else {
                FocusDirection::Next
            };

            Some(Message::FocusMove {
                window_id: id,
                direction,
            })
        });

        let check_for_update =
            iced::time::every(Duration::from_hours(1)).map(|_| Message::CheckForUpdate);

        Subscription::batch([
            window::close_events().map(Message::WindowClosed),
            redraw_events,
            keyboard_events,
            ha,
            check_for_update,
        ])
    }

    fn handle_redraw_requested(&mut self, id: window::Id, now: Instant) {
        self.debug_now = now;

        let Some(window) = self.windows.get_mut(&id) else {
            return;
        };

        window.debug.record_redraw(now);

        if let WindowKind::Entity { .. } = window.kind
            && window.entity.pulse > 0.0
        {
            window.entity.pulse = (window.entity.pulse - 0.08).max(0.0);
        }
    }

    #[cfg(feature = "diagnostics")]
    fn seed_debug_overlay(&mut self, now: Instant) {
        self.debug_now = now;

        for window in self.windows.values_mut() {
            // Treat enabling diagnostics as the first visible redraw so static
            // windows do not stay stuck at zero until some unrelated repaint.
            window.debug.record_redraw(now);
        }
    }

    #[cfg(feature = "diagnostics")]
    fn seed_debug_window(&mut self, id: window::Id, now: Instant) {
        self.debug_now = now;

        if let Some(window) = self.windows.get_mut(&id) {
            window.debug.record_redraw(now);
        }
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
            window.entity.pulse = 1.0;
        }
    }

    fn apply_initial_states(&mut self, states: Vec<EntityState>) {
        self.entities_by_id.clear();

        for state in states {
            let entity_id = state.entity_id.clone();

            self.set_window_entity_state(&entity_id, &state, false);
            self.entities_by_id.insert(entity_id, state.clone());
        }
        self.rebuild_settings_sensors();
    }

    fn apply_entity_state(&mut self, new_state: EntityState) {
        let entity_id = new_state.entity_id.clone();

        self.set_window_entity_state(&entity_id, &new_state, true);

        let should_refresh_settings = match self.entities_by_id.get(&entity_id) {
            None => true,
            Some(old) => {
                let old_is_sensor = old.entity_id.starts_with("sensor.");
                let new_is_sensor = new_state.entity_id.starts_with("sensor.");

                let old_name = old.attributes.get("friendly_name").and_then(|v| v.as_str());
                let new_name = new_state
                    .attributes
                    .get("friendly_name")
                    .and_then(|v| v.as_str());

                old_is_sensor != new_is_sensor || old_name != new_name
            }
        };

        self.entities_by_id.insert(entity_id, new_state);

        if should_refresh_settings {
            self.rebuild_settings_sensors();
        }
    }

    fn handle_ha_event(&mut self, ev: HaEvent) {
        match ev {
            HaEvent::Connected => {
                self.ha_connected = true;
                self.set_status("HA Connected", LogType::Info);
            }
            HaEvent::Disconnected(why) => {
                self.ha_connected = false;
                self.set_status(format!("HA disconnected: {why}"), LogType::Warn);
            }
            HaEvent::InitialState(states) => {
                self.apply_initial_states(states);
            }
            HaEvent::StateChanged { new_state } => {
                self.apply_entity_state(new_state);
            }
            HaEvent::AuthFailed(why) => {
                self.ha_connected = false;
                self.ha_connection = None;
                self.config.ha_token_present = false;

                self.set_status(format!("Authentication failed: {why}"), LogType::Error);
                let cfg = self.config.clone();
                tokio::spawn(async move { cfg.save_async().await });
            }
        }
    }

    // Set status for status bar and log this message to log.
    pub fn set_status(&mut self, msg: impl Into<String>, error_type: LogType) {
        let msg = msg.into();
        match error_type {
            LogType::Info => log::info(&msg),
            LogType::Warn => log::warn(&msg),
            LogType::Error => log::error(&msg),
            LogType::DoNotLog => (),
        }
        self.status = msg;
    }

    fn is_entity_window_open(&self, entity_id: &str) -> bool {
        self.entity_windows.contains_key(entity_id)
    }

    fn find_window_id(
        windows: &HashMap<window::Id, WindowState>,
        kind: WindowKind,
        name: Option<&str>,
    ) -> Option<window::Id> {
        windows
            .iter()
            .find(|(_, v)| {
                if v.kind != kind {
                    return false;
                }

                match name {
                    None => true,
                    Some(exp) => exp == v.entity.entity_id,
                }
            })
            .map(|(&id, _)| id)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => Task::none(),

            Message::EntitySearchChanged(value) => {
                self.entity_search_query = value;
                Task::none()
            }

            #[cfg(feature = "diagnostics")]
            Message::DebugOverlayToggled(enabled) => {
                self.config.debug_overlay = enabled;
                if enabled {
                    self.seed_debug_overlay(Instant::now());
                }
                let cfg = self.config.clone();

                Task::perform(async move { cfg.save_async().await }, |_| {
                    Message::SaveConfig
                })
            }

            Message::HaTokenDelete => {
                match secrets::delete_ha_token() {
                    Ok(_) => {
                        self.set_status("Token deleted from key-chain", LogType::Info);
                        self.config.ha_token_present = false;
                        self.ha_connection = None;
                        self.ha_connected = false;
                        log::warn("HA disconnected due to erased token.");
                    }
                    Err(s) => {
                        self.set_status(s, LogType::Error);
                    }
                };
                Task::none()
            }

            Message::ConfigLoad(res) => {
                match res {
                    Ok(cfg) => {
                        self.theme = cfg.theme;
                        self.config = cfg;
                        self.rebuild_selected_widgets();
                        self.set_status("Config loaded", LogType::Info);

                        let mut tasks: Vec<Task<Message>> = Vec::new();

                        if !self.boot_open_done {
                            let open_task = if self.config.widgets.is_empty() {
                                Task::perform(async {}, |_| Message::OpenSettings)
                            } else {
                                Task::perform(async {}, |_| Message::OpenEntity("".to_string()))
                            };

                            self.boot_open_done = true;
                            tasks.push(open_task);
                        }

                        let connect_task = Task::perform(async {}, |_| Message::ConnectHa);
                        tasks.push(connect_task);

                        return if tasks.is_empty() {
                            Task::none()
                        } else {
                            Task::batch(tasks)
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
                task.push(Task::perform(async {}, |_| Message::SaveConfig));

                Task::batch(task)
            }

            Message::HaTokenDraftChanged(val) => {
                self.ha_token_draft = val;
                Task::none()
            }

            Message::WindowActuallyOpened(id) => {
                if let Some(kind) = self.pending_opens.pop_front() {
                    let mut entity = EntityWindowState::default();

                    if let WindowKind::Entity { entity_id } = &kind {
                        entity.entity_id = entity_id.clone();

                        if let Some(st) = self.entities_by_id.get(&entity.entity_id) {
                            entity.last = Some(st.clone());
                        }
                        self.entity_windows.insert(entity_id.clone(), id);
                    }
                    self.windows.insert(
                        id,
                        WindowState {
                            kind,
                            entity,
                            debug: WindowDebugState::default(),
                        },
                    );
                    #[cfg(feature = "diagnostics")]
                    if self.config.debug_overlay {
                        self.seed_debug_window(id, Instant::now());
                    }
                } else {
                    self.set_status(
                        "Received WindowActuallyOpened but pending_opens is empty",
                        LogType::Warn,
                    );
                }
                Task::none()
            }

            Message::WindowClosed(id) => {
                if let Some(window) = self.windows.remove(&id)
                    && let WindowKind::Entity { entity_id } = window.kind
                {
                    self.entity_windows.remove(&entity_id);
                }

                if self.windows.is_empty() {
                    iced::exit()
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
                if !self.ha_token_draft.trim().is_empty() {
                    match crate::secrets::set_ha_token(self.ha_token_draft.trim()) {
                        Ok(()) => {
                            self.config.ha_token_present = true;
                            self.ha_token_draft.clear();
                            self.set_status("Token saved into keychain.", LogType::Info);
                        }
                        Err(e) => {
                            self.set_status(format!("Keychain error: {e}"), LogType::Error);
                            return Task::none();
                        }
                    }
                }
                let cfg = self.config.clone();
                Task::perform(async move { cfg.save_async().await }, |_| Message::Saved)
            }

            Message::Saved => {
                self.set_status("Saved", LogType::DoNotLog);
                Task::perform(async {}, |_| Message::ConnectHa)
            }

            Message::OpenSettings => {
                // if Settings window is opened, give focus
                //
                if let Some(settings_id) =
                    Snapdash::find_window_id(&self.windows, WindowKind::Settings, None)
                {
                    return iced::window::gain_focus::<Message>(settings_id);
                }

                self.pending_opens.push_back(WindowKind::Settings);

                // The platform helper adds a transparent shadow margin on
                // Linux (where we render our own shader shadow) and is a
                // no-op on macOS/Windows (where the OS clips + draws its
                // own shadow). See `ui::platform` module doc.
                let settings = window::Settings {
                    size: crate::ui::platform::window_size(820.0, 950.0),
                    resizable: false,
                    decorations: false,
                    transparent: true,
                    ..window::Settings::default()
                };

                let (_id, task_id) = window::open(settings);
                task_id.map(Message::WindowActuallyOpened)
            }

            Message::OpenEntity(entity_id) => {
                // vytvoř nové okno pro entitu
                // let (id, cmd) = window::open(...);
                // self.entity_windows.insert(id, EntityWindowState { ... });
                //

                let widgets: Vec<String> = if entity_id.is_empty() {
                    self.config.widgets.clone()
                } else {
                    vec![entity_id]
                };

                // Platform helper: adds shadow margin on Linux, pass-through
                // on macOS/Windows. See `ui::platform` module doc.
                let win_settings = window::Settings {
                    size: crate::ui::platform::window_size(240.0, 160.0),
                    resizable: false,
                    decorations: false,
                    transparent: true,
                    ..Default::default()
                };

                let mut task = Vec::new();

                for widget in widgets {
                    if self.is_entity_window_open(&widget) {
                        continue;
                    }

                    self.pending_opens
                        .push_back(WindowKind::Entity { entity_id: widget });

                    let (_id, task_id) = window::open(win_settings.clone());
                    task.push(task_id.map(Message::WindowActuallyOpened));
                }

                if task.is_empty() {
                    Task::none()
                } else {
                    Task::batch(task)
                }
            }

            Message::ThemeSelected(t) => {
                self.theme = t;
                self.config.theme = t;
                let cfg = self.config.clone();
                Task::perform(async move { cfg.save_async().await }, |_| {
                    Message::SaveConfig
                })
            }

            Message::SaveConfig => Task::none(),

            Message::ConnectHa => {
                if !self.config.ha_enabled() {
                    self.ha_connected = false;
                    self.ha_connection = None;
                    self.set_status("HA not enabled (missing token/URL)", LogType::Error);
                    return Task::none();
                }

                let token = match crate::secrets::get_ha_token() {
                    Ok(t) => t,
                    Err(e) => {
                        self.ha_connected = false;
                        self.ha_connection = None;
                        self.set_status(format!("Missing token in keychain {e}"), LogType::Error);
                        self.config.ha_token_present = false;

                        let cfg = self.config.clone();
                        return Task::perform(
                            async move {
                                let _ = cfg.save_async().await;
                            },
                            |_| Message::Noop,
                        );
                    }
                };

                let next_connection = HaConnectionConfig {
                    url: self.config.ha_url.clone(),
                    token,
                };

                if self.ha_connection.as_ref() != Some(&next_connection) {
                    self.ha_connected = false;
                    self.set_status("Connecting to HA ...", LogType::Info);
                    self.ha_connection = Some(next_connection);
                }

                Task::none()
            }

            Message::HaEvent(ev) => {
                self.handle_ha_event(ev);
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

            Message::WindowRedraw(id, now) => {
                self.handle_redraw_requested(id, now);
                Task::none()
            }

            Message::CheckForUpdate => {
                Task::perform(update::get_latest_version(), Message::LastVersionChecked)
            }

            Message::LastVersionChecked(release) => {
                if release.is_some() {
                    self.update_state = UpdateState::UpdateAvailable;
                    Task::none()
                } else {
                    self.update_state = UpdateState::UptoDate;
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self, id: window::Id) -> Element<'_, Message> {
        let Some(win) = self.windows.get(&id) else {
            return iced::widget::text("Loading...").into();
        };

        let inner = crate::ui::chrome::window_content(self, win, id);
        let inner = crate::ui::chrome::with_debug_overlay(self, inner, win);

        let inner = match win.kind {
            WindowKind::Entity { .. } => {
                let with_gear = crate::ui::chrome::with_gear_overlay(self, inner, win);
                crate::ui::chrome::with_mouse_area(with_gear, id, win)
            }
            WindowKind::Settings => inner,
        };

        // Platform-specific outer wrapping:
        // - Linux: transparent `SHADOW_MARGIN` padding around the card so
        //   the iced-wgpu shader shadow has room to fade inside the surface.
        // - macOS/Windows: pass-through. The OS hack in iced_winit clips the
        //   window to a rounded shape and the OS draws the drop shadow.
        crate::ui::platform::wrap_outer(inner)
    }

    /// Surface clear color. **Linux-only**: set to `Color::TRANSPARENT` so the
    /// cleared pixels in the shadow margin actually composite as transparent
    /// (the default theme background would show as an opaque color there).
    ///
    /// On macOS/Windows we intentionally do NOT install this callback (see
    /// `main.rs`) — the OS hack clips the window to a rounded shape before
    /// any cleared pixel is visible, so the default opaque theme background
    /// is never seen and matches the pre-shadow-margin behavior users
    /// reported looked "nice and optimal".
    #[cfg(target_os = "linux")]
    pub fn style(&self, _theme: &iced::Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: self.theme.palette().text_primary,
        }
    }
}
