use serde::Deserialize;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    #[serde(default)]
    pub body: String,

    #[serde(default)]
    pub html_url: String, //intentionaly left here for future useage.
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UpdateState {
    #[default]
    Unknown,
    UptoDate,
    UpdateAvailable,
}

#[derive(Debug, Default)]
pub struct UpdateStatus {
    pub state: UpdateState,
    pub latest_release: Option<GitHubRelease>,
    pub release_notes_items: Vec<iced::widget::markdown::Item>,
}

impl UpdateStatus {
    pub fn is_available(&self) -> bool {
        self.state == UpdateState::UpdateAvailable
    }

    pub fn record_check(&mut self, release: Option<GitHubRelease>) {
        match release {
            Some(release) => {
                self.state = UpdateState::UpdateAvailable;
                self.release_notes_items =
                    iced::widget::markdown::parse(&release.body).collect();
                self.latest_release = Some(release);
            }
            None => {
                self.state = UpdateState::UptoDate;
                self.latest_release = None;
                self.release_notes_items.clear();
            }
        }
    }
}

pub async fn get_latest_version() -> Option<GitHubRelease> {
    let client = reqwest::Client::new();

    let release: GitHubRelease = client
        .get("https://api.github.com/repos/schizza/snapdash/releases/latest")
        .header("User-Agent", "snapdash")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    let latest = release.tag_name.trim_start_matches('v');

    if latest != CURRENT_VERSION {
        Some(release)
    } else {
        None
    }
}
