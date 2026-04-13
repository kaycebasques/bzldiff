use scraper::{Html, Selector};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Scrape links (original functionality)
    scrape_links()?;

    // 2. Write message to workspace root (new functionality)
    write_message_to_workspace()?;

    Ok(())
}

fn scrape_links() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scraping links from pigweed.dev...");
    let response = reqwest::blocking::get("https://pigweed.dev")?.text()?;
    let document = Html::parse_document(&response);
    let selector = Selector::parse("a").unwrap();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            println!("{}", href);
        }
    }
    Ok(())
}

fn write_message_to_workspace() -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking BUILD_WORKSPACE_DIRECTORY...");
    if let Ok(workspace_dir) = env::var("BUILD_WORKSPACE_DIRECTORY") {
        println!("Found workspace dir: {}", workspace_dir);
        let mut path = PathBuf::from(workspace_dir);
        path.push("message.txt");
        
        let mut file = File::create(&path)?;
        file.write_all(b"Hello, world!\n")?;
        println!("Wrote to {}", path.display());
    } else {
        println!("BUILD_WORKSPACE_DIRECTORY not set. Are you running with 'bazel run'?");
    }
    Ok(())
}
