use scraper::{Html, Selector};
use std::env;
use std::fs;
use std::path::Path;
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting crawler...");
    
    let start_url = "https://bazel.build";
    let html = fetch_page(start_url)?;
    let links = extract_links(&html, start_url)?;
    
    let workspace_dir = env::var("BUILD_WORKSPACE_DIRECTORY")
        .expect("BUILD_WORKSPACE_DIRECTORY not set. Run with 'bazel run'");
    let data_dir = Path::new(&workspace_dir).join("data");
    
    println!("Data directory: {}", data_dir.display());
    
    for link in links {
        if should_process_link(&link, start_url) {
            create_dir_for_link(&link, &data_dir)?;
        }
    }
    
    Ok(())
}

fn fetch_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    println!("Fetching {}...", url);
    let response = reqwest::blocking::get(url)?.text()?;
    Ok(response)
}

fn extract_links(html: &str, base_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a").unwrap();
    let base = Url::parse(base_url)?;
    
    let mut links = Vec::new();
    
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            // Resolve relative URLs
            match base.join(href) {
                Ok(url) => links.push(url.to_string()),
                Err(e) => eprintln!("Failed to parse URL '{}': {}", href, e),
            }
        }
    }
    Ok(links)
}

fn should_process_link(url_str: &str, base_url_str: &str) -> bool {
    let url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };
    
    let base_url = match Url::parse(base_url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };
    
    // Must be same domain
    if url.domain() != base_url.domain() {
        return false;
    }
    
    // Must not be a subdomain
    if url.domain() != Some("bazel.build") {
        return false;
    }
    
    true
}

fn create_dir_for_link(url_str: &str, data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(url_str)?;
    let path = url.path();
    
    // Trim leading slash to make it relative for joining
    let rel_path = path.trim_start_matches('/');
    
    if rel_path.is_empty() {
        return Ok(());
    }
    
    let target_dir = data_dir.join(rel_path);
    
    println!("Creating directory: {}", target_dir.display());
    fs::create_dir_all(target_dir)?;
    
    Ok(())
}
