use std::{collections::HashMap, net::SocketAddr, sync::Arc};

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
    users: HashMap<SocketAddr, mpsc::Sender<String>>,
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
    let stream = Framed::new(stream, LinesCodec::new());
    info!("Handling connection: {}", addr);
    let (tx, mut rx) = mpsc::channel(MAX_CHANNEL_SIZE);
    state.lock().await.add_user(addr, tx);

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

        for (user_addr, tx) in state.lock().await.users.iter() {
            if *user_addr != addr {
                info!("Sending message to channel: {}", user_addr);
                tx.send(line.clone()).await?;
            }
        }
    }
    info!("Connection closed: {}", addr);

    Ok(())
}

impl ChatState {
    fn add_user(&mut self, addr: SocketAddr, sender: mpsc::Sender<String>) {
        self.users.insert(addr, sender);
        info!("current users: {:?}", self.users.keys());
    }
}
