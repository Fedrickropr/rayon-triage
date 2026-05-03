use std::time::Instant;

use reqwest::header::{HeaderMap, HeaderValue, IntoHeaderName, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

const GITHUB_ENDPOINT: &str = "https://api.github.com/search/code?q=";
const PAGE_SIZE: u64 = 100;
const FILE_PATH: &str = "data/github_data.json";
const TOKEN: &str = "";

#[derive(Debug, Deserialize)]
struct CodeSearchResponse {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<CodeSearchItem>,
}

#[derive(Debug, Deserialize)]
struct CodeSearchItem {
    name: String,
    path: String,
    sha: String,
    url: String,
    html_url: String,
    repository: Repository,
    score: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Repository {
    id: u64,
    name: String,
    full_name: String,
    private: bool,
    fork: bool,
    description: Option<String>,
    url: String,
    html_url: String,
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
    let mut repos: std::collections::HashMap<String, Repository> = std::collections::HashMap::new();
    let mut page = 1;

    let url_base = format!(
        "{}rayon+language:Rust+filename:Cargo.toml&per_page={}&page=",
        GITHUB_ENDPOINT, PAGE_SIZE
    );

    loop {
        let res: CodeSearchResponse = client.get(format!("{}{}", url_base, page)).send()?.json()?;

        let total_pages = (std::cmp::min(res.total_count, 1000) + (PAGE_SIZE - 1)) / PAGE_SIZE;

        if res.incomplete_results {
            eprintln!("Warning: incomplete results on page {}", page);
        }

        for item in res.items {
            if !item.repository.fork {
                repos
                    .entry(item.repository.full_name.clone())
                    .or_insert(item.repository);
            }
        }

        println!(
            ">>> Page {}/{} ({} unique repos so far)",
            page,
            total_pages,
            repos.len()
        );

        let repos_vec: Vec<&Repository> = repos.values().collect();
        write_file(&repos_vec)?;

        if page >= total_pages {
            break;
        }
        page += 1;

        std::thread::sleep(std::time::Duration::from_millis(2000));
    }

    Ok(())
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
