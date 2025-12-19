use std::path::PathBuf;
use cliclack::{confirm, input, intro, log, note, outro, password, spinner};
use reqwest::Client;
use tracing_subscriber::EnvFilter;
use wp_mini::field::StoryField;
use wp_mini::types::StoryResponse;
use wp_mini::WattpadClient;
use wp_mini_epub::download_story_to_folder;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    intro("WattDownload")?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let http_client = Client::builder()
        .cookie_store(true)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36")
        .build()?;

    let wattpad_client = WattpadClient::new();

    let base_story_fields = vec![
        StoryField::Title,
        StoryField::Description,
        StoryField::Mature,
    ];

    let story_id: u64;

    loop {
        let story_id_input: u64 = input("Enter Story ID")
            .placeholder("e.g. 123456789")
            .interact()?;

        let StoryResponse {
            title: story_title,
            description: story_description,
            mature: story_is_mature,
            ..
        } = wattpad_client
            .story
            .get_story_info(story_id_input, Some(&base_story_fields))
            .await?;

        let story_title = story_title.unwrap_or("Unknown Title".to_string());
        let mut story_description = story_description.unwrap_or("Unknown Description".to_string());

        const MAX_CAPTION_LENGTH: usize = 100;

        if 0 < MAX_CAPTION_LENGTH {
            let ellipsis = "...";
            if story_description.chars().count() > MAX_CAPTION_LENGTH {
                if MAX_CAPTION_LENGTH > ellipsis.len() {
                    let trim_to = MAX_CAPTION_LENGTH - ellipsis.len();
                    story_description = story_description.chars().take(trim_to).collect();
                    story_description.push_str(ellipsis);
                } else {
                    story_description.clear();
                }
            }
        } else {
            story_description.clear();
        }

        let story_is_mature = story_is_mature
            .map(|m| if m { "Mature" } else { "Not Mature" })
            .unwrap_or("Unknown");

        note(
            "Story Details",
            format!(
                "Title:       {}\nDescription: {}\nIs Mature:   {}",
                story_title, story_description, story_is_mature
            ),
        )?;

        let is_correct = confirm("Is this the story you want to proceed?").interact()?;

        if is_correct {
            story_id = story_id_input;
            break;
        } else {
            log::info("Don't worry, let's start over")?;
        }
    }

    let include_images = confirm("Do you want to include images?").interact()?;

    let semaphore_count: u64 = input("Please enter no of concurrent chapters to be processed concurrently. (Default is 20)")
        .placeholder("e.g. 20")
        .default_input("20")
        .interact()?;

    let output_dir: PathBuf = input("Please enter output directory path (relative or absolute) (Default is current dir)")
        .placeholder("Ex: C:/book_downloads")
        .default_input("./")
        .interact()?;

    let want_auth = confirm("Do you want to login to Wattpad?").interact()?;

    if want_auth {
        loop {
            let username: String = input("Please enter your Wattpad username").interact()?;
            let pass: String = password("Please enter your Wattpad password")
                .mask('▪')
                .interact()?;

            let spin = spinner();
            spin.start("Checking authentication status...");

            if wp_mini_epub::login(&wattpad_client, &username, &pass).await.is_ok() {
                spin.stop("Authentication successful!");
                break;
            } else {
                spin.error("Authentication failed");
                log::error("Failed to authenticate, retry.")?;
            }
        }
    }

    let spin = spinner();
    spin.start("Processing download...");

    download_story_to_folder(
        &wattpad_client,
        &http_client,
        story_id,
        include_images,
        semaphore_count as usize,
        &output_dir,
        None,
    )
    .await?;

    spin.stop("Download complete!");

    outro("Job's done!!!")?;

    Ok(())
}