use std::net::SocketAddr;

use anyhow::Result;
use message::DNSMessage;

mod message;

#[derive(Debug, PartialEq, Clone)]
pub struct Forwarder {
    pub destination: SocketAddr,
    message: DNSMessage,
}

impl Forwarder {
    // Returns the bytes representing the DNS Message with the next question
    // If it is the last question to send, the forwarder marks is_complete as true
    pub fn forward(&mut self) -> Result<Vec<u8>> {
        let mut message = DNSMessage::default();
        message.header = self.message.header;
        // it forwards one question at a time.
        // This is a codecrafters requirement.
        message.header.qd_count = 1;
        if let Some(q) = &self.message.question {
            let question = q
                .get(self.message.answers())
                .expect("invalid questions lenght");
            message.question = Some(vec![question.clone()]);
        }
        Ok(message.to_bytes())
    }

    // Add the received answer from the resolver to the current response
    // If the answers now match the questions from the request, the forwarder is complete and returns true
    // Otherwise returns false indicating the need to keep forwarding
    pub fn add_answer(&mut self, buf: &[u8]) -> Result<bool> {
        let reply = DNSMessage::from_bytes(buf)?;
        match reply.answer {
            Some(mut ans) => {
                let answer = ans.remove(0);
                self.message.add_answer(answer);
                Ok(self.message.questions() == self.message.answers())
            }
            // Just finish the forwarder. (no questions no answers)
            _ => Ok(true),
        }
    }

    pub fn build_reply(&mut self) -> Vec<u8> {
        self.message.header = self.message.header.build_reply();
        self.message.to_bytes()
    }
}

// Parses the buffer as a DNS message, and the builds the reply with the local data.
pub fn parse_and_reply(buf: &[u8]) -> Result<Vec<u8>> {
    let message = DNSMessage::from_bytes(buf)?;
    Ok(message.build_reply().to_bytes())
}

pub fn create_forwarder(buf: &[u8], destination: SocketAddr) -> Result<Forwarder> {
    let request = DNSMessage::from_bytes(buf)?;
    Ok(Forwarder {
        destination,
        message: request,
    })
}
