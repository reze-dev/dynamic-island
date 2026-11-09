//! Wallhaven remote source.
//!
//! The official Wallhaven API needs no key for SFW content and returns a
//! direct image URL, so this source can `Direct`-fetch without scraping.
//! Rate limit is 45 req/min unauthenticated: the client debounces searches
//! client-side (250ms) and the caller never polls.
//!
//! Quickshell's QML engine cannot link QtNetwork, so the UI cannot load
//! remote images directly. The server therefore downloads thumbnails to a
//! local cache and the panel shows `file://` images from there. Full-size
//! images are downloaded on demand into the wallpaper catalog.

use serde::{Deserialize, Serialize};

pub const API_BASE: &str = "https://wallhaven.cc/api/v1";
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
pub const REQUEST_TIMEOUT_SECS: u64 = 15;

/// Hosts a Wallhaven fetch may reach. Anything else is rejected.
const ALLOWED_HOSTS: &[&str] = &["wallhaven.cc", "w.wallhaven.cc", "th.wallhaven.cc"];

/// Map a tier name to the API's `purity` bits: SFW is `100`, plus-sketchy
/// is `110`, everything is `111`. Unknown names are invalid.
pub fn purity_bits(tier: &str) -> Option<&'static str> {
    match tier {
        "sfw" => Some("100"),
        "sketchy" => Some("110"),
        "nsfw" => Some("111"),
        _ => None,
    }
}

/// Build a `/search` URL. The API key is appended only when present:
/// unauthenticated requests work for SFW and are refused (403) otherwise.
pub fn build_search_url(
    query: &str,
    page: u32,
    purity: &str,
    api_key: &str,
) -> Result<url::Url, String> {
    let mut url = url::Url::parse(&format!("{API_BASE}/search")).map_err(|e| format!("{e}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        if !query.trim().is_empty() {
            pairs.append_pair("q", query.trim());
        }
        pairs.append_pair("categories", "100");
        pairs.append_pair("purity", purity);
        pairs.append_pair("sorting", "toplist");
        pairs.append_pair("atleast", "1920x1080");
        pairs.append_pair("page", &page.to_string());
        if !api_key.trim().is_empty() {
            pairs.append_pair("apikey", api_key.trim());
        }
    }
    Ok(url)
}

/// Reject URLs that do not match the Wallhaven allowlist.
pub fn bounded_url(raw: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(raw).map_err(|e| format!("bad url: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("non-https url".to_string());
    }
    let host = parsed.host_str().unwrap_or("");
    if !ALLOWED_HOSTS.contains(&host) {
        return Err(format!("host not allowlisted: {host}"));
    }
    Ok(parsed)
}

fn api_error(context: &str, status: reqwest::StatusCode, body: &str) -> String {
    let preview = body.chars().take(160).collect::<String>();
    format!("wallhaven {context}: http {}: {preview}", status.as_u16())
}

/// One result from a search. Field set chosen for the panel: identity and
/// attribution, the direct image path, a cheap thumbnail, and the free
/// palette the API hands back *before* download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallhavenItem {
    /// Short catalog id, e.g. "jel1jq".
    pub id: String,
    /// Page URL for attribution.
    pub url: String,
    /// Direct full-image URL.
    pub path: String,
    pub resolution: String,
    pub file_size: String,
    pub file_type: String,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub thumbs: Thumbs,
    #[serde(default)]
    pub source: String,
    /// Local cache file holding the small thumbnail, if it has been
    /// downloaded. The panel paints this so the grid works offline.
    #[serde(default)]
    pub thumb_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbs {
    #[serde(default)]
    pub large: String,
    #[serde(default)]
    pub small: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<ApiItem>,
    #[serde(default)]
    meta: Option<SearchMeta>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SearchMeta {
    #[serde(default)]
    current_page: u32,
    #[serde(default)]
    last_page: u32,
}

fn flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    })
}

#[derive(Debug, Clone, Deserialize)]
struct ApiItem {
    #[serde(default)]
    id: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    resolution: String,
    #[serde(default, deserialize_with = "flexible_string")]
    file_size: String,
    #[serde(default)]
    file_type: String,
    #[serde(default)]
    colors: Vec<String>,
    #[serde(default)]
    thumbs: ApiThumbs,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ApiThumbs {
    #[serde(default)]
    large: String,
    #[serde(default)]
    small: String,
}

impl From<ApiItem> for WallhavenItem {
    fn from(item: ApiItem) -> Self {
        Self {
            id: item.id.clone(),
            url: item.url.clone(),
            path: item.path.clone(),
            resolution: item.resolution.clone(),
            file_size: item.file_size.clone(),
            file_type: item.file_type.clone(),
            colors: item.colors.clone(),
            thumbs: Thumbs {
                large: item.thumbs.large.clone(),
                small: item.thumbs.small.clone(),
            },
            source: item.source.clone(),
            thumb_path: None,
        }
    }
}

/// Search page: items plus whether a next page exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchPage {
    pub items: Vec<WallhavenItem>,
    pub page: u32,
    pub last_page: u32,
}

/// The thin API client. Cheap to construct per call; the search cache lives
/// in the server and is keyed on (query, page).
pub struct WallhavenClient {
    client: reqwest::Client,
}

