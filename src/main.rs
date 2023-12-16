use actix_web::{web, App, HttpResponse, HttpServer};
use chrono::{DateTime, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::fs;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Deserialize)]
enum ApiType {
    Chainz,
    Blnscan,
}

#[derive(Deserialize)]
pub struct Coins {
    coins: Vec<Coin>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Coin {
    name: String,
    ticker: String,
    api: ApiType,
    address: String,
    #[serde(default)]
    balance: Option<f32>,
    #[serde(default)]
    last_block_timestamp: Option<u64>,
}

lazy_static! {
    static ref COINS: Mutex<Vec<Coin>> = Mutex::new(Vec::new());
}

fn load_coins() -> Vec<Coin> {
    let toml_str = match fs::read_to_string("coins.toml") {
        Ok(content) => content,
        Err(_) => panic!("Error reading the config file"),
    };

    let coins: Result<Coins, toml::de::Error> = toml::from_str(&toml_str);

    match coins {
        Ok(coins) => coins.coins,
        Err(e) => panic!("Error parsing TOML: {:?}", e),
    }
}

async fn update_coins_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut coins_list = COINS.lock().unwrap();
    for coin in &mut *coins_list {
        match coin.api {
            ApiType::Chainz => {
                let url = format!(
                    "https://chainz.cryptoid.info/{}/api.dws?q=addressinfo&a={}",
                    &coin.ticker.to_lowercase(),
                    &coin.address
                );
                let resp = reqwest::get(url).await?;
                let res = resp.text().await?;

                let json_data: serde_json::Value = serde_json::from_str(&res)?;
                if let Some(balance) = json_data.get("balance").and_then(|b| b.as_f64()) {
                    coin.balance = Some(balance as f32);
                }

                if let Some(last_timestamp) = json_data
                    .get("lastBlockTimestamp")
                    .and_then(|ts| ts.as_i64())
                {
                    coin.last_block_timestamp = Some(last_timestamp as u64);
                }
            }
            ApiType::Blnscan => {
                let url = "https://blnexplorer.io/api/account/".to_owned() + &coin.address;
                let resp = reqwest::get(url).await?;
                let res = resp.text().await?;

                let json_data: serde_json::Value = serde_json::from_str(&res)?;
                if let Some(txn) = json_data.get("txns").and_then(|txns| txns.get(0)) {
                    if let Some(last_timestamp) = txn.get("time") {
                        if let Some(timestamp) = last_timestamp
                            .as_i64()
                            .or_else(|| last_timestamp.as_str().and_then(|s| s.parse().ok()))
                        {
                            coin.last_block_timestamp = Some(timestamp as u64);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn respond() -> HttpResponse {
    let _ = update_coins_list().await;
    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Crypto Wallet Watcher</title>
            <style>
                h1 {{
                    border-bottom: 1px solid #ccc;
                    padding-bottom: 0.25em;
                }}
                h2 {{
                    display: flex;
                    align-items: center;
                    margin-bottom: 0.5em;
                }}
                h2 img {{
                    width: 32px;
                    height: 32px;
                    margin-right: 0.35em;
                }}
                .container {{
                    width: 800px;
                    margin: 0 auto;
                }}
                .row {{
                    margin: 1.5em 0;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Wallet Status</h1>
                {coins}
            </div>
        </body>
        </html>
    "#,
        coins = format_coins(&COINS.lock().unwrap())
    );

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html_content)
}

fn get_time_since_last_activity(last_timestamp: u64) -> String {
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let duration = Duration::from_secs(current_timestamp - last_timestamp);

    let days = duration.as_secs() / (60 * 60 * 24);
    let hours = (duration.as_secs() % (60 * 60 * 24)) / (60 * 60);
    let minutes = (duration.as_secs() % (60 * 60)) / 60;
    let seconds = duration.as_secs() % 60;

    if days > 0 {
        format!(
            "{} day{}, {} hour{}, {} minute{}, {} second{}",
            days,
            if days != 1 { "s" } else { "" },
            hours,
            if hours != 1 { "s" } else { "" },
            minutes,
            if minutes != 1 { "s" } else { "" },
            seconds,
            if seconds != 1 { "s" } else { "" }
        )
    } else if hours > 0 {
        format!(
            "{} hour{}, {} minute{}, {} second{}",
            hours,
            if hours != 1 { "s" } else { "" },
            minutes,
            if minutes != 1 { "s" } else { "" },
            seconds,
            if seconds != 1 { "s" } else { "" }
        )
    } else if minutes > 0 {
        format!(
            "{} minute{}, {} second{}",
            minutes,
            if minutes != 1 { "s" } else { "" },
            seconds,
            if seconds != 1 { "s" } else { "" }
        )
    } else {
        format!("{} second{}", seconds, if seconds != 1 { "s" } else { "" })
    }
}

fn format_timestamp(timestamp: u64) -> String {
    if let Some(naivedatetime) = NaiveDateTime::from_timestamp_opt(timestamp as i64, 0) {
        let datetime = DateTime::<Utc>::from_naive_utc_and_offset(
            naivedatetime,
            Utc,
        );
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        String::from("?")
    }
}

fn format_coins(coins: &[Coin]) -> String {
    coins.iter().map(|coin| {
        format!(
            r#"
                <div class="row">
                    <h2><img src="{}" alt="{}">{}</h2>
                    Address: <a href="{}">{}</a><br>
                    Balance: {}<br>
                    Last Active On: {}<br>
                    Time Since Last Activity: {}
                </div>
            "#,
            if coin.api == ApiType::Chainz {
                format!("https://chainz.cryptoid.info/logo/{}.png", coin.ticker.to_lowercase())
            } else if coin.api == ApiType::Blnscan {
                "https://blnexplorer.io/favicon.ico".to_string()
            }
            else {
                // Return an empty string for now
                String::new()
            },
            coin.name,
            coin.name,
            if coin.api == ApiType::Chainz {
                format!("https://chainz.cryptoid.info/{}/address.dws?{}.htm", coin.ticker.to_lowercase(), coin.address)
            } else if coin.api == ApiType::Blnscan {
                format!("https://blnexplorer.io/{}", coin.address)
            }
            else {
                // Return an empty string for now
                String::new()
            },
            coin.address,
            coin.balance.map_or("?".to_string(), |balance| balance.to_string() + &coin.ticker),
            coin.last_block_timestamp.map_or("?".to_string(), |timestamp| format_timestamp(timestamp)),
            if let Some(last_timestamp) = coin.last_block_timestamp { get_time_since_last_activity(last_timestamp) } else { "?".to_string() },
        )
    }).collect()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let initial_coins = load_coins();
    *COINS.lock().unwrap() = initial_coins;

    HttpServer::new(move || App::new().service(web::resource("/").to(respond)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
