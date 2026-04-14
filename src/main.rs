use scraper::{Html, Selector};
use std::env;
use std::fs;
use std::path::Path;
use url::Url;

const START_URL: &str = "https://technicalwriting.dev";

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
    
    loop {
        let mut undone_pages = Vec::new();
        find_undone_pages(&data_dir, &mut undone_pages)?;
        
        if undone_pages.is_empty() {
            println!("No more undone pages. Crawling complete.");
            break;
        }
        
        println!("Found {} undone pages.", undone_pages.len());
        
        // Process the first undone page
        let current_dir = &undone_pages[0];
        
        // Map path back to URL
        let rel_path = current_dir.strip_prefix(&data_dir)?;
        let rel_path_str = rel_path.to_str().unwrap();
        let current_url = if rel_path_str.is_empty() {
            START_URL.to_string()
        } else {
            format!("{}/{}", START_URL, rel_path_str)
        };
        
        println!("Processing {}...", current_url);
        
        // Fetch page
        let response = reqwest::blocking::get(&current_url);
        
        match response {
            Ok(resp) => {
                let status = resp.status().as_u16().to_string();
                let prod_file = current_dir.join("prod.txt");
                
                println!("Writing status {} to {}", status, prod_file.display());
                fs::write(prod_file, status)?;
                
                let html = resp.text()?;
                let links = extract_links(&html, &current_url)?;
                
                for link in links {
                    if should_process_link(&link) {
                        let url = Url::parse(&link)?;
                        let path = url.path();
                        let rel_link_path = path.trim_start_matches('/');
                        let target_dir = data_dir.join(rel_link_path);
                        
                        let done_file = target_dir.join("done.txt");
                        if !done_file.exists() {
                            println!("Found new link: {}. Queuing...", link);
                            fs::create_dir_all(&target_dir)?;
                            fs::write(done_file, "0")?;
                        }
                    }
                }
                
                // Mark current page as done
                fs::write(current_dir.join("done.txt"), "1")?;
            }
            Err(e) => {
                eprintln!("Failed to fetch '{}': {}", current_url, e);
                // Mark as done with error to avoid infinite loop
                println!("Marking failed page as done to avoid infinite loop.");
                fs::write(current_dir.join("done.txt"), "1")?;
            }
        }
    }
    
    println!("All crawled URLs:");
    print_all_urls(&data_dir, &data_dir, START_URL)?;
    
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
    
    let mut base_url = Url::parse(START_URL).unwrap();
    
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

fn print_all_urls(dir: &Path, data_dir: &Path, start_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                print_all_urls(&path, data_dir, start_url)?;
            } else if path.file_name().and_then(|s| s.to_str()) == Some("done.txt") {
                let rel_path = dir.strip_prefix(data_dir)?;
                let rel_path_str = rel_path.to_str().unwrap();
                let url = if rel_path_str.is_empty() {
                    start_url.to_string()
                } else {
                    format!("{}/{}", start_url, rel_path_str)
                };
                println!("{}", url);
            }
        }
    }
    Ok(())
}
