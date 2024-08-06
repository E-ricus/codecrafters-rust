mod answer;
mod header;
mod question;

use answer::ResourceRecord;
use header::Header;
use question::Question;

use anyhow::Result;

#[allow(dead_code)]
struct RawMessage<'a> {
    buffer: &'a [u8],
    current_pos: usize,
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
    answer: Option<Vec<ResourceRecord>>,
}

impl Default for DNSMessage {
    fn default() -> Self {
        let header = Header::default();
        let question = Question::default();
        Self {
            header,
            question,
            answer: None,
        }
    }
}

impl DNSMessage {
    pub fn from_buf(buf: &[u8]) -> Result<Self> {
        // Will be used to map the whole message easier
        let _raw = RawMessage::new(buf);
        let mut header = Header::default();
        let header_bytes = buf[0..12].try_into()?;
        header.read_bytes(header_bytes)?;
        // TODO: sending all bytes to the question
        let question = Question::from_bytes(&buf[12..])?;
        // TODO: do we need to parse an answer?
        Ok(Self {
            header,
            question,
            answer: None,
        })
    }

    fn add_answer(&mut self, rr: ResourceRecord) {
        match &mut self.answer {
            Some(answers) => answers.push(rr),
            None => self.answer = Some(vec![rr]),
        }
    }

    pub fn build_reply(&self) -> Self {
        let mut reply = Self::default();
        reply.header = self.header.build_reply();
        reply.question = self.question.clone();

        if let Some(rr) = ResourceRecord::answer_by_type(reply.question.qtype, &reply.question.name)
        {
            reply.add_answer(rr)
        }
        reply
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend(self.question.to_bytes());
        if let Some(answer) = self.answer {
            for rr in answer {
                bytes.extend(rr.to_bytes());
            }
        }
        bytes
    }
}
