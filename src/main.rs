use std::time::Instant;

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

const REVERSE_DEP_ENDPOINT: &str = "https://crates.io/api/v1/crates/rayon/reverse_dependencies";
const FILE_PATH: &str = "data/raw_crates.json";

#[derive(Debug, Deserialize)]
struct ReverseDepsResponse {
    versions: Vec<CrateVersion>,
    meta: Meta,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CrateVersion {
    id: u64,
    #[serde(rename = "crate")]
    crate_name: String,
    num: String,
    downloads: u64,
    description: Option<String>,
    repository: Option<String>,
    license: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    total: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    println!("Running");
    let timing = Instant::now();

    store_crates(&client)?;

    println!("Ran in {} ms", timing.elapsed().as_millis());

    sort_crates()?;

    Ok(())
}

fn store_crates(client: &reqwest::blocking::Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut res: std::collections::HashMap<String, CrateVersion> = std::collections::HashMap::new();
    let mut page = 1;

    loop {
        let url = format!("{}?page={}&per_page=100", REVERSE_DEP_ENDPOINT, page);
        let response: ReverseDepsResponse = client.get(&url).send()?.json()?;
        let total_pages = (response.meta.total + 99) / 100;

        for version in response.versions {
            let entry = res
                .entry(version.crate_name.clone())
                .or_insert(version.clone());

            if version.downloads > entry.downloads {
                *entry = version;
            }
        }

        println!(">>> Page {}/{}", page, total_pages);
        if page >= total_pages {
            break;
        }
        page += 1;

        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let crates: Vec<&CrateVersion> = res.values().collect();
    write_file(&crates)?;
    println!("Wrote {} crates to {}", crates.len(), FILE_PATH);

    Ok(())
}

fn write_file<T: Serialize>(crates: &[T]) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("data")?;
    std::fs::write(FILE_PATH, serde_json::to_string_pretty(&crates)?)?;

    Ok(())
}

fn sort_crates() -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(FILE_PATH)?;
    let mut crates: Vec<CrateVersion> = serde_json::from_str(&json)?;
    crates.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    std::fs::write(
        FILE_PATH.replace(".json", "_sorted.json"),
        serde_json::to_string_pretty(&crates)?,
    )?;

    Ok(())
}

fn get_client() -> Result<reqwest::blocking::Client, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("rayon-triage/0.1"));

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok(client)
}
