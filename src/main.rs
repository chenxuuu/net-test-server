//#![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::{sync::{mpsc::Receiver, Arc}, net::Ipv4Addr, collections::HashMap};

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

use serde::{Deserialize, Serialize};
use serde_json::json;

//获取资源文件路径
pub fn get_path() -> String{
    let path = std::env::args().nth(1).unwrap_or(String::from("web/"));
    if path.ends_with('/') || path.ends_with('\\') {
        path
    } else {
        path + "/"
    }
}

#[derive(Debug)]
enum Socket2Ws {
    Created(u16),           //tcp通道开启，带端口
    Connected(String,String),      //tcp客户端连上
    Disconnected(String),   //tcp客户端断开
    SocketMessage(String,Vec<u8>), //收到tcp消息
    WsMessage(String),      //收到ws消息
    Error(String),      //错误信息
    Quit,   //wx断开连接
}

//返回给客户端的 统一错误信息
#[derive(Serialize)]
struct WsError{
    action : String,
    msg : String
}
impl WsError {
    fn new(error: &str) -> WsError {
        WsError {
            action: String::from("error"),
            msg: String::from(error)
        }
    }
}

//返回给客户端的 响应新端口的申请
#[derive(Serialize)]
struct WsNewPort{
    action : String,
    port : u16
}
impl WsNewPort {
    fn new(port: u16) -> WsNewPort {
        WsNewPort {
            action: String::from("port"),
            port
        }
    }
}

//返回给客户端的 新客户端已建立连接
#[derive(Serialize)]
struct WsNewClient{
    action : String,
    client : String,
    addr : String,
}
impl WsNewClient {
    fn new(client: String, addr: String) -> WsNewClient {
        WsNewClient {
            action: String::from("connected"),
            client,
            addr
        }
    }
}

//返回给客户端的 新客户端已建立连接
#[derive(Serialize)]
struct WsDisconnectClient{
    action : String,
    client : String,
}
impl WsDisconnectClient {
    fn new(client: String) -> WsDisconnectClient {
        WsDisconnectClient {
            action: String::from("closed"),
            client
        }
    }
}

//返回给客户端的 收到客户端的数据(data)
#[derive(Serialize)]
struct WsRecvData{
    action : String,
    client : String,
    data: String,
    hex: bool
}
impl WsRecvData {
    fn new(client: String, data: String) -> WsRecvData {
        WsRecvData {
            action: String::from("data"),
            client,
            data,
            hex: true
        }
    }
}

//ws向tcp发消息的数据结构 发送数据到指定客户端(sendc)
#[derive(Deserialize)]
struct Wssendc {
    action: String,
    data: String,
    hex: bool,
    client: String,
}

lazy_static! {
    //存放已打开的端口
    static ref TCP_PORTS: Mutex<[bool;65535]> = {
        Mutex::new([false;65535])
    };
}

