use crate::logger as log;
use crate::logger::LogType;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use iced::window;
use iced::{Element, Task};

use crate::config::Config;
use crate::ha::{EntityState, HaEvent};
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
}

#[derive(Debug, Default, Clone)]
pub struct EntityWindowState {
    pub entity_id: String,
    pub selected_fields: std::collections::BTreeSet<String>,
    pub last: Option<EntityState>,
    pub pulse: f32, // TODO: Replace with Animation/spring. Now just easy "animation paramter" (0..1), později nahradit Animation/spring
    pub hovered: bool,
}

#[derive(Debug)]
pub struct Snapdash {
    pub config: Config,
    pub theme: ThemeKind,

    pub ha_connected: bool,
    pub ha_rx: Option<tokio::sync::mpsc::UnboundedReceiver<HaEvent>>,
    pub ha_token_draft: String,

    pub windows: HashMap<window::Id, WindowState>,
    pub pending_opens: VecDeque<WindowKind>,

    pub theme_options: Vec<ThemeKind>,
    pub status: String,

    pub entities: Vec<EntityState>,
    pub boot_open_done: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    OpenSettings,
    OpenEntity(String),
    WindowClosed(window::Id),
    //  WindowOpened(window::Id, WindowKind),
    WindowActuallyOpened(window::Id),

    ThemeSelected(ThemeKind),
    SaveConfig,
    ToggleWidget(String),

    // Home Assistant
    ConnectHa,
    HaEvent(HaEvent),
    HaInitialStates(Vec<EntityState>),
    HaUrlChanged(String),
    HaTokenChanged(String),
    HaTokenDelete,

    // UI events
    ToggleField { window: window::Id, field: String },
    SavePressed,
    Saved,
    HaTokenDraftChanged(String),
    ClearStatus,
    OpenEntityPressed,
    Tick(Instant),
    ConfigLoad(Result<Config, String>),
    StartDrag(window::Id),
    EntityHover { window: window::Id, on: bool },
}

