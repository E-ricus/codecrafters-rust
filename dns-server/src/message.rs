mod header;
mod question;

use header::Header;
use question::Question;

use anyhow::Result;

#[allow(dead_code)]
struct RawMessage<'a> {
    buffer: &'a [u8],
    current_pos: usize,
}

impl<'a> RawMessage<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            current_pos: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DNSMessage {
    header: Header,
    question: Question,
}

impl Default for DNSMessage {
    fn default() -> Self {
        let header = Header::default();
        let question = Question::default();
        Self { header, question }
    }
}

impl DNSMessage {
    pub fn from_buf(buf: &[u8]) -> Result<Self> {
        // Will be used to map the whole message easier
        let _raw = RawMessage::new(buf);
        let mut header = Header::default();
        let header_bytes = buf[0..12].try_into()?;
        header.read_bytes(header_bytes)?;
        // TODO: sending all bytes
        let question = Question::from_bytes(&buf[12..])?;
        Ok(Self { header, question })
    }

    pub fn build_reply(&self) -> Self {
        let mut reply = Self::default();
        reply.header = self.header.build_reply();
        reply.question = self.question.clone();
        reply
    }

    // TODO: Only sending the header and question.
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend(self.question.to_bytes());
        bytes
    }
}
