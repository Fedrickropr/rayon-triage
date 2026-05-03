use std::time::Instant;

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

const GITHUB_REPO_ENDPOINT: &str =
    "https://api.github.com/search/repositories?q=language:Rust&sort=stars&order=desc";
const GITHUB_CONTENTS_ENDPOINT: &str = "https://api.github.com/repos";
const PAGE_SIZE: u64 = 100;
const FILE_PATH: &str = "data/github_data.json";
const TOKEN: &str = "";

#[derive(Debug, Deserialize)]
struct RepoSearchResponse {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<RepoItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RepoItem {
    id: u64,
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: u64,
    fork: bool,
    owner: Owner,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Owner {
    login: String,
    id: u64,
    #[serde(rename = "type")]
    owner_type: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = get_client()?;

    println!("Running");
    let timing = Instant::now();

    get_projects(&client)?;

    println!("Ran in {} ms", timing.elapsed().as_millis());

    sort_crates()?;

    Ok(())
}

fn get_projects(client: &reqwest::blocking::Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut repos: Vec<RepoItem> = Vec::new();
    let mut page = 1;

    let url_base = format!("{}&per_page={}&page=", GITHUB_REPO_ENDPOINT, PAGE_SIZE);

    loop {
        let res: RepoSearchResponse = client.get(format!("{}{}", url_base, page)).send()?.json()?;

        let total_pages = (std::cmp::min(res.total_count, 1000) + (PAGE_SIZE - 1)) / PAGE_SIZE;

        if res.incomplete_results {
            eprintln!("Warning: incomplete results on page {}", page);
        }

        for item in res.items {
            if item.fork {
                continue;
            }
            if has_rayon(client, &item.full_name)? {
                println!(">>> Found repo with Rayon");
                repos.push(item);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        println!(
            ">>> Page {}/{} ({} unique repos so far)",
            page,
            total_pages,
            repos.len()
        );

        write_file(&repos)?;

        if page >= total_pages {
            break;
        }
        page += 1;

        std::thread::sleep(std::time::Duration::from_millis(1500));
    }

    Ok(())
}

fn has_rayon(
    client: &reqwest::blocking::Client,
    repo: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let url_base = format!("{}/{}/contents/Cargo.toml", GITHUB_CONTENTS_ENDPOINT, repo);
    let res = client.get(&url_base).send()?;

    if res.status() == 404 {
        return Ok(false);
    }

    let body: serde_json::Value = res.json()?;
    let content = body["content"].as_str().unwrap_or("").replace('\n', "");
    let decoded = STANDARD.decode(content)?;
    let text = String::from_utf8_lossy(&decoded);

    Ok(text.contains("rayon"))
}

fn write_file<T: Serialize>(crates: &[T]) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("data")?;
    std::fs::write(FILE_PATH, serde_json::to_string_pretty(&crates)?)?;

    Ok(())
}

fn sort_crates() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn get_client() -> Result<reqwest::blocking::Client, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("rayon-triage/0.1"));
    let token = format!("Bearer {}", TOKEN);
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&token)?);
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok(client)
}
