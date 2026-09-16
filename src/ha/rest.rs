use crate::ha::HaConnectionConfig;
use crate::ha::types::EntityState;

pub async fn fetch_all_states(connection: &HaConnectionConfig) -> Vec<EntityState> {
    let base = connection.url.trim_end_matches('/');

    // A client that cannot be built means the user's TLS settings are
    // broken, and the WS connect that precedes this call has already
    // surfaced that. Here it only degrades to "no initial states".
    let client = match crate::ha::tls::http_client(&connection.tls) {
        Ok(client) => client,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), "cannot build HTTP client");
            return vec![];
        }
    };

    let resp = client
        .get(format!("{base}/api/states"))
        .bearer_auth(&connection.token)
        .send()
        .await;

    let Ok(resp) = resp else { return vec![] };
    if !resp.status().is_success() {
        return vec![];
    }

    resp.json::<Vec<EntityState>>().await.unwrap_or_default()
}
