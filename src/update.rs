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
