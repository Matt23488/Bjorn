use ws_protocol::BjornWsServer;

fn main() {
    let server = BjornWsServer::new();

    server.join_connection_thread();
}
