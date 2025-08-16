mod logger;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::{io::Write};
use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use crate::logger::Logger;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "devlog")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        #[arg(short, long)]
        task: String,
    },
}

#[derive(Serialize, Deserialize)]
struct TaskLog {
    task: String,
    started_at: String,
}

#[tokio::main]
async fn main() {
    // Arc is used to let logger been accessible between multiple tasks
    let logger = Arc::new(Logger::new("log.jsonl"));
    // still not quite sure, why we need to clone the logger object here.
    let logger_for_task = logger.clone();
    //this channel for an asynchronous logging
    let (tx, rx) = mpsc::channel(100);

    let logger_task = tokio::spawn(async move {
        logger_for_task.run(rx).await;
    });

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    // enter the name of the task
    while let Ok(Some(line)) = lines.next_line().await {
        let input = line.trim();
        if input.is_empty() { continue; }

        match input {
            "show" => {
                let events = logger.load_events().await;
                if events.is_empty() {
                    println!("No events.");
                } else {
                    for e in &events {
                        println!("{:?}", e);
                    }
                }
            }
            "exit" => {
                println!("Exit...");
                break;
            }

            task_name => {
                if let Err(e) = tx.send(task_name.to_string()).await {
                    eprintln!("Cannot send task: channel is empty. {}", e);
                    break;
                }
            }
        }
    }
    // close channel
    drop(tx);
    logger_task.await.unwrap();
}