use crate::app::Message;
use crate::helpers::humanize_bytes;
use crate::ui::components;

use iced::Element;

use iced::widget::row;

use crate::app::Snapdash;
use crate::theme::Palette;
use crate::ui::components::error_message;
use crate::ui::components::settings_components;

pub fn view<'a>(snap: &'a Snapdash, p: Palette) -> Element<'a, Message> {
    if let Some(info) = &snap.sys_info {
        settings_components::page_with_scrollable_sections(
            "System information",
            [
                settings_components::section(
                    [settings_components::item_with_status(
                        "Hostname",
                        None,
                        info.host_name.clone().unwrap_or("?".into()),
                        p,
                    )],
                    p,
                ),
                settings_components::section(
                    [
                        settings_components::item_with_status(
                            "Operating system",
                            None,
                            info.system_name.as_deref().unwrap_or("Unknown"),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Version",
                            None,
                            info.system_version.as_deref().unwrap_or("Unknown"),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Kernel",
                            None,
                            info.kernel_version.as_deref().unwrap_or("Unknown"),
                            p,
                        ),
                    ],
                    p,
                ),
                // CPU section
                settings_components::section(
                    [
                        settings_components::item_with_status(
                            "CPU Brand",
                            None,
                            info.cpu_brand.as_str(),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Cores",
                            Some("Physical / Logical / load %"),
                            format!(
                                "{} / {} / {:.1}%",
                                info.cpu_cores_physical.unwrap_or(0),
                                info.cpu_cores_logical,
                                info.cpu_usage
                            ),
                            p,
                        ),
                    ],
                    p,
                ),
                // Memory section
                settings_components::section(
                    [
                        settings_components::item_with_status(
                            "Memory total",
                            None,
                            humanize_bytes(info.memory_total),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Memory available",
                            None,
                            humanize_bytes(info.memory_available),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Memory used",
                            None,
                            humanize_bytes(info.memory_used),
                            p,
                        ),
                    ],
                    p,
                ),
                //Graphics adapter
                settings_components::section(
                    [
                        settings_components::item_with_status(
                            "Graphics adapter",
                            None,
                            info.graphics_adapter.as_str(),
                            p,
                        ),
                        settings_components::item_with_status(
                            "Graphics backend",
                            None,
                            info.graphics_backend.as_str(),
                            p,
                        ),
                    ],
                    p,
                ),
            ],
            [row![
                components::primary_button("Copy as Markdown", p, Some(Message::CopySystemInfoMd)),
                components::pill_button("Copy as simple text", p, Some(Message::CopySystemInfo)),
                components::pill_button("Refresh", p, Some(Message::RefresSystemInfo)),
            ]
            .into()],
            p,
        )
    } else {
        error_message("OS info is loading ...", p)
    }
}
