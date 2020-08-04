use std::net::{
    IpAddr,
    SocketAddr,
};

use std::{
    sync::Arc,
    time::Duration,
};

use crate::{
    actors::{
        self,
        GameOfLife,
    },
    deps::{
        futures_util::{
            future::{
                select,
                Either,
            },
            SinkExt,
            StreamExt,
        },
        rayon::{
            ThreadPool,
            ThreadPoolBuilder,
        },
        tokio::net::TcpStream,
        tokio_tungstenite::WebSocketStream,
    },
};

use crate::deps::{
    crossbeam::channel,
    gameoflife,
    locutus_actor::Actor,
    serde_json as json,
    tracing::{
        error,
        info,
        trace,
        warn,
        Level,
    },
    tungstenite::Message,
};

macro_rules! panic_on_err {
    ($fmt:expr, $($args:tt)*) => {{
        |err| {
            let message = format!("{}:{}: {} {}", err, module_path!(), line!(), format_args!($fmt, $($args)*));
            error!("{}", &message);
            panic!("[FATAL] {}", &message);
        }
    }};
    ($msg:expr,) => {{
        |err| {
            let message = format_args!("{}:{}: {} {}", err, module_path!(), line!(), $msg);
            error!("{}", &message);
            panic!("[FATAL] {}", &message);
        }
    }};
    ($msg:expr) => {{
        panic_on_err!($msg,)
    }};
    () => {{
        panic_on_err!("unrecoverable error encountered");
    }};
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Config {
    pub ip:                    IpAddr,
    pub port:                  u16,
    pub sim_threads:           usize,
    pub sim_thread_stack_size: usize,
    pub tick:                  Duration,
}

impl std::default::Default for Config {
    fn default() -> Self {
        Self {
            ip:                    IpAddr::from([127, 0, 0, 1]),
            port:                  9001,
            sim_threads:           16,
            sim_thread_stack_size: 2 << 20,
            tick:                  Duration::from_millis(33),
        }
    }
}

#[tracing::instrument]
pub async fn serve(config: Config) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use crate::deps::tokio::net::TcpListener;

    let sim_thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(config.sim_threads)
            .stack_size(config.sim_thread_stack_size)
            .build()?,
    );

    let addr = SocketAddr::new(config.ip, config.port);

    info!("Creating TcpListener on: {}", addr);
    let mut listener = TcpListener::bind(&addr).await.unwrap_or_else(panic_on_err!(
        "Cannot listen to {:?}, config: {:?}",
        addr,
        config
    ));
    info!("Listening on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _socketaddr)) => {
                let peer = stream
                    .peer_addr()
                    .unwrap_or_else(panic_on_err!("connected streams should have a peer address"));
                info!("Peer address: {}", peer);

                tokio::spawn(accept_connection(peer, stream, sim_thread_pool.clone(), config.tick));
            }
            Err(err) => {
                warn!("Shutting down server, error accepting connection - {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip(stream, sim_thread_pool))]
async fn accept_connection(
    peer: SocketAddr,
    stream: TcpStream,
    sim_thread_pool: Arc<ThreadPool>,
    tick: Duration,
) {
    let addr = stream
        .peer_addr()
        .unwrap_or_else(panic_on_err!("connected streams should have a peer address"));

    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .unwrap_or_else(panic_on_err!("Error during the websocket handshake occurred"));

    info!("New WebSocket connection: {}", addr);

    let actor = Arc::new(actors::GameOfLife::new());
    info!("Created simulation actor for connection: {:?}", actor);

    let actor_clone = actor.clone();
    sim_thread_pool.spawn(move || run_actor(actor_clone, tick));
    let connection_result = handle_connection(ws_stream, actor.clone(), tick).await;
    actor.send(gameoflife::Message::End);
    connection_result.unwrap_or_else(panic_on_err!(
        "Connection did not terminate gracefully: addr={}; actor={:?}",
        addr,
        actor
    ));
}

fn nano_now() -> u64 {
    ::std::time::SystemTime::now()
        .duration_since(::std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| panic!())
        .as_nanos() as u64
}

async fn handle_connection(
    websocket: WebSocketStream<TcpStream>,
    actor: Arc<actors::GameOfLife>,
    tick: Duration,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (mut outstream, mut instream) = websocket.split();
    let mut interval = tokio::time::interval(tick);
    let mut interval_future = interval.next();
    let mut message_future = instream.next();
    actor.send(gameoflife::Message::Random);
    actor.send(gameoflife::Message::Start);

    loop {
        match select(message_future, interval_future).await {
            Either::Left((msg, tick_fut_continue)) => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        info!("Received: {}", msg);
                        if msg.is_close() {
                            break;
                        } else {
                            let json_str = bincode::serialize(&*actor.state())?;
                            trace!("Sending: {} bytes", json_str.len());
                            let state_msg = Message::Binary(json_str);
                            outstream.send(state_msg).await?;
                        }
                        interval_future = tick_fut_continue; // Continue waiting for tick.
                        message_future = instream.next(); // Receive next WebSocket message.
                    }
                    None => {
                        error!("disconnected");

                        break;
                    } // WebSocket stream terminated.
                };
            }
            Either::Right((_, msg_fut_continue)) => {
                let json_str = bincode::serialize(&*actor.state())?;
                trace!("Sending: {} bytes", json_str.len());
                let state_msg = Message::Binary(json_str);
                outstream.send(state_msg).await?;
                message_future = msg_fut_continue; // Continue receiving the WebSocket message.
                interval_future = interval.next(); // Wait for next tick.
            }
        }
    }
    Ok(())
}

#[tracing::instrument]
fn run_actor(
    actor: Arc<GameOfLife>,
    tick: Duration,
) {
    let mut frames = 0;
    info!("starting new actor: GameOfLife::{}", actor.id());
    let ticker = channel::tick(tick);

    let mut start = nano_now();
    'update_loop: while let Ok(_tick) = ticker.recv() {
        actor.send(gameoflife::Message::Tick);

        if let Err(err) = actor.on_tick() {
            error!("game over: {:?}", err);
            break 'update_loop;
        }
        frames += 1;
        if frames % 120 == 0 {
            let now = nano_now();
            info!(
                "simulated {:>5} frames ({:.1} fps)",
                frames,
                120_000.0f64 / Duration::from_nanos(now - start).as_millis() as f64
            );
            start = now;
        }
    }
    info!("terminating actor: GameOfLife::{}", actor.id());
}
