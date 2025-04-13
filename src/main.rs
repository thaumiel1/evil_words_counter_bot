// std imports
use std::fmt::format;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::Path;

// Discord bot import
use poise::serenity_prelude as serenity;
use serenity::http::Http;
use serenity::model::id::UserId;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[derive(Debug)]
struct Entry {
    username: String,
    logged_words: usize,
}

#[poise::command(slash_command, prefix_command)]
async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let http = Http::new(&token);
    let mut entries: Vec<Entry> = Vec::new();
    if let Ok(lines) = read_lines("./db.txt") {
        for line in lines.map_while(Result::ok) {
            let username = get_username_from_user_id(
                &http,
                line.split_whitespace()
                    .next()
                    .unwrap()
                    .replace(",", "")
                    .replace(" ", "")
                    .parse::<u64>()
                    .unwrap(),
            )
            .await
            .unwrap();
            let comma_count = line.chars().filter(|&c| c == ',').count();
            let entry = Entry {
                username,
                logged_words: comma_count - 1,
            };
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| b.logged_words.cmp(&a.logged_words));
    let text = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            format!(
                "{}. {} - {} words",
                i + 1,
                entry.username,
                entry.logged_words
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    ctx.reply(text).await?;
    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[poise::command(slash_command, prefix_command)]
async fn evil(
    ctx: Context<'_>,
    #[description = "Selected user"] user: serenity::User,
    #[description = "Word used"] word: String,
) -> Result<(), Error> {
    let u = user.id.to_string();
    if !take_all_names().contains(&u) {
        let mut file = OpenOptions::new().append(true).open("db.txt")?;
        writeln!(file, "{}, ", u)?;
        append_word_to_line(&user, &word)?;
    } else {
        append_word_to_line(&user, &word)?;
    }
    let response = format!("{} saying {} logged.", user.to_string(), word);
    ctx.say(response).await?;
    Ok(())
}

fn append_word_to_line(user: &serenity::User, word: &String) -> Result<(), Error> {
    let mut lines: Vec<String> = io::BufReader::new(File::open("db.txt")?)
        .lines()
        .filter_map(Result::ok)
        .collect();
    let u = user.id.to_string();
    for line in &mut lines {
        if line.starts_with(&u) {
            line.push_str(&format!(" {}, ", word));
            break;
        }
    }
    let mut file = File::create("db.txt")?;
    for line in lines {
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

fn take_all_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(file) = File::open("db.txt") {
        for line in io::BufReader::new(file).lines() {
            if let Ok(line) = line {
                if let Some(first_word) = line.split_whitespace().next() {
                    let cleaned = first_word.trim_end_matches(',');
                    names.push(cleaned.to_string());
                }
            }
        }
    }
    names
}

async fn get_username_from_user_id(http: &Http, user_id: u64) -> Option<String> {
    let user_id = UserId::new(user_id);
    match user_id.to_user(http).await {
        Ok(user) => Some(user.name),
        Err(_) => None,
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let file_path = "db.txt";
    let file = File::open(file_path).unwrap();
    let lines: Vec<String> = io::BufReader::new(file)
        .lines()
        .filter_map(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut file = File::create(file_path).unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age(), evil(), leaderboard()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
