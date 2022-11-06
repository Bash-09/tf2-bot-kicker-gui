use std::fmt::Display;

use rcon::{Connection, Error};
use tokio::{net::TcpStream, runtime::Runtime};

#[derive(Debug)]
pub enum KickReason {
    None,
    Idle,
    Cheating,
    Scamming,
}

impl Display for KickReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                KickReason:: None => "other",
                KickReason::Idle => "idle",
                KickReason::Cheating => "cheating",
                KickReason::Scamming => "scamming",
            }
        )
    }
}

pub struct CommandManager {
    runtime: Runtime,
    rcon: Option<Connection<TcpStream>>,
}

pub const CMD_STATUS: &str = "status";
pub const CMD_TF_LOBBY_DEBUG: &str = "tf_lobby_debug";

impl CommandManager {
    pub fn new(password: &str) -> CommandManager {
        let runtime = Runtime::new().unwrap();

        let mut rcon = None;
        runtime.block_on(async {
            rcon = Some(Connection::connect("127.0.0.1:27015", password).await);
        });

        CommandManager {
            runtime,
            rcon: rcon.unwrap().ok(),
        }
    }

    /// Checks if a valid rcon connection is currently established
    pub fn connected(&mut self, password: &str) -> Result<(), Error> {
        let mut connected = Ok(());

        self.runtime.block_on(async {
            match &mut self.rcon {
                Some(con) => match con.cmd("echo Ping").await {
                    Ok(_) => {}
                    Err(e) => {
                        connected = Err(e);
                        self.rcon = None;
                    }
                },
                None => match Connection::connect("127.0.0.1:27015", password).await {
                    Ok(con) => {
                        self.rcon = Some(con);
                    }
                    Err(e) => {
                        connected = Err(e);
                        self.rcon = None;
                    }
                },
            }
        });
        connected
    }

    pub fn run_command(&mut self, command: &str) -> Option<String> {
        let mut out = None;
        log::debug!("Running command \"{}\"", command);

        self.runtime.block_on(async {
            if let Some(rcon) = &mut self.rcon {
                if let Ok(response) = rcon.cmd(command).await {
                    out = Some(response);
                }
            }
        });
        out
    }

    pub fn kick_player(&mut self, player_userid: &str, reason: KickReason) -> Option<String> {
        let command = format!("callvote kick \"{}\" \"{}\"", player_userid, reason);
        self.run_command(&command)
    }

    pub fn send_chat(&mut self, message: &str) -> Option<String> {
        self.run_command(&format!("say \"{}\"", message))
    }
}
