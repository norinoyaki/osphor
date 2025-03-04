use std::net::UdpSocket;
use tokio::sync::mpsc;

pub struct Packets {}

impl Packets {
    pub async fn init(address: String) {
        let udp_socket = UdpSocket::bind(&address).expect("Couldn't bind UDP socket");
        println!("UDP listening on {}", address);

        let (tx, mut rx) = mpsc::channel::<String>(10);
        tokio::spawn(async move {
            let mut buf = [0; 256];
            loop {
                match udp_socket.recv_from(&mut buf) {
                    Ok((amt, src)) => {
                        let msg = String::from_utf8_lossy(&buf[..amt]).to_string();
                        println!("Received from {}: {}", src, msg);

                        if tx.send(msg.clone()).await.is_err() {
                            eprintln!("Receiver dropped, stopping UDP task.");
                            break;
                        }

                        udp_socket
                            .send_to(&buf[..amt], &src)
                            .expect("Couldn't send data");
                    }
                    Err(e) => eprintln!("UDP error: {}", e),
                }
            }
        });

        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                println!("Processing message: {}", msg);
            }
        });
    }
}
