use serde::{Serialize, Deserialize};
use chrono::Utc;
use once_cell::sync::Lazy;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;
#[derive(Serialize)]
#[derive(Deserialize)]
#[derive(Debug)]
pub struct Event
{
    pub id : i32,
    pub name: String,
    pub time: String
}

#[derive(Clone)]
pub struct Logger
{
    path : String,
}

static LAST_ID: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));
impl Logger
{
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    // Background task which listens to channel
    pub async fn run(&self, mut rx: Receiver<String>) {
        while let Some(task_name) = rx.recv().await {
            if let Err(e) = self.log_event(&task_name).await {
                eprintln!("Logging error: {}", e);
            }
        }

        println!("Channel closed. Exit Logger ");
    }

    async fn init_last_id(&self) -> i32 {

        let file = File::open(&self.path).await.ok();
        if file.is_none() {
            return 0;
        }
        let reader = BufReader::new(file.unwrap());
        let mut lines = reader.lines();
        let mut last_id = 0;

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                last_id = event.id;
            }
        }
        last_id // return
    }

    //Log single event
    async fn log_event(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut cached = LAST_ID.lock().await;
        if *cached == 0 {
            *cached = self.init_last_id().await;
        }
        *cached += 1;

        let event = Event {
            id: *cached,
            name: name.to_string(),
            time: Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&event)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn load_events(&self) -> Vec<Event> {
        let file = File::open(&self.path).await.ok();
        if file.is_none() { return Vec::new(); }

        let reader = BufReader::new(file.unwrap());
        let mut lines = reader.lines();
        let mut events = Vec::new();

        // Some returns Option<>
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                events.push(event);
            }
        }

        events // return
    }
}

mod tests {
    use super::*;
    async fn test_event_serialization() {
        let event = Event {
            id: 1,
            name: "Test".into(),
            time: "2025-08-16T12:00:00Z".into(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();

        assert_eq!(event.id, parsed.id);
        assert_eq!(event.name, parsed.name);
    }
}





