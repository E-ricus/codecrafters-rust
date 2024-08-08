mod answer;
mod header;
mod question;

use std::ops::Range;

use answer::ResourceRecord;
use header::Header;
use question::Question;

use anyhow::{anyhow, Result};

// Small wrapper to keep track of the current position while parsing.
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
    fn get(&self, n: usize) -> Result<u8> {
        self.buffer
            .get(n)
            .map(|v| *v)
            .ok_or(anyhow!("invalid index: {n}"))
    }

    // get the range without updating the current pointer
    fn get_range(&self, range: Range<usize>) -> Result<&[u8]> {
        self.buffer.get(range).ok_or(anyhow!("invalid range"))
    }

    // updates the current pointer
    fn current_and_advance_range(&mut self, n: usize) -> Result<&[u8]> {
        if self.current_pos + n > self.buffer.len() {
            return Err(anyhow!("the {n} exceeds the size of the buffer"));
        }
        let next = self
            .buffer
            .get(self.current_pos..self.current_pos + n)
            .ok_or(anyhow!("invalid range"));
        self.current_pos += n;
        next
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u16)]
enum Class {
    IN = 1, // IN: Internet
    CS = 2, // CSNET (obsolete)
    CH = 3, // CH: Chaos class
    HS = 4, // HS: Hesiod
}

impl TryFrom<u16> for Class {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::IN),
            2 => Ok(Self::CS),
            3 => Ok(Self::CH),
            4 => Ok(Self::HS),
            _ => Err(anyhow::anyhow!("Class: invalid value: {value}")),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u16)]
enum Type {
    A = 1,      // host address
    NS = 2,     // NS: authorative name server
    MD = 3,     // MD: mail destination (obsolete)
    MF = 4,     // MF: mail forwarder (obsolete)
    CName = 5,  // CNAME: canonical name for alias
    SOA = 6,    // SOA: zone of a authority
    MB = 7,     // MB: Mail box domain
    MG = 8,     // // MG: Mail group member
    MR = 9,     // MR: Mail rename
    Null = 10,  // NULL
    WKS = 11,   // WKS: well known service description
    PTR = 12,   // PTR: a domain name pointer
    HInfo = 13, // HINFO: host information
    MInfo = 14, // MINFO:  mailbox or mail list information
    MX = 15,    // MX: mail exchange
    TXT = 16,   // TXT: text strings
}

impl TryFrom<u16> for Type {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::A),
            2 => Ok(Self::NS),
            3 => Ok(Self::MD),
            4 => Ok(Self::MF),
            5 => Ok(Self::CName),
            6 => Ok(Self::SOA),
            7 => Ok(Self::MB),
            8 => Ok(Self::MG),
            9 => Ok(Self::MR),
            10 => Ok(Self::Null),
            11 => Ok(Self::WKS),
            12 => Ok(Self::PTR),
            13 => Ok(Self::HInfo),
            14 => Ok(Self::MInfo),
            15 => Ok(Self::MX),
            16 => Ok(Self::TXT),
            _ => Err(anyhow::anyhow!("Type: invalid value: {value}")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DNSMessage {
    header: Header,
    question: Option<Vec<Question>>,
    answer: Option<Vec<ResourceRecord>>,
}

impl Default for DNSMessage {
    fn default() -> Self {
        let header = Header::default();
        Self {
            header,
            question: None,
            answer: None,
        }
    }
}

impl DNSMessage {
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 12 {
            return Err(anyhow!(
                "invalid message: expecting at least 12 octets for the header."
            ));
        }
        println!("{:?}", buf);
        let header_bytes = buf[0..12].try_into()?;
        let header = Header::from_bytes(header_bytes)?;

        let mut raw = RawMessage::new(&buf);
        // The 12 bytes of the header are already parsed
        raw.current_pos = 12;
        let mut questions = Vec::with_capacity(header.qd_count as usize);
        for _ in 0..header.qd_count {
            questions.push(Question::from_bytes(&mut raw)?)
        }
        Ok(Self {
            header,
            question: Some(questions),
            answer: None,
        })
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_bytes());
        if let Some(questions) = self.question {
            for q in questions {
                bytes.extend(q.to_bytes());
            }
        }
        if let Some(answer) = self.answer {
            for rr in answer {
                bytes.extend(rr.to_bytes());
            }
        }
        bytes
    }

    pub fn build_reply(self) -> Self {
        let mut reply = Self::default();
        reply.header = self.header.build_reply();

        if let Some(questions) = &self.question {
            for q in questions {
                let rr = ResourceRecord::answer_by_type(q.qtype, &q.name);
                reply.add_answer(rr)
            }
        }
        reply.question = self.question;
        reply
    }

    fn add_answer(&mut self, rr: ResourceRecord) {
        match &mut self.answer {
            Some(answers) => answers.push(rr),
            None => self.answer = Some(vec![rr]),
        }
    }
}

// returns the index to start reading the label from if it is a pointer
// otherwise, returns None
fn pointer(byte: u8, next: u8) -> Option<u16> {
    if byte != 0b11000000 {
        return None;
    }
    let pointer = ((byte as u16) << 8) | (next as u16);
    // XORing with this mask to remove the 11 in the more significant bits indicating the pointer and get the correct offset
    Some(pointer ^ 0xC000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer() {
        let b1 = 0b11000000;
        let b2 = 0b00001100;

        let p = pointer(b1, b2);
        assert!(p.is_some());
        assert_eq!(12, p.unwrap());

        let b1 = 0b00000000;
        let b2 = 0b00001100;

        let p = pointer(b1, b2);
        assert!(p.is_none());
    }

    #[test]
    fn test_from_bytes_uncompressed() -> Result<()> {
        let request: [u8; 512] = [
            118, 24, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 12, 99, 111, 100, 101, 99, 114, 97, 102, 116,
            101, 114, 115, 2, 105, 111, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let message = DNSMessage::from_bytes(&request)?;
        assert!(message.question.is_some());
        let question = message.question.unwrap();
        assert_eq!(1, question.len());
        assert_eq!("codecrafters.io", question[0].name);
        Ok(())
    }

    #[test]
    fn test_from_bytes_compressed() -> Result<()> {
        let request: [u8; 512] = [
            219, 56, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 3, 97, 98, 99, 17, 108, 111, 110, 103, 97, 115,
            115, 100, 111, 109, 97, 105, 110, 110, 97, 109, 101, 3, 99, 111, 109, 0, 0, 1, 0, 1, 3,
            100, 101, 102, 192, 16, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];
        let message = DNSMessage::from_bytes(&request)?;
        assert!(message.question.is_some());
        let question = message.question.unwrap();
        assert_eq!(2, question.len());
        assert_eq!("abc.longassdomainname.com", question[0].name);
        assert_eq!("def.longassdomainname.com", question[1].name);
        Ok(())
    }
}
