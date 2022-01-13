//#![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::sync::{mpsc::Receiver, Arc};

use futures_util::{FutureExt, StreamExt, SinkExt};
use tokio::{net::TcpListener, io::{AsyncReadExt, AsyncWriteExt}, sync::Mutex, select, time};
use warp::{Filter, ws::Message};

#[derive(Debug)]
enum Socket2Ws {
    Connected(String),
    Disconnected(String),
    SocketMessage(Vec<u8>),
    WsMessage(String),
    Quit,
}

#[derive(Debug)]
enum Ws2Socket {
    WsMessage(String),
    Kick(String),
    Quit,
}

async fn handle_ws_client(websocket: warp::ws::WebSocket) {
    info!("new client");
    let (mut sender, mut receiver) = websocket.split();
    let (wt, mut wr) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        while let Some(message) = wr.recv().await {
            info!("received msg {:?}",message);
            match message {
                Socket2Ws::Connected(s) => {
                    sender.send(Message::text(format!("connect: {}",s))).await.unwrap()
                },
                Socket2Ws::Disconnected(s) => {
                    sender.send(Message::text(format!("disconnect: {}",s))).await.unwrap()
                },
                Socket2Ws::SocketMessage(s) => {
                    //todo!()
                },
                Socket2Ws::WsMessage(s) => {
                    //todo!()
                },
                Socket2Ws::Quit => return,//断开了
            };
        }
    });

    //let (st, mut sr) = tokio::sync::broadcast::channel(32);
    let wts = wt.clone();
    let exit = Arc::new(Mutex::new(false));
    let es = exit.clone();
    tokio::spawn(async move {
        let mut listener = TcpListener::bind("127.0.0.1:23333").await.unwrap();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut wtc = wts.clone();
            let exitc = es.clone();
            tokio::spawn(async move {
                select! {
                    _ = async {
                        info!("new client connected");
                        wtc.send(Socket2Ws::Connected(String::from("test client"))).await.unwrap_or(());
                        let mut buf = vec![0; 1024];
                        loop {
                            if *exitc.lock().await {
                                info!("find exit!");
                                break
                            }
                            match socket.read(&mut buf).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    info!("recv tcp msg");
                                    wtc.send(Socket2Ws::SocketMessage((&buf[..n]).to_vec())).await.unwrap_or(());
                                    // if socket.write_all(&buf[..n]).await.is_err() {
                                    //     break
                                    // }
                                },
                                Err(_) => break,
                            }
                        }
                        info!("new client disconnected");
                        wtc.send(Socket2Ws::Disconnected(String::from("test client"))).await.unwrap_or(());
                    } => {}
                    _ = async {
                        while !*es.lock().await {
                            time::Delay(Duration::from_millis(10));
                        }
                    } => {}
                }
            });
            if *es.lock().await {
                info!("socket exit!");
                drop(listener);
                break
            }
        }
    });
    
    let mut wt = wt.clone();
    while let Some(body) = receiver.next().await {
        let message = match body {
            Ok(msg) => msg,
            Err(e) => {
                error!("error reading message on websocket: {}", e);
                break;
            }
        };
        if message.is_text() {
            wt.send(
                Socket2Ws::WsMessage(
                    String::from_utf8(message.into_bytes()).unwrap()
                )
            ).await.unwrap();
        }
    }
    {
        let mut exit = exit.lock().await;
        *exit = true;
    }
    wt.send(Socket2Ws::Quit).await.unwrap();

    info!("client disconnected");
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let routes = warp::path!("ws" / "netlab")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(handle_ws_client)
        });

    warp::serve(routes).run(([127, 0, 0, 1], 2333)).await;
}
