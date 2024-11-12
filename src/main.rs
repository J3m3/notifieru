//! # Simple & Minimal Todo Notifier
//!
//! The output binary notifies Todos listed in your Notion database, achieving
//! synchronization across different devices.
//!
//! To connect any Notion database with notifieru, you should provide `.secrets`
//! file in the project root directory. The format of `.secrets` file content
//! should follow the below specification (the order of the lines may vary):
//!
//! ```
//! DB_URL=<database_url>
//! API_KEY=<api_key>
//! ```

mod secrets;

use secrets::Secret;

use minreq;
use serde_json::{json, Value};

use std::env;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> crate::Result<()> {
    let _args: Vec<_> = env::args().collect();

    let secret_path = PathBuf::from(".secrets");
    let Secret { db_url, api_key } = secrets::read_secrets(&secret_path)?;

    let res = minreq::post(&db_url)
        .with_header("Authorization", format!("Bearer {api_key}"))
        .with_header("Notion-Version", "2022-06-28")
        .with_header("Content-Type", "application/json")
        .with_json(&json!({"sorts": [{"property": "Due", "direction": "ascending"}]}))?
        .send()?;

    process_todos(res)
}

fn process_todos(res: minreq::Response) -> crate::Result<()> {
    let json = res.json::<Value>()?;
    let todos = json["results"]
        .as_array()
        .ok_or("expected 'results' array field which is not present in the response")?;

    let mut errors: Vec<String> = Vec::new();

    for (i, todo) in todos.iter().enumerate() {
        let properties = &todo["properties"];

        let title = match properties["Name"]["title"][0]["plain_text"].as_str() {
            Some(t) => t,
            None => {
                errors.push(format!("todo {i}: missing or invalid title"));
                continue;
            }
        };

        let start_date = properties["Due"]["date"]["start"].as_str();
        let end_date = properties["Due"]["date"]["end"].as_str();

        let done = match properties["Done"]["checkbox"].as_bool() {
            Some(d) => d,
            None => {
                errors.push(format!("todo {i}: missing or invalid 'Done' checkbox"));
                continue;
            }
        };

        let mut output = format!("[{}] {}: {:35} | ", if done { "x" } else { " " }, i, title);

        if let Some(start) = start_date {
            push_datetime(start, &mut output);
        }
        if let Some(end) = end_date {
            output.push_str(&format!(" ~ "));
            push_datetime(end, &mut output);
        }

        println!("{output}");
    }

    if !errors.is_empty() {
        eprintln!("Errors encountered while processing todos:");
        for error in errors {
            eprintln!("{}", error);
        }
    }

    Ok(())
}

fn push_datetime(datetime: &str, buf: &mut String) {
    let f = datetime
        .split_once('T')
        .inspect(|(ymd, t)| buf.push_str(&format!("{} {}", ymd, &t[..8])));
    if f.is_none() {
        buf.push_str(datetime);
    }
}