fn create_client_name() -> String {
    random_string::generate(6, "1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
}

use std::{fmt::Write, num::ParseIntError};
pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
pub fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

async fn handle_ws_client(websocket: warp::ws::WebSocket) {
    info!("new client");
    let (mut sender, mut receiver) = websocket.split();
    //给websocket通信用
    let (wt, mut wr) = tokio::sync::mpsc::channel(32);
    //给ws线程用
    let mut wt_ws = wt.clone();
    //开启tcp服务端用
    let (mut open_send, mut open_recv) = tokio::sync::mpsc::channel(1);
    //存储每个客户端的名字和关闭消息通道
    let clients : Arc<Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>> = Arc::new(Mutex::new(HashMap::new()));
    //存储每个客户端的名字和关闭消息通道
    let clients_data : Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<Vec<u8>>>>> = Arc::new(Mutex::new(HashMap::new()));
    //给ws线程里面用的
    let clients_ws = clients.clone();
    let clients_data_ws = clients_data.clone();
    tokio::spawn(async move {
        let mut port = 0;
        while let Some(message) = wr.recv().await {
            info!("received msg {:?}", message);
            match message {
                Socket2Ws::Created(s) => {
                    port = s;
                    sender.send(Message::text(json!(WsNewPort::new(s)).to_string())).await.unwrap_or(())
                }
                Socket2Ws::Connected(s,addr) => 
                    sender.send(Message::text(json!(WsNewClient::new(s,addr)).to_string())).await.unwrap_or(()),
                Socket2Ws::Disconnected(s) => {
                    {
                        let mut clients = clients_ws.lock().await;
                        match clients.remove(&s) {_=>()};
                        let mut clients = clients_data_ws.lock().await;
                        match clients.remove(&s) {_=>()};
                    }
                    sender.send(Message::text(json!(WsDisconnectClient::new(s)).to_string())).await.unwrap_or(());
                }
                Socket2Ws::SocketMessage(c,d) => {
                    let data = encode_hex(&d);
                    sender.send(Message::text(json!(WsRecvData::new(c,data)).to_string())).await.unwrap_or(());
                }
                Socket2Ws::WsMessage(s) => {
                    let r : serde_json::Value = match serde_json::from_str(&s) {
                        Ok(r) => r,
                        Err(_) => continue
                    };
                    if let Some(action) = r.get("action") {//没action就当心跳
                        let action = action.as_str().unwrap_or("");
                        match action {
                            "newp" => {//开新端口
                                if let Some(t) = r.get("type") {
                                    let t = t.as_str().unwrap_or("");
                                    if t == "tcp" {
                                        open_send.send(()).await.unwrap_or(());
                                    }
                                }
                            },
                            "closec" => {//关掉指定客户端
                                if let Some(t) = r.get("client") {
                                    let t = t.as_str().unwrap_or("");
                                    let mut clients = clients_ws.lock().await;
                                    if let Some(c) = &clients.get_mut(t){
                                        c.broadcast(true).unwrap_or(())
                                    }
                                }
                            },
                            "sendc" => {//发送数据到指定客户端
                                let r : Wssendc = match serde_json::from_str(&s) {
                                    Ok(r) => r,
                                    _ => continue
                                };
                                let data = if r.hex {
                                    match decode_hex(&r.data) {
                                        Ok(r) => r,
                                        _ => {
                                            wt_ws.send(Socket2Ws::Error(String::from("hex decode error!")))
                                            .await.unwrap_or(());
                                            continue
                                        }
                                    }
                                }else{
                                    r.data.to_owned().into_bytes()
                                };
                                let mut clients = clients_data_ws.lock().await;
                                if let Some(c) = &mut clients.get_mut(&r.client){
                                    c.send(data).await.unwrap_or(())
                                }
                            },
                            _ => (),
                        }
                    }
                }
                Socket2Ws::Error(s) => {
                    warn!("received error: {}",s);
                    sender.send(Message::text(json!(WsError::new(&s)).to_string())).await.unwrap_or(())
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
    //给主线程用
    let clients_s = clients.clone();
    let clients_data_s = clients_data.clone();
    tokio::spawn(async move {
        select! {//用select可以强退
            _ = async {
                open_recv.recv().await.unwrap();
                let mut port : u16 = 0;
                {
                    let mut ports = TCP_PORTS.lock().await;
                    for i in 50000..59999 {
                        if !ports[i] {
                            ports[i] = true;
                            port = i as u16;
                            break
                        }
                    }
                }
                if port == 0{
                    wts.send(Socket2Ws::Error(String::from("no more free port"))).await.unwrap_or(());
                }
                let mut listener = match TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), port)).await {
                    Ok(l) => {
                        wts.send(Socket2Ws::Created(port)).await.unwrap_or(());
                        l
                    },
                    Err(_) => {
                        wts.send(Socket2Ws::Error(String::from("open port failed"))).await.unwrap_or(());
                        return
                    }
                };
                loop {
                    let (mut socket, _) = match listener.accept().await {
                        Ok(l) => l,
                        Err(_) => continue,
                    };
                    let addr = socket.peer_addr().unwrap().to_string();//客户端地址
                    let (mut socket_read,mut socket_write) = socket.into_split();
                    let mut krs = kill_all_rx.clone();
                    let (kill_c_tx, mut kill_c_rx) = tokio::sync::watch::channel(false);
                    let clients = clients_s.clone();
                    let client = {
                        let mut clients = clients.lock().await;
                        let mut name;
                        loop {
                            name = create_client_name();
                            if clients.contains_key(&name) {
                                continue
                            }
                            clients.insert(name.clone(),kill_c_tx);
                            break
                        }
                        name
                    };

                    //收ws-->tcp消息用的
                    let (st, mut sr) = tokio::sync::mpsc::channel(32);
                    {
                        let mut clients_data = clients_data_s.lock().await;
                        clients_data.insert(client.clone(),st);
                    }

                    let mut wtc = wts.clone();      //给每个能关闭的都加上
                    let mut wt_send = wts.clone();      //给每个能关闭的都加上
                    let client_send = client.clone();  //给每个能关闭的都加上
                    let mut wta = wts.clone();      //给每个能关闭的都加上
                    let client_a = client.clone();  //给每个能关闭的都加上
                    let mut wtk = wts.clone();      //给每个能关闭的都加上
                    let client_k = client.clone();  //给每个能关闭的都加上
                    tokio::spawn(async move {
                        select! {
                            _ = async {
                                info!("new client connected");
                                wtc.send(Socket2Ws::Connected(client.clone(),addr))
                                    .await
                                    .unwrap_or(());
                                let mut buf = vec![0; 2048];
                                loop {
                                    match socket_read.read(&mut buf).await {
                                        Ok(0) => {
                                            info!("receive 0 length");
                                            break
                                        },
                                        Ok(n) => {
                                            info!("recv tcp msg");
                                            wtc.send(Socket2Ws::SocketMessage(client.clone(),(&buf[..n]).to_vec()))
                                                .await
                                                .unwrap_or(());
                                            // if socket.write_all(&buf[..n]).await.is_err() {
                                            //     break
                                            // }
                                        }
                                        Err(e) => {
                                            error!("read error! {:?}",e);
                                            break
                                        }
                                    }
                                }
                                info!("client disconnected by remote");
                                wtc.send(Socket2Ws::Disconnected(client)).await.unwrap_or(());
                            } => {}
                            _ = async {
                                while let Some(msg) = sr.recv().await {
                                    if socket_write.write_all(&msg).await.is_err() {
                                        break
                                    }
                                }
                                info!("client disconnected by remote");
                                wt_send.send(Socket2Ws::Disconnected(client_send)).await.unwrap_or(());
                            } => {}
                            _ = async {
                                while let Some(value) = krs.recv().await {
                                    if value {
                                        info!("client disconnected by all");
                                        wta.send(Socket2Ws::Disconnected(client_a)).await.unwrap_or(());
                                        return
                                    }
                                }
                            } => {}
                            _ = async {
                                while let Some(value) = kill_c_rx.recv().await {
                                    if value {
                                        info!("client disconnected by user");
                                        wtk.send(Socket2Ws::Disconnected(client_k)).await.unwrap_or(());
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
                //此处绝对不会panic，因为前面检查过了
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
    let web = warp::fs::dir(get_path());
    info!("ws://192.168.31.116:10241/ws/netlab");
    info!("{}",get_path());
    warp::serve(routes.or(web)).run(([0,0,0,0], 10240)).await;
}
