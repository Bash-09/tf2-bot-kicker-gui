use rcon::Connection;
use tokio::{runtime::Runtime, net::TcpStream};

pub struct CommandManager {
    runtime: Runtime,
    rcon: rcon::Result<Connection<TcpStream>>,
}