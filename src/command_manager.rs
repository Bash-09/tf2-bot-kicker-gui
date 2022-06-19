use rcon::Connection;
use tokio::{net::TcpStream, runtime::Runtime};

pub struct CommandManager {
    runtime: Runtime,
    rcon: rcon::Result<Connection<TcpStream>>,
}
