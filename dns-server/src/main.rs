mod message;

use anyhow::Result;
use message::DNSMessage;
use std::net::UdpSocket;

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let udp_socket = UdpSocket::bind("127.0.0.1:2053")?;
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);
                let message = DNSMessage::from_bytes(&buf)?;
                let response = message.build_reply().to_bytes();

                udp_socket.send_to(&response, source)?;
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
    Ok(())
}
