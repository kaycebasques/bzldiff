use scraper::{Html, Selector};
use std::env;
use std::fs;
use std::path::Path;
use url::Url;

const OLD_URL: &str = "https://bazel.build";
const NEW_URL: &str = "https://preview.bazel.build";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting crawler...");
    
    let workspace_dir = env::var("BUILD_WORKSPACE_DIRECTORY")
        .expect("BUILD_WORKSPACE_DIRECTORY not set. Run with 'bazel run'");
    let data_dir = Path::new(&workspace_dir).join("data");
    
    println!("Data directory: {}", data_dir.display());
    
    // Ensure data directory exists
    fs::create_dir_all(&data_dir)?;
    
    let homepage_done_file = data_dir.join("done.txt");
    if !homepage_done_file.exists() {
        println!("Initializing queue with homepage...");
        fs::write(&homepage_done_file, "0")?;
    }
   
    // Process homepage first
    let mut undone_pages = Vec::new();
    find_undone_pages(&data_dir, &mut undone_pages)?;
    
    if let Some(homepage_dir) = undone_pages.first() {
        let homepage_url = path_to_url(homepage_dir, &data_dir)?;
        println!("Processing homepage: {}", homepage_url);
        process_page(homepage_dir, &homepage_url, &data_dir)?;
    }
    
    // Now process links found on homepage
    let mut homepage_links = Vec::new();
    find_undone_pages(&data_dir, &mut homepage_links)?;
    
    println!("Found {} links on homepage to check.", homepage_links.len());
    
    for dir in homepage_links {
        let url = path_to_url(&dir, &data_dir)?;
        check_link_existence(&dir, &url)?;
    }
    
    println!("Finished checking homepage links. Exiting as requested.");
    

    
    Ok(())
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
                Ok(mut url) => {
                    url.set_fragment(None);
                    
                    let path = url.path().to_string();
                    if path.ends_with("/index.html") {
                        url.set_path(&path[..path.len() - 10]);
                    }
                    
                    links.push(url.to_string());
                }
                Err(e) => eprintln!("Failed to parse URL '{}': {}", href, e),
            }
        }
    }
    Ok(links)
}

fn should_process_link(url_str: &str) -> bool {
    let mut url = match Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };
    
    let mut base_url = Url::parse(OLD_URL).unwrap();
    
    url.set_fragment(None);
    base_url.set_fragment(None);
    
    // Skip homepage
    if url == base_url {
        return false;
    }
    
    // Must be same domain
    if url.domain() != base_url.domain() {
        return false;
    }
    
    true
}



fn find_undone_pages(dir: &Path, results: &mut Vec<std::path::PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_undone_pages(&path, results)?;
            } else if path.file_name().and_then(|s| s.to_str()) == Some("done.txt") {
                let content = fs::read_to_string(&path)?;
                if content.trim() == "0" {
                    results.push(dir.to_path_buf());
                }
            }
        }
    }
    Ok(())
}



fn path_to_url(path: &Path, data_dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let rel_path = path.strip_prefix(data_dir)?;
    let rel_path_str = rel_path.to_str().unwrap();
    if rel_path_str.is_empty() {
        Ok(OLD_URL.to_string())
    } else {
        Ok(format!("{}/{}", OLD_URL, rel_path_str))
    }
}

fn queue_link(link: &str, data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(link)?;
    let path = url.path();
    let rel_link_path = path.trim_start_matches('/');
    let target_dir = data_dir.join(rel_link_path);
    
    let done_file = target_dir.join("done.txt");
    if !done_file.exists() {
        println!("Found new link: {}. Queuing...", link);
        fs::create_dir_all(&target_dir)?;
        fs::write(done_file, "0")?;
    }
    Ok(())
}

fn process_page(dir: &Path, url: &str, data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing {}...", url);
    let response = reqwest::blocking::get(url);
    
    match response {
        Ok(resp) => {
            let status = resp.status().as_u16().to_string();
            let old_file = dir.join("old.txt");
            
            println!("Writing status {} to {}", status, old_file.display());
            fs::write(old_file, status)?;
            
            // Check on new site (NEW_URL)
            let new_url = url.replace(OLD_URL, NEW_URL);
            println!("Checking new site: {}...", new_url);
            let new_resp = reqwest::blocking::get(&new_url);
            let new_status = match new_resp {
                Ok(r) => r.status().as_u16().to_string(),
                Err(e) => {
                    eprintln!("Failed to fetch from new site '{}': {}", new_url, e);
                    "Error".to_string()
                }
            };
            let new_file = dir.join("new.txt");
            println!("Writing status {} to {}", new_status, new_file.display());
            fs::write(new_file, new_status)?;

            let html = resp.text()?;
            let links = extract_links(&html, url)?;
            
            for link in links {
                if should_process_link(&link) {
                    queue_link(&link, data_dir)?;
                }
            }
            
            fs::write(dir.join("done.txt"), "1")?;
        }
        Err(e) => {
            eprintln!("Failed to fetch '{}': {}", url, e);
            println!("Marking failed page as done to avoid infinite loop.");
            fs::write(dir.join("done.txt"), "1")?;
        }
    }
    Ok(())
}

fn check_link_existence(dir: &Path, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking existence of {}...", url);
    let response = reqwest::blocking::get(url);
    
    match response {
        Ok(resp) => {
            let status = resp.status().as_u16().to_string();
            let old_file = dir.join("old.txt");
            
            println!("Writing status {} to {}", status, old_file.display());
            fs::write(old_file, status)?;
            
            // Check on new site (NEW_URL)
            let new_url = url.replace(OLD_URL, NEW_URL);
            println!("Checking new site: {}...", new_url);
            let new_resp = reqwest::blocking::get(&new_url);
            let new_status = match new_resp {
                Ok(r) => r.status().as_u16().to_string(),
                Err(e) => {
                    eprintln!("Failed to fetch from new site '{}': {}", new_url, e);
                    "Error".to_string()
                }
            };
            let new_file = dir.join("new.txt");
            println!("Writing status {} to {}", new_status, new_file.display());
            fs::write(new_file, new_status)?;

            fs::write(dir.join("done.txt"), "1")?;
        }
        Err(e) => {
            eprintln!("Failed to fetch '{}': {}", url, e);
            println!("Marking failed page as done to avoid infinite loop.");
            fs::write(dir.join("done.txt"), "1")?;
        }
    }
    Ok(())
}