impl Default for WallhavenClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WallhavenClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("isle/1.0 (dynamic-island; wallpaper browser)")
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client is always buildable");
        Self { client }
    }

    /// Search at a purity tier (`purity_bits` output) with an optional key.
    /// SFW needs no key; sketchy/nsfw need one (the server refuses them
    /// without one before this lands here).
    pub async fn search(
        &self,
        query: &str,
        page: u32,
        purity: &str,
        api_key: &str,
    ) -> Result<SearchPage, String> {
        let url = build_search_url(query, page, purity, api_key)?;
        let resp = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|e| format!("wallhaven search failed: {e}"))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("wallhaven body: {e}"))?;
        if !status.is_success() {
            return Err(api_error("search", status, &body));
        }
        let parsed: SearchResponse =
            serde_json::from_str(&body).map_err(|e| format!("wallhaven parse: {e}"))?;
        let meta = parsed.meta.unwrap_or(SearchMeta {
            current_page: page,
            last_page: 1,
        });
        Ok(SearchPage {
            items: parsed.data.into_iter().map(WallhavenItem::from).collect(),
            page: meta.current_page,
            last_page: meta.last_page,
        })
    }

    /// Fetch image bytes, enforcing BoundedUrl, MAX_BYTES and magic check.
    pub async fn fetch_image(&self, url_str: &str) -> Result<(Vec<u8>, String), String> {
        let url = bounded_url(url_str)?;
        let resp = self
            .client
            .get(url)
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|e| format!("wallhaven fetch failed: {e}"))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("wallhaven fetch: http {}", status.as_u16()));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("wallhaven download: {e}"))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(format!("download exceeds {MAX_BYTES} bytes"));
        }
        let mime = sniff_magic(&bytes);
        Ok((bytes.to_vec(), mime))
    }
}

/// Sniff the image type from magic bytes; never trust the filename.
pub fn sniff_magic(bytes: &[u8]) -> String {
    let header: Vec<u8> = bytes.iter().take(12).cloned().collect();
    if header.len() >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF {
        return "image/jpeg".into();
    }
    if header.len() >= 8 && &header[0..8] == b"\x89PNG\r\n\x1a\n" {
        return "image/png".into();
    }
    if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WEBP" {
        return "image/webp".into();
    }
    if header.len() >= 6 && (&header[0..6] == b"GIF87a" || &header[0..6] == b"GIF89a") {
        return "image/gif".into();
    }
    if header.len() >= 12 && &header[4..8] == b"ftyp" {
        return "video/mp4".into();
    }
    "application/octet-stream".into()
}

/// Whether a fetched payload is a renderable image (a video is not).
pub fn is_renderable_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/jpeg" | "image/png" | "image/webp" | "image/gif"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_wallhaven_hosts_are_allowlisted() {
        assert!(bounded_url("https://wallhaven.cc/api/v1/search").is_ok());
        assert!(bounded_url("https://w.wallhaven.cc/full/test.jpg").is_ok());
        assert!(bounded_url("http://wallhaven.cc/x").is_err());
        assert!(bounded_url("https://evil.example/x.jpg").is_err());
    }

    #[test]
    fn magic_bytes_identify_common_images() {
        assert_eq!(
            sniff_magic(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]),
            "image/jpeg"
        );
        assert_eq!(sniff_magic(b"\x89PNG\r\n\x1a\nrest"), "image/png");
        assert_eq!(sniff_magic(b"RIFF\x00\x00\x00\x00WEBP"), "image/webp");
        assert_eq!(sniff_magic(b"GIF89a123"), "image/gif");
    }

    #[test]
    fn a_video_is_not_renderable() {
        assert!(!is_renderable_mime("video/mp4"));
        assert!(is_renderable_mime("image/jpeg"));
    }

    #[test]
    fn purity_tiers_map_to_bits() {
        assert_eq!(purity_bits("sfw"), Some("100"));
        assert_eq!(purity_bits("sketchy"), Some("110"));
        assert_eq!(purity_bits("nsfw"), Some("111"));
        assert_eq!(purity_bits("everything"), None);
        assert_eq!(purity_bits(""), None);
    }

    #[test]
    fn search_urls_carry_tier_and_key() {
        let sfw = build_search_url("", 1, "100", "").unwrap();
        assert!(sfw.query_pairs().any(|(k, v)| k == "purity" && v == "100"));
        assert!(!sfw.query_pairs().any(|(k, _)| k == "apikey"));

        let keyed = build_search_url("storm", 2, "111", "my-key").unwrap();
        assert!(keyed.query_pairs().any(|(k, v)| k == "q" && v == "storm"));
        assert!(
            keyed
                .query_pairs()
                .any(|(k, v)| k == "purity" && v == "111")
        );
        assert!(
            keyed
                .query_pairs()
                .any(|(k, v)| k == "apikey" && v == "my-key")
        );
        assert!(keyed.query_pairs().any(|(k, v)| k == "page" && v == "2"));
    }

    #[test]
    fn api_items_map_to_panel_items() {
        let json = r##"{
          "data": [{
            "id": "jel1jq", "url": "https://wallhaven.cc/w/jel1jq",
            "path": "https://w.wallhaven.cc/full/full-1.jpg",
            "resolution": "1920x1080", "file_size": "123456",
            "file_type": "image/jpeg",
            "colors": ["#123456", "#abcdef"],
            "thumbs": {"large": "https://w.wallhaven.cc/l.jpg", "small": "https://w.wallhaven.cc/s.jpg"},
            "source": "https://example.com/artist"
          }],
          "meta": {"current_page": 1, "last_page": 9}
        }"##;
        let parsed: SearchResponse = serde_json::from_str(json).unwrap();
        let page = SearchPage {
            items: parsed.data.into_iter().map(WallhavenItem::from).collect(),
            page: 1,
            last_page: 9,
        };
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].colors.len(), 2);
        assert_eq!(page.last_page, 9);
    }
}
