use anyhow::{anyhow, Result};
use dns_starter_rust::{create_forwarder, parse_and_reply, Forwarder};
use std::net::SocketAddr;
use std::str::FromStr;
use std::{env, net::UdpSocket};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut resolver = None;
    if args.len() > 1 {
        if args.len() != 3 {
            return Err(anyhow!("invalid number of arguments, the only valid use of arguments is --resolver <address>"));
        }
        if &args[1] != "--resolver" {
            return Err(anyhow!(
                "invalid argument, the only valid use of arguments is --resolver <address>"
            ));
        }
        resolver = Some(SocketAddr::from_str(&args[2])?);
    }

    start_server(resolver)
}

fn start_server(resolver: Option<SocketAddr>) -> Result<()> {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053")?;
    let mut buf = [0; 512];
    let mut forwarder: Option<Forwarder> = None;
    loop {
        match (udp_socket.recv_from(&mut buf), resolver) {
            (Ok((size, source)), Some(addr_resolver)) => {
                println!("Received {} bytes from {} with resolver", size, source);
                match &mut forwarder {
                    Some(fw) => match fw.add_answer(&buf)? {
                        true => {
                            let reply = fw.build_reply();
                            udp_socket.send_to(&reply, fw.destination)?;
                            forwarder = None
                        }
                        false => {
                            let req = fw.forward()?;
                            udp_socket.send_to(&req, addr_resolver)?;
                        }
                    },
                    None => {
                        let mut fw = create_forwarder(&buf, source)?;
                        let req = fw.forward()?;
                        udp_socket.send_to(&req, addr_resolver)?;
                        forwarder = Some(fw);
                    }
                }
            }
            (Ok((size, source)), None) => {
                println!("Received {} bytes from {}", size, source);
                let response = parse_and_reply(&buf)?;
                udp_socket.send_to(&response, source)?;
            }
            (Err(e), _) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
    Ok(())
}
