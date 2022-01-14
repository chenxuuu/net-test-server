//#![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::{sync::{mpsc::Receiver, Arc}, net::Ipv4Addr};

use futures_util::{FutureExt, SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    select,
    sync::Mutex,
    time,
};
use warp::{ws::Message, Filter};
use lazy_static::*;

#[derive(Debug)]
enum Socket2Ws {
    Created(u16),           //tcp通道开启，带端口
    Connected(String),      //tcp客户端连上
    Disconnected(String),   //tcp客户端断开
    SocketMessage(Vec<u8>), //收到tcp消息
    WsMessage(String),      //收到ws消息
    Quit,   //wx断开连接
}

#[derive(Debug)]
enum Ws2Socket {
    WsMessage(String),  //收到ws消息
    Kick(String),       //某客户端要被踢了
    Quit,               //完全退出
}

lazy_static! {
    //存放已打开的端口
    static ref TCP_PORTS: Mutex<[bool;65535]> = {
        Mutex::new([false;65535])
    };
}

async fn handle_ws_client(websocket: warp::ws::WebSocket) {
    info!("new client");
    let (mut sender, mut receiver) = websocket.split();
    //给websocket通信用
    let (wt, mut wr) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        let mut port = 0;
        while let Some(message) = wr.recv().await {
            info!("received msg {:?}", message);
            match message {
                Socket2Ws::Created(s) => {
                    port = s;
                    sender.send(Message::text(format!("port: {}", s)))
                    .await
                    .unwrap()
                }
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
                Socket2Ws::Quit => {
                    if port != 0 {
                        let mut ports = TCP_PORTS.lock().await;
                        ports[port as usize] = false;
                    }
                    return
                }, //断开了
            };
        }
    });

    //完全退出tcp服务端
    let (kill_all_tx, kill_all_rx) = tokio::sync::watch::channel(false);
    //tcp给ws发消息用
    let mut wts = wt.clone();
    //给tcp服务端主线程用
    let mut krm = kill_all_rx.clone();
    tokio::spawn(async move {
        select! {//用select可以强退
            _ = async {
                let mut port : u16 = 0;
                {
                    let mut ports = TCP_PORTS.lock().await;
                    for i in 10000..65535 {
                        if !ports[i] {
                            ports[i] = true;
                            port = i as u16;
                            break
                        }
                    }
                }
                if port == 0{
                    //fail todo
                }
                let mut listener = match TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), port)).await {
                    Ok(l) => {
                        wts.send(Socket2Ws::Created(port)).await.unwrap_or(());
                        l
                    },
                    Err(_) => {
                        //fail todo
                        return
                    }
                };
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
            _ = async {//强退任务用，别的同理
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
            .unwrap_or(());
        }
    }
    kill_all_tx.broadcast(true).unwrap_or(());
    wt.send(Socket2Ws::Quit).await.unwrap_or(());

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
    info!("ws://127.0.0.1:2333/ws/netlab");
    warp::serve(routes).run(([127, 0, 0, 1], 2333)).await;
}
