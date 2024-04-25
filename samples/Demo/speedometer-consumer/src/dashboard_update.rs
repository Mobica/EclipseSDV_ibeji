use log::{info};
use tokio::task::JoinHandle;
use mio_extras::timer::Timeout;
use ws::util::Token;
use ws::{listen, CloseCode, Error, ErrorKind, Handler, Message, Sender, Handshake };
use serde::{Deserialize, Serialize};

const PING: Token = Token(1);
const PING_INTERVAL: u64 = 300;

pub static mut current_vehicle_speed: i32 = 0;

#[derive(Debug,Deserialize,Serialize)]
struct VehicleState{
    speed: i32,
    rpm: u32,
    gear: u8
}

pub async fn update_dashboard() -> Result<JoinHandle<()>, String>
{
    struct Server {
        out: Sender,
        ping_timeout: Option<Timeout>
    }

    impl Handler for Server {
        fn on_message(&mut self, msg: Message) -> ws::Result<()> {
            info!("Server got message '{}'. ", msg);
            Ok(())
        }

        fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
            self.out.timeout(PING_INTERVAL, PING)
        }

        fn on_close(&mut self, code: CloseCode, reason: &str) {
            info!("WebSocket closing for ({:?}) {}", code, reason);
            info!("Shutting down server after first connection closes.");
            self.out.shutdown().unwrap();
        }

        fn on_timeout(&mut self, event: Token) -> ws::Result<()> {
            match event {
                // PING timeout has occured, send a msg and reschedule
                PING => {
                    unsafe{
                        let data_to_dashboard = VehicleState{speed: current_vehicle_speed, rpm: 0, gear: 0};
                        let msg = serde_json::to_string(&data_to_dashboard).unwrap();
                        let _ = self.out.send(Message::text(msg));
                    }
                    self.ping_timeout.take();
                    self.out.timeout(PING_INTERVAL, PING)
                }
                // default - No other timeouts are possible
                _ => Err(Error::new(
                    ErrorKind::Internal,
                    "Invalid timeout token encountered!",
                )),
            }
        }

        fn on_new_timeout(&mut self, event: Token, timeout: Timeout) -> ws::Result<()> {
            // Cancel the old timeout and replace.
            if event == PING {
                // This ensures there is only one ping timeout at a time
                if let Some(t) = self.ping_timeout.take() {
                    self.out.cancel(t)?
                }
                self.ping_timeout = Some(timeout)
            }

            Ok(())
        }
    }

    // Server thread
    let server = tokio::spawn(async move  {
        listen("127.0.0.1:8000", |out| {
            Server { out, ping_timeout: None }
        }).unwrap();
    });

    Ok(server)
}