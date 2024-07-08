use std::{collections::HashMap, net::SocketAddr, ops::DerefMut, sync::Arc};

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
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
        info!("Accecpted connection from: {}", addr);

        let state = Arc::clone(&state);
        let stream = Arc::new(Mutex::new(stream));
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, state).await {
                info!("Error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(
    stream: Arc<Mutex<TcpStream>>,
    addr: SocketAddr,
    state: Arc<Mutex<ChatState>>,
) -> Result<()> {
    info!("Handling connection: {}", addr);
    let (tx, mut rx) = mpsc::channel(MAX_CHANNEL_SIZE);
    state.lock().await.add_user(addr, tx);

    let stream_clone = stream.clone();
    tokio::spawn(async move {
        let mut stream = stream_clone.lock().await;
        while let Some(msg) = rx.recv().await {
            stream.write_all(msg.as_bytes()).await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    let mut buf = vec![0; 1024];
    let stream_clone2 = stream.clone();
    loop {
        let n = stream_clone2.lock().await.read(&mut buf).await?;
        info!("Read {} bytes", n);
        if n == 0 {
            info!("Connection closed: {}", addr);
            break;
        }

        let msg = String::from_utf8_lossy(&buf[..n]);
        info!("Received message: {}", msg);

        for (user_addr, tx) in state.lock().await.users.iter() {
            if *user_addr != addr {
                info!("Sending message to channel: {}", user_addr);
                tx.send(msg.to_string()).await?;
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
