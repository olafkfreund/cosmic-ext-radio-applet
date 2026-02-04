use reqwest::Error;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Station {
    #[serde(default)]
    pub stationuuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub url_resolved: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub favicon: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub language: String,
}

/// Intermediate struct to handle null values from API JSON
#[derive(Deserialize)]
struct ApiStation {
    #[serde(default)]
    stationuuid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    url_resolved: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    favicon: Option<String>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    language: Option<String>,
}

impl From<ApiStation> for Station {
    fn from(api: ApiStation) -> Self {
        Self {
            stationuuid: api.stationuuid.unwrap_or_default(),
            name: api.name.unwrap_or_default(),
            url: api.url.unwrap_or_default(),
            url_resolved: api.url_resolved.unwrap_or_default(),
            homepage: api.homepage.unwrap_or_default(),
            favicon: api.favicon.unwrap_or_default(),
            tags: api.tags.unwrap_or_default(),
            country: api.country.unwrap_or_default(),
            language: api.language.unwrap_or_default(),
        }
    }
}

/// Mirror servers for radio-browser.info API redundancy
const API_SERVERS: &[&str] = &[
    "https://all.api.radio-browser.info",
    "https://de1.api.radio-browser.info",
    "https://fr1.api.radio-browser.info",
    "https://at1.api.radio-browser.info",
    "https://nl1.api.radio-browser.info",
    "https://us1.api.radio-browser.info",
    "https://es1.api.radio-browser.info",
];

/// Search for radio stations by name
pub async fn search_stations(query: String) -> Result<Vec<Station>, Error> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    debug!("Searching stations for '{}'", query);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut last_error: Option<Error> = None;

    for server in API_SERVERS {
        let url = format!("{}/json/stations/search", server);
        let params = [("name", query.as_str()), ("limit", "20")];

        match client.get(&url).query(&params).send().await {
            Ok(response) => match response.error_for_status() {
                Ok(valid_response) => match valid_response.json::<Vec<ApiStation>>().await {
                    Ok(api_stations) => {
                        debug!("Found {} stations from {}", api_stations.len(), server);
                        return Ok(api_stations.into_iter().map(Station::from).collect());
                    }
                    Err(e) => {
                        warn!("JSON parse error from {}: {}", server, e);
                        last_error = Some(e);
                    }
                },
                Err(e) => {
                    warn!("HTTP error from {}: {}", server, e);
                    last_error = Some(e);
                }
            },
            Err(e) => {
                warn!("Connection error to {}: {}", server, e);
                last_error = Some(e);
            }
        }
    }

    // All servers failed - return the last error or empty result
    match last_error {
        Some(e) => Err(e),
        None => Ok(Vec::new()),
    }
}