impl Snapdash {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            theme: ThemeKind::default(),
            ha_connected: false,
            ha_rx: None,
            ha_token_draft: String::new(),
            status: "-".into(),
            theme_options: vec![ThemeKind::MacLight, ThemeKind::MacDark],
            windows: HashMap::new(),
            pending_opens: VecDeque::new(),
            entities: Vec::new(),
            boot_open_done: false,
        }
    }
    pub fn boot() -> (Self, Task<Message>) {
        let state = Self::new();

        let load_task = Task::perform(
            async { Config::load().await.map_err(|e| e.to_string()) },
            Message::ConfigLoad,
        );

        (state, load_task)
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

    fn apply_ha_event(&mut self, ev: HaEvent) {
        match ev {
            HaEvent::Connected => {
                self.ha_connected = true;
                self.set_status("HA connected", LogType::Info);
            }
            HaEvent::Disconnected(why) => {
                self.ha_connected = false;
                self.set_status(format!("HA disconnected: {why}"), LogType::Error);
            }
            HaEvent::StateChanged { new_state } => {
                let id = new_state.entity_id.clone();
                for win in self.windows.values_mut() {
                    if let WindowKind::Entity { entity_id } = &win.kind {
                        if entity_id == &id {
                            win.entity.last = Some(new_state.clone());
                            win.entity.pulse = 1.0;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn is_entity_window_open(&self, entity_id: &str) -> bool {
        self.windows
            .values()
            .any(|w| matches!(&w.kind, WindowKind::Entity { entity_id: eid } if eid == entity_id))
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

            Message::HaTokenDelete => {
                match secrets::delete_ha_token() {
                    Ok(_) => {
                        self.set_status("Token deleted from key-chain", LogType::Info);
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

            Message::StartDrag(id) => {
                return iced::window::drag(id);
            }

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
                    let to_close: Vec<_> = self
                        .windows
                        .iter()
                        .filter_map(|(id, win)| match &win.kind {
                            WindowKind::Entity { entity_id: e } if e == &entity_id => Some(*id),
                            _ => None,
                        })
                        .collect();

                    for id in to_close {
                        self.windows.remove(&id);
                        task.push(iced::window::close(id));
                    }
                } else {
                    // open widget
                    self.config.widgets.push(entity_id.clone());
                }
                task.push(Task::perform(async {}, |_| Message::SaveConfig));
                task.push(Task::perform(async {}, |_| {
                    Message::OpenEntity("".to_string())
                }));

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

                        // get initial state for widget
                        if let Some(st) = self
                            .entities
                            .iter()
                            .find(|st| st.entity_id == entity.entity_id)
                        {
                            entity.last = Some(st.clone());
                        }
                    }
                    self.windows.insert(id, WindowState { kind, entity });
                } else {
                    self.set_status(
                        "Recieved WindowActuallyOpened but pending_opens is empty",
                        LogType::Warn,
                    );
                }
                Task::none()
            }

            // Message::WindowOpened(id, kind) => {
            //     let mut entity = EntityWindowState::default();

            //     if let WindowKind::Entity { entity_id } = &kind {
            //         entity.entity_id = entity_id.clone();
            //     }

            //     self.windows.insert(id, WindowState { kind, entity });
            //     Task::none()
            // }
            //
            Message::WindowClosed(id) => {
                self.windows.remove(&id);

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
            Message::HaTokenChanged(val) => Task::none(),

            Message::SavePressed => {
                self.set_status("Saving...", LogType::DoNotLog);
                if !self.ha_token_draft.trim().is_empty() {
                    match crate::secrets::set_ha_token(&self.ha_token_draft.trim()) {
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

            Message::ClearStatus => {
                // self.status.clear();
                Task::none()
            }

            Message::OpenEntityPressed => {
                // TODO
                self.set_status("TODO: Open entity window", LogType::Warn);
                Task::none()
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

                let settings = window::Settings {
                    size: iced::Size::new(720.0, 650.0),
                    resizable: false,
                    decorations: true,
                    ..window::Settings::default()
                };

                let (_id, task_id) = window::open(settings);
                task_id.map(|actual_id| Message::WindowActuallyOpened(actual_id))
            }

            Message::OpenEntity(entity_id) => {
                // vytvoř nové okno pro entitu
                // let (id, cmd) = window::open(...);
                // self.entity_windows.insert(id, EntityWindowState { ... });

                let win_settings = window::Settings {
                    size: iced::Size::new(240.0, 160.0),
                    resizable: false,
                    decorations: false,
                    transparent: true,
                    ..Default::default()
                };

                // pokud už jsou pro všechny widgety okna otevřená, nedělej nic
                let all_open = self
                    .config
                    .widgets
                    .iter()
                    .all(|w| self.is_entity_window_open(w));

                if all_open {
                    return Task::none();
                }

                let mut task = Vec::new();

                for widget in self.config.widgets.iter().cloned() {
                    if self.is_entity_window_open(&widget) {
                        continue;
                    }

                    self.pending_opens
                        .push_back(WindowKind::Entity { entity_id: widget });

                    let (_id, task_id) = window::open(win_settings.clone());
                    task.push(
                        task_id.map(move |actual_id| Message::WindowActuallyOpened(actual_id)),
                    );
                }

                if task.is_empty() {
                    Task::none()
                } else {
                    Task::batch(task)
                }
            }

            // Message::CloseWindow(id) => {
            //     self.entity_windows.remove(&id);
            //     if self.settings_window == Some(id) {
            //         self.settings_window = None;
            //     }
            //     window::close(id)
            // }
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
                // spawn REST init + WS stream
                // let url = self.config.ha_url.clone();
                // let token = self.config.ha_token.clone();
                // Task::batch(vec![
                //     Task::perform(
                //         crate::ha::rest::fetch_all_states(url.clone(), token.clone()),
                //         Message::HaInitialStates,
                //     ),
                //     Task::perform(crate::ha::ws::run_event_loop(url, token), Message::HaEvent),
                // ])
                //
                if self.ha_rx.is_some() {
                    self.set_status("HA already connected", LogType::Warn);
                    return Task::none();
                }

                if !self.config.ha_enabled() {
                    self.set_status("HA not enabled (missing token/URL)", LogType::Error);
                    return Task::none();
                }

                let token = match crate::secrets::get_ha_token() {
                    Ok(t) => t,
                    Err(e) => {
                        self.set_status(format!("Missing token in keychain: {e}"), LogType::Error);
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

                let url = self.config.ha_url.clone();

                let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<HaEvent>();
                self.ha_rx = Some(rx);

                let rest = Task::perform(
                    crate::ha::rest::fetch_all_states(url.clone(), token.clone()),
                    Message::HaInitialStates,
                );

                tokio::spawn(async move {
                    crate::ha::ws::run_forever(url, token, tx).await;
                });

                rest
            }

            Message::HaInitialStates(states) => {
                self.entities.clear();
                for st in states {
                    for win in self.windows.values_mut() {
                        if let WindowKind::Entity { .. } = &win.kind
                            && win.entity.entity_id == st.entity_id
                        {
                            win.entity.last = Some(st.clone())
                        }
                    }
                    self.entities.push(st)
                }
                Task::none()
            }

            Message::HaEvent(ev) => {
                match ev {
                    HaEvent::Connected => {
                        self.ha_connected = true;
                        self.set_status("HA connected", LogType::Info);
                    }

                    HaEvent::Disconnected(_why) => {
                        self.ha_connected = false;
                        self.set_status(_why, LogType::Warn);
                    }

                    HaEvent::StateChanged { new_state } => {
                        let id = new_state.entity_id.clone();
                        let mut matched = 0usize;
                        // let mut lines: Vec<String> = Vec::new();

                        // lines.push(format!("ID {}  state: {:?}", id.clone(), new_state.clone()));

                        for (win_id, win) in self.windows.iter_mut() {
                            if let WindowKind::Entity { entity_id } = &win.kind {
                                if entity_id == &id {
                                    win.entity.last = Some(new_state.clone());
                                    win.entity.pulse = 1.0;

                                    // lines.push(format!("last value {:?}", win.entity.last));
                                    matched += 1;
                                }
                            }
                        }

                        // if let Some((win_id, win)) = self.windows.iter_mut().find(|(_, w)| {
                        //     matches!(&w.kind, WindowKind::Entity { .. })
                        //         && &w.entity.entity_id == &id
                        // }) {
                        //     win.entity.last = Some(new_state.clone());
                        //     win.entity.pulse = 1.0;
                        // }
                        //

                        // if matched == 0 {
                        //     for (win_id, win) in self.windows.iter_mut() {
                        //         if let WindowKind::Entity { entity_id } = &win.kind {
                        //             lines.push(format!(
                        //                 "Open entity window: {:?} => {}",
                        //                 win_id, entity_id
                        //             ))
                        //         }
                        //     }
                        // } else {
                        //     self.set_status(format!("Match for {}", id));
                        // }

                        // for line in lines {
                        //     self.set_status(line.clone());
                        // }
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::ToggleField { window, field } => {
                // if let Some(w) = self.windows.get_mut(&window) {
                //     if !w.selected_fields.insert(field.clone()) {
                //         w.selected_fields.remove(&field);
                //     }
                // }
                Task::none()
            }

            Message::Tick(_now) => {
                let mut events = Vec::new();

                if let Some(rx) = self.ha_rx.as_mut() {
                    while let Ok(ev) = rx.try_recv() {
                        events.push(ev);
                    }
                }

                for ev in events {
                    self.apply_ha_event(ev);
                }

                for win in self.windows.values_mut() {
                    if let WindowKind::Entity { .. } = win.kind
                        && win.entity.pulse > 0.0
                    {
                        win.entity.pulse = (win.entity.pulse - 0.08).max(0.0);
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

        let inner = crate::ui::chrome::window_content(self, win);

        match win.kind {
            WindowKind::Entity { .. } => {
                let with_gear = crate::ui::chrome::with_gear_overlay(self, inner, win);
                crate::ui::chrome::with_mouse_area(with_gear, id, win)
            }
            WindowKind::Settings => inner,
        }
    }
}
