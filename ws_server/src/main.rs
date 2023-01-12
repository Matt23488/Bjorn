fn main() {
    let mut server = ws_protocol::WsServer::new();

    server.wait();
}
