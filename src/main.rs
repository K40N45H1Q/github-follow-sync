use anyhow::{bail, Context, Result};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env, fs, time::Duration};

const MAX_ACTIONS: usize = 10;

#[derive(Deserialize)]
struct User {
    id: u64,
    login: String,
}

#[derive(Serialize, Deserialize)]
struct State {
    auto_followed: BTreeMap<u64, String>,
}

impl State {
    fn load() -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read("state.json").context("Missing state.json")?)?)
    }

    fn save(&self) -> Result<()> {
        Ok(fs::write("state.json", serde_json::to_vec_pretty(self)?)?)
    }
}

fn request(client: &Client, token: &str, method: Method, path: &str) -> reqwest::RequestBuilder {
    client
        .request(method, format!("https://api.github.com{path}"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
}

async fn list(client: &Client, token: &str, path: &str) -> Result<BTreeMap<u64, String>> {
    let mut users = BTreeMap::new();

    for page in 1..=100 {
        let url = format!("{path}?per_page=100&page={page}");
        let chunk: Vec<User> = request(client, token, Method::GET, &url)
            .send().await?.error_for_status()?.json().await?;

        let count = chunk.len();

        for user in chunk {
            users.insert(user.id, user.login);
        }

        if count < 100 {
            return Ok(users);
        }
    }

    bail!("More than 10,000 accounts in {path}; sync stopped safely")
}

async fn update(client: &Client, token: &str, login: &str, follow: bool) -> Result<()> {
    let method = if follow { Method::PUT } else { Method::DELETE };
    let mut req = request(client, token, method, &format!("/user/following/{login}"));

    if follow {
        req = req.body(Vec::<u8>::new());
    }

    let response = req.send().await?;

    if response.status() != StatusCode::NO_CONTENT {
        bail!("GitHub rejected @{login}: HTTP {}", response.status());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let token = env::var("GH_FOLLOW_TOKEN").context("GH_FOLLOW_TOKEN is missing")?;
    let client = Client::builder()
        .user_agent("github-follow-sync")
        .timeout(Duration::from_secs(30))
        .build()?;

    let me: User = request(&client, &token, Method::GET, "/user")
        .send().await?.error_for_status()?.json().await?;

    let followers = list(&client, &token, "/user/followers").await?;
    let following = list(&client, &token, "/user/following").await?;
    let mut state = State::load()?;
    let mut changes = 0;

    for (id, login) in &following {
        if followers.contains_key(id) { continue; }
        if changes == MAX_ACTIONS { break; }

        update(&client, &token, login, false).await?;
        state.auto_followed.remove(id);
        state.save()?;

        println!("Unfollowed @{login}");
        changes += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    for (id, login) in followers {
        if id == me.id || following.contains_key(&id) { continue; }
        if changes == MAX_ACTIONS { break; }

        update(&client, &token, &login, true).await?;
        state.auto_followed.insert(id, login.clone());
        state.save()?;

        println!("Followed @{login}");
        changes += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    println!("@{}: {changes} changes", me.login);
    Ok(())
}