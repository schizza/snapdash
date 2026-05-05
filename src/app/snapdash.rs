use crate::ha::types::HaError;
use crate::ha::{EntityState, HaConnectionConfig, HaEvent};
use crate::logger::LogType;
use crate::ui::platform::window_settings;
use crate::ui::settings::*;
use crate::update;
use crate::widget_size::WidgetSize;
use crate::{ha, logger};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use iced::window;
use iced::{Element, Task};

use crate::config::{Config, WidgetPosition};
use crate::ha::token::{self, TokenPresence};
use crate::theme::ThemeKind;

use super::window::{EntityWindowState, WindowKind, WindowState, find_window_id};

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

#[derive(Debug)]
pub struct Snapdash {
    pub config: Config,
    pub token_presence: TokenPresence,
    pub theme: ThemeKind,

    pub ha: ha::HaState,

    pub windows: HashMap<window::Id, WindowState>,

    pub theme_options: Vec<ThemeKind>,
    pub status: String,

    pub entity_windows: HashMap<String, window::Id>,
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
}

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    OpenSettings,
    OpenSettingsTo(SettingsPage),
    OpenEntity(String),
    OpenReleaseNotes,
    OpenUrl(String),
    CloseWindow(window::Id),
    QuitApp,
    WindowClosed(window::Id),
    //  WindowOpened(window::Id, WindowKind),
    WindowOpened {
        id: window::Id,
        kind: WindowKind,
    },
    AnimationFrame(iced::time::Instant),

    ThemeSelected(ThemeKind),
    SaveConfig,
    ToggleWidget(String),

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
}

impl Default for Snapdash {
    fn default() -> Self {
        Self::new()
    }
}

impl Snapdash {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            token_presence: TokenPresence::Unchecked,
            theme: ThemeKind::default(),
            ha: ha::HaState::default(),
            status: "-".into(),
            theme_options: vec![ThemeKind::MacLight, ThemeKind::MacDark],
            windows: HashMap::new(),
            entity_windows: HashMap::new(),
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
        }
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

        let (pulse, should_refresh_settings) = match self.ha.entities.get(&entity_id) {
            None => (true, true),
            Some(old) => {
                let old_is_sensor = old.entity_id.starts_with("sensor.");
                let new_is_sensor = new_state.entity_id.starts_with("sensor.");

                let old_name = old.attributes.get("friendly_name").and_then(|v| v.as_str());
                let new_name = new_state
                    .attributes
                    .get("friendly_name")
                    .and_then(|v| v.as_str());

                (
                    self.should_pulse(Some(old), &new_state),
                    old_is_sensor != new_is_sensor || old_name != new_name,
                )
            }
        };

        self.set_window_entity_state(&entity_id, &new_state, pulse);
        self.ha.entities.insert(entity_id, new_state);

        if should_refresh_settings {
            self.rebuild_settings_sensors();
        }
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
                Task::none()
            }
            HaEvent::StateChanged { new_state } => {
                self.apply_entity_state(new_state);
                Task::none()
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
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => Task::none(),

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

            Message::WidgetSizeChanged(size) => {
                self.config.widget_settings.widget_size = size;

                // Route the card size through the platform helper so Linux gets its
                // SHADOW_MARGIN inflation (composited.rs:28) — same path as
                // window_settings() uses at window creation time. Without this,
                // resizing on Linux drops the shadow margin and clips drawn shadows.
                let card = size.window_size();
                let surface = crate::ui::platform::window_size(card.width, card.height);

                let mut tasks: Vec<Task<Message>> = self
                    .windows
                    .iter()
                    .filter(|(_id, win)| matches!(win.kind, WindowKind::Entity { .. }))
                    .map(|(id, _win)| iced::window::resize::<Message>(*id, surface))
                    .collect();

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
                self.settings_page = page;
                Task::none()
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

                        self.theme = cfg.theme;
                        self.config = cfg;
                        crate::autostart::validate_state(self.config.autostart);

                        tasks.push(Task::perform(
                            async {
                                tokio::task::spawn_blocking(token::presence)
                                    .await
                                    .unwrap_or_else(|e| TokenPresence::AccessFailed(e.to_string()))
                            },
                            Message::HaTokenPresenceChecked,
                        ));

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
                task.push(self.save_config());

                Task::batch(task)
            }

            Message::HaTokenDraftChanged(val) => {
                self.ha.token_draft = val;
                Task::none()
            }

            Message::WindowOpened { id, kind } => {
                let mut entity = EntityWindowState::default();

                if let WindowKind::Entity { entity_id } = &kind {
                    entity.entity_id = entity_id.clone();

                    if let Some(st) = self.ha.entities.get(&entity.entity_id) {
                        entity.last = Some(st.clone());
                    }

                    self.entity_windows.insert(entity_id.clone(), id);
                }

                self.windows.insert(id, WindowState { kind, entity });

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

                // The platform helper adds a transparent shadow margin on
                // Linux (where we render our own shader shadow) and is a
                // no-op on macOS/Windows (where the OS clips + draws its
                // own shadow). See `ui::platform` module doc.
                let settings = window_settings(iced::Size::new(920.0, 640.0), true);
                let (id, task_id) = window::open(settings);
                task_id.map(move |_| Message::WindowOpened {
                    id,
                    kind: WindowKind::Settings,
                })
            }

            Message::OpenEntity(entity_id) => {
                let widgets: Vec<String> = if entity_id.is_empty() {
                    self.config.widgets.clone()
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

                    let mut win_settings = window_settings(
                        self.config.widget_settings.widget_size.window_size(),
                        false,
                    );
                    if let Some(saved) = self.config.widget_positions.get(&widget) {
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

            Message::ThemeSelected(t) => {
                self.theme = t;
                self.config.theme = t;

                self.save_config()
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
            Message::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    tracing::warn!(url, error = %e, "failed to open URL");
                }
                Task::none()
            }

            Message::WidgetMoved { id, position } => {
                let Some(window) = self.windows.get(&id) else {
                    return Task::none();
                };
                let WindowKind::Entity { entity_id } = &window.kind else {
                    return Task::none();
                };

                let entity_id = entity_id.clone();
                let new_position = WidgetPosition {
                    x: position.x,
                    y: position.y,
                };

                // Filter: if position is not moved - do nothing. Without filter we will fire
                // debounce timer with every programatic move (ex. window manager snap-to-grid on borders).

                if self.config.widget_positions.get(&entity_id) == Some(&new_position) {
                    return Task::none();
                }

                self.config.widget_positions.insert(entity_id, new_position);
                self.last_widget_move_at = Some(std::time::Instant::now());

                Task::perform(
                    async {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    },
                    |_| Message::PersistWidgetPositions,
                )
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
            text_color: self.theme.palette().text_primary,
        }
    }
}
