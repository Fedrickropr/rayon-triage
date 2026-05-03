use std::process::Command;

use serde::{Deserialize, Serialize};

const INPUT_PATH: &str = "data/github_data.json";
const OUTPUT_PATH: &str = "data/analysis.json";
const REPOSITORY_DIR: &str = "data/repositories";

const PATTERNS: &[&str] = &[
    "par_iter",
    "par_chunks",
    "par_windows",
    "par_bridge",
    "par_extend",
    "flat_map_iter",
    "par_sort",
    "par_sort_by",
    "par_sort_by_key",
    "rayon::iter::fold",
    "reduce",
    "unzip",
    "rayon::scope",
    "rayon::join",
    "rayon::spawn",
    "ThreadPoolBuilder",
    "ThreadPool",
    "impl ParallelIterator",
    "rayon::broadcast",
    "scope_fifo",
];

#[derive(Debug, Deserialize)]
struct RepoItem {
    full_name: String,
    html_url: String,
    stargazers_count: u64,
}

#[derive(Debug, Serialize)]
struct AnalysisResult {
    full_name: String,
    stargazers_count: u64,
    patterns: Vec<(String, usize)>,
    unsafe_in_rayon_files: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(REPOSITORY_DIR)?;

    let repos: Vec<RepoItem> = serde_json::from_str(&std::fs::read_to_string(INPUT_PATH)?)?;
    let mut res: Vec<AnalysisResult> = Vec::new();

    let total = repos.len();
    for (i, repo) in repos.iter().enumerate() {
        println!(">>> Repo {}/{} {}", i + 1, total, repo.full_name);

        let repo_path = format!("{}/{}", REPOSITORY_DIR, repo.full_name.replace('/', "_"));

        if !std::path::Path::new(&repo_path).exists() {
            let status = Command::new("git")
                .args(["clone", "--depth=1", &repo.html_url, &repo_path])
                .status()?;

            if !status.success() {
                eprintln!(">>>  Failed to clone {}", repo.full_name);
                continue;
            }
        }

        let patterns = find_patterns(&repo_path);
        let unsafe_in_rayon_files = find_unsafe_in_rayon_files(&repo_path);

        println!(">>>  Found: {:?}", patterns);

        res.push(AnalysisResult {
            full_name: repo.full_name.clone(),
            stargazers_count: repo.stargazers_count,
            patterns,
            unsafe_in_rayon_files,
        });

        std::fs::write(OUTPUT_PATH, serde_json::to_string_pretty(&res)?)?;
    }

    Ok(())
}

fn find_patterns(clone_path: &str) -> Vec<(String, usize)> {
    PATTERNS
        .iter()
        .filter_map(|&pattern| {
            let output = Command::new("rg")
                .args(["--type", "rust", "-c", pattern, clone_path])
                .output()
                .ok()?;

            if !output.status.success() {
                return None;
            }

            let count: usize = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.split(':').last()?.parse::<usize>().ok())
                .sum();

            if count > 0 {
                Some((pattern.to_string(), count))
            } else {
                None
            }
        })
        .collect()
}

fn find_unsafe_in_rayon_files(clone_path: &str) -> usize {
    let rayon_files = Command::new("rg")
        .args(["--type", "rust", "-l", "rayon", clone_path])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    rayon_files
        .iter()
        .filter_map(|file| {
            let output = Command::new("rg")
                .args(["-c", "unsafe", file])
                .output()
                .ok()?;
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .split(':')
                .last()?
                .parse::<usize>()
                .ok()
        })
        .sum()
}
