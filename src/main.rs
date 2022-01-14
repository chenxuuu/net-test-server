//#![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::sync::{mpsc::Receiver, Arc};

use futures_util::{FutureExt, SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    select,
    sync::Mutex,
    time,
};
use warp::{ws::Message, Filter};

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
            info!("received msg {:?}", message);
            match message {
                Socket2Ws::Connected(s) => sender
                    .send(Message::text(format!("connect: {}", s)))
                    .await
                    .unwrap(),
                Socket2Ws::Disconnected(s) => sender
                    .send(Message::text(format!("disconnect: {}", s)))
                    .await
                    .unwrap(),
                Socket2Ws::SocketMessage(s) => {
                    //todo!()
                }
                Socket2Ws::WsMessage(s) => {
                    //todo!()
                }
                Socket2Ws::Quit => return, //断开了
            };
        }
    });

    let (kill_all_tx, kill_all_rx) = tokio::sync::watch::channel(false);
    let wts = wt.clone();
    let mut krm = kill_all_rx.clone();
    tokio::spawn(async move {
        select! {
            _ = async {
                let mut listener = TcpListener::bind("127.0.0.1:23333").await.unwrap();
                loop {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mut wtc = wts.clone();
                    let mut krs = kill_all_rx.clone();
                    tokio::spawn(async move {
                        select! {
                            _ = async {
                                info!("new client connected");
                                wtc.send(Socket2Ws::Connected(String::from("test client")))
                                    .await
                                    .unwrap_or(());
                                let mut buf = vec![0; 1024];
                                loop {
                                    match socket.read(&mut buf).await {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            info!("recv tcp msg");
                                            wtc.send(Socket2Ws::SocketMessage((&buf[..n]).to_vec()))
                                                .await
                                                .unwrap_or(());
                                            // if socket.write_all(&buf[..n]).await.is_err() {
                                            //     break
                                            // }
                                        }
                                        Err(_) => break,
                                    }
                                }
                                info!("new client disconnected");
                                wtc.send(Socket2Ws::Disconnected(String::from("test client")))
                                    .await
                                    .unwrap_or(());
                            } => {}
                            _ = async {
                                while let Some(value) = krs.recv().await {
                                    println!("received = {:?}", value);
                                    if value {
                                        return
                                    }
                                }
                            } => {}
                        }
                    });
                }
            } => {}
            _ = async {
                while let Some(value) = krm.recv().await {
                    println!("received = {:?}", value);
                    if value {
                        return
                    }
                }
            } => {}
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
            wt.send(Socket2Ws::WsMessage(
                String::from_utf8(message.into_bytes()).unwrap(),
            ))
            .await
            .unwrap();
        }
    }
    kill_all_tx.broadcast(true).unwrap();
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
