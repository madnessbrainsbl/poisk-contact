use super::extractor::transliterate_to_latin;
use reqwest::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub(super) struct Location {
    pub requested: String,
    pub native: String,
    pub country_code: String,
    /// Overpass area id of the resolved boundary (relation id + 3600000000).
    pub osm_area: Option<u64>,
    names: Vec<String>,
    countries: Vec<String>,
    resolved: bool,
    city: bool,
}

static CACHE: LazyLock<Mutex<HashMap<String, (Instant, Location)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn normalized(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|word| {
            if word.chars().any(|c| ('\u{0400}'..='\u{04ff}').contains(&c)) {
                transliterate_to_latin(word)
            } else {
                word.to_lowercase()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl Location {
    pub fn raw(query: &str) -> Self {
        Self {
            requested: query.to_string(),
            native: String::new(),
            country_code: String::new(),
            osm_area: None,
            names: vec![query.to_string()],
            countries: Vec::new(),
            resolved: false,
            city: false,
        }
    }

    fn unrestricted(&self) -> bool {
        matches!(
            self.requested.trim().to_lowercase().as_str(),
            "" | "remote" | "remote worldwide" | "worldwide" | "global" | "anywhere"
        )
    }

    pub fn matches(&self, text: &str, country: &str) -> bool {
        if self.unrestricted() {
            return true;
        }
        let text = format!(" {} ", normalized(text));
        let has_place = |place: &str| {
            let place = normalized(place);
            !place.is_empty() && text.contains(&format!(" {place} "))
        };
        if !self.resolved {
            return self.requested.split(',').all(has_place);
        }
        if !self.country_matches(country) {
            return false;
        }
        // shortcut: city text establishes relevance, not a verified postal address.
        self.names.iter().any(|name| has_place(name))
    }

    pub fn country_matches(&self, country: &str) -> bool {
        !self.resolved
            || self.unrestricted()
            || country.trim().is_empty()
            || self
                .countries
                .iter()
                .any(|name| normalized(name) == normalized(country))
    }

    pub(super) fn is_place_name(&self, text: &str) -> bool {
        let text = normalized(text);
        self.names.iter().any(|name| normalized(name) == text)
            || normalized(self.requested.split(',').next().unwrap_or("")) == text
    }

    pub(super) fn locality_conflicts(&self, locality: &str) -> bool {
        self.resolved && self.city && !locality.trim().is_empty() && !self.is_place_name(locality)
    }

    pub(super) fn from_photon(query: &str, english: &Value, native: &Value) -> Self {
        let mut location = Self::raw(query);
        let Some(features) = english["features"].as_array() else {
            return location;
        };
        for feature in features {
            let en = &feature["properties"];
            if !matches!(en["type"].as_str(), Some("city" | "state" | "country"))
                || en["osm_id"].is_null()
            {
                continue;
            }
            let local = native["features"]
                .as_array()
                .and_then(|features| {
                    features.iter().find(|f| {
                        f["properties"]["osm_id"] == en["osm_id"]
                            && f["properties"]["osm_type"] == en["osm_type"]
                    })
                })
                .map(|f| &f["properties"]);
            let primary = normalized(query.split(',').next().unwrap_or(""));
            if !std::iter::once(en).chain(local).any(|p| {
                p["name"]
                    .as_str()
                    .is_some_and(|name| normalized(name) == primary)
            }) {
                continue;
            }
            let mut known = Vec::new();
            for properties in std::iter::once(en).chain(local) {
                for key in ["name", "state", "country", "countrycode"] {
                    if let Some(value) = properties[key].as_str() {
                        known.push(normalized(value));
                    }
                }
            }
            if !query
                .split(',')
                .all(|part| !normalized(part).is_empty() && known.contains(&normalized(part)))
            {
                continue;
            }
            for properties in std::iter::once(en).chain(local) {
                if let Some(name) = properties["name"].as_str() {
                    location.names.push(name.into());
                    if en["type"] == "country" {
                        location.countries.push(name.into());
                    }
                }
                for key in ["country", "countrycode"] {
                    if let Some(name) = properties[key].as_str() {
                        location.countries.push(name.into());
                    }
                }
            }
            location.native = local.and_then(|p| p["name"].as_str()).unwrap_or("").into();
            location.country_code = en["countrycode"].as_str().unwrap_or("").to_string();
            location.osm_area = match (en["osm_type"].as_str(), en["osm_id"].as_u64()) {
                (Some("R"), Some(id)) => Some(3_600_000_000 + id),
                (Some("W"), Some(id)) => Some(2_400_000_000 + id),
                _ => None,
            };
            location.resolved = true;
            location.city = en["type"] == "city";
            return location;
        }
        location
    }

    pub async fn resolve(client: &Client, query: &str) -> Result<Self, String> {
        let raw = Self::raw(query);
        if raw.unrestricted() {
            return Ok(raw);
        }
        let mut cache = CACHE.lock().await;
        cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(300));
        if let Some((_, location)) = cache.get(query) {
            return Ok(location.clone());
        }
        let mut data = Vec::new();
        for language in [Some("en"), None] {
            let mut url = url::Url::parse("https://photon.komoot.io/api/").unwrap();
            url.query_pairs_mut().extend_pairs([
                ("q", query),
                ("limit", "5"),
                ("layer", "city"),
                ("layer", "state"),
                ("layer", "country"),
            ]);
            if let Some(language) = language {
                url.query_pairs_mut().append_pair("lang", language);
            }
            data.push(
                super::http::get_json(client, url, Duration::from_secs(8))
                    .await
                    .map_err(|e| format!("Location names unavailable: {e:#}"))?,
            );
        }
        let location = Self::from_photon(query, &data[0], &data[1]);
        if cache.len() >= 32 {
            cache.clear();
        }
        cache.insert(query.into(), (Instant::now(), location.clone()));
        Ok(location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_local_names_and_country_codes_without_requiring_region_text() {
        let en = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":1,"type":"city","name":"Krasnoyarsk","state":"Krasnoyarsk Krai","country":"Russia","countrycode":"RU"}}]});
        let native = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":1,"type":"city","name":"Красноярск","state":"Красноярский край","country":"Россия","countrycode":"RU"}}]});
        let place = Location::from_photon("Krasnoyarsk, Krasnoyarsk Krai, Russia", &en, &native);
        assert!(place.matches("Address: Красноярск, улица Ленина", "RU"));
        assert!(place.matches("Фотостудия Красноярск", ""));
        assert!(!place.matches("Office in Moscow", "RU"));
        assert!(!place.matches("Krasnoyarsk", "CA"));
        assert!(!place.matches("Krasnoyarskville", "RU"));
        assert!(place.locality_conflicts("Kazan"));
        assert!(!place.locality_conflicts("Красноярск"));
        assert!(!place.locality_conflicts(""));
        let wrong = Location::from_photon("Krasnoyarsk, Canada", &en, &native);
        assert!(!wrong.resolved);
        let unrelated = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":2,"type":"city","name":"Москва","country":"Россия","countrycode":"RU"}}]});
        assert_eq!(
            Location::from_photon("Krasnoyarsk", &en, &unrelated).native,
            ""
        );
        assert!(!Location::raw(", ,").matches("Clinic", ""));
    }

    #[test]
    fn chooses_the_city_when_geocoder_languages_rank_region_and_city_differently() {
        let en = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":2,"type":"state","name":"Krasnoyarsk Krai","country":"Russia","countrycode":"RU"}},{"properties":{"osm_type":"R","osm_id":1,"type":"city","name":"Krasnoyarsk","state":"Krasnoyarsk Krai","country":"Russia","countrycode":"RU"}}]});
        let native = serde_json::json!({"features":[{"properties":{"osm_type":"R","osm_id":2,"type":"state","name":"Красноярский край","country":"Россия","countrycode":"RU"}},{"properties":{"osm_type":"R","osm_id":1,"type":"city","name":"Красноярск","state":"Красноярский край","country":"Россия","countrycode":"RU"}}]});
        let location = Location::from_photon("Krasnoyarsk, Krasnoyarsk Krai, Russia", &en, &native);
        assert_eq!(location.native, "Красноярск");
        assert!(location.matches("Фотостудия Красноярск", "RU"));
    }
}
