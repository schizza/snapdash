use crate::ha::types::EntityState;

pub async fn fetch_all_states(ha_url: &str, token: &str) -> Vec<EntityState> {
    let base = ha_url.trim_end_matches('/');

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/states"))
        .bearer_auth(token)
        .send()
        .await;

    let Ok(resp) = resp else { return vec![] };
    if !resp.status().is_success() {
        return vec![];
    }

    resp.json::<Vec<EntityState>>().await.unwrap_or_default()
}
