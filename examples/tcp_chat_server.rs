use std::{
    collections::HashMap,
    fmt::{self, Display},
    net::SocketAddr,
    sync::Arc,
};

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

const MAX_CHANNEL_SIZE: usize = 32;

#[derive(Debug)]
struct ChatState {
    users: HashMap<SocketAddr, User>,
}

#[derive(Debug)]
enum Message {
    UserJoined(String),
    UserLeft(String),
    Chat { from: String, message: String },
}

#[derive(Debug, Clone)]
#[allow(unused)]
struct User {
    name: String,
    sender: mpsc::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new()
        .with_writer(std::io::stdout)
        .pretty()
        .with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on: {}", addr);

    let state = Arc::new(Mutex::new(ChatState {
        users: HashMap::new(),
    }));

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from: {}", addr);

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, state).await {
                info!("Error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: Arc<Mutex<ChatState>>,
) -> Result<()> {
    let mut stream = Framed::new(stream, LinesCodec::new());
    info!("Handling connection: {}", addr);

    let mut state = state.lock().await;

    let (tx, mut rx) = mpsc::channel(MAX_CHANNEL_SIZE);
    stream.send("Please enter your name: ".to_string()).await?;
    let name = match stream.next().await {
        Some(Ok(name)) => name,
        _ => anyhow::Error::msg("Failed to get name from user").to_string(),
    };
    state.add_user(addr, tx, &name);
    state
        .broadcast(Message::UserJoined(name.clone()), addr)
        .await?;

    let (mut writer, mut reader) = stream.split();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            writer.send(msg).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    while let Some(line) = reader.next().await {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                info!("Error: {:?}", e);
                break;
            }
        };
        info!("Received message: {}", line);

        state
            .broadcast(
                Message::Chat {
                    from: name.clone(),
                    message: line,
                },
                addr,
            )
            .await?;
    }
    state.users.remove(&addr);
    state
        .broadcast(Message::UserLeft(name.clone()), addr)
        .await?;
    info!("Connection closed: {}", addr);

    Ok(())
}

impl ChatState {
    fn add_user(&mut self, addr: SocketAddr, sender: mpsc::Sender<String>, name: &str) {
        let user = User {
            name: name.to_string(),
            sender,
        };
        self.users.insert(addr, user.clone());
        info!("current users: {:?}", self.users.keys());
    }

    async fn broadcast(&mut self, msg: Message, addr: SocketAddr) -> Result<()> {
        for (user_addr, tx) in self.users.iter() {
            if *user_addr != addr {
                info!("Sending message to channel: {}", user_addr);
                tx.sender.send(msg.to_string()).await?;
            }
        }
        Ok(())
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::UserJoined(name) => write!(f, "{} joined the chat", name),
            Message::UserLeft(name) => write!(f, "{} left the chat", name),
            Message::Chat { from, message } => write!(f, "{}: {}", from, message),
        }
    }
}
