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
pub struct DNSMessage<'a> {
    header: Header,
    question: Question<'a>,
}

impl Default for DNSMessage<'_> {
    fn default() -> Self {
        let header = Header::default();
        let question = Question::default();
        Self { header, question }
    }
}

impl DNSMessage<'_> {
    pub fn from_buf(buf: &[u8]) -> Result<Self> {
        // Will be used to map the whole message easier
        let _raw = RawMessage::new(buf);
        let mut header = Header::default();
        // TODO: Parse question from message
        let question = Question::default();
        let header_bytes = buf[0..12].try_into()?;
        header.read_bytes(header_bytes)?;
        Ok(Self { header, question })
    }

    pub fn build_reply(&self) -> Self {
        let mut reply = Self::default();
        reply.header = self.header.build_reply();
        // TODO: Harcoding for now, when question is parsed it should be removed.
        reply.question.name = "codefracters.io";
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
