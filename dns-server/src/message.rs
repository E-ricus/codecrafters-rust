mod answer;
mod header;
mod question;

use std::ops::Range;

use answer::ResourceRecord;
use header::Header;
use question::Question;

use anyhow::{anyhow, Result};

// Small macro to impl try from in enums repr
// Is it worth it to make it a proc macro to just derive it in each enum?
#[macro_export]
macro_rules! impl_try_from {
    ($enum_name:ident, $repr:ty, { $($variant:ident = $value:expr,)* }) => {
        impl TryFrom<$repr> for $enum_name {
            type Error = anyhow::Error;

            fn try_from(value: $repr) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $value => Ok(Self::$variant),
                    )*
                    _ => Err(anyhow::anyhow!("{} invalid value: {}", stringify!($enum_name) , value)),
                }
            }
        }
    };
}

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
            .copied()
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

impl_try_from!(Class, u16, {
    IN = 1,
    CS = 2,
    CH = 3,
    HS = 4,
});

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u16)]
enum Type {
    A = 1,      // host address
    NS = 2,     // NS: authorative name server
    MD = 3,     // MD: mail destination (obsolete)
    MF = 4,     // MF: mail forwarder (obsolete)
    CName = 5,  // CNAME: canonical name for alias
    Soa = 6,    // SOA: zone of a authority
    MB = 7,     // MB: Mail box domain
    MG = 8,     // // MG: Mail group member
    MR = 9,     // MR: Mail rename
    Null = 10,  // NULL
    Wks = 11,   // WKS: well known service description
    Ptr = 12,   // PTR: a domain name pointer
    HInfo = 13, // HINFO: host information
    MInfo = 14, // MINFO:  mailbox or mail list information
    MX = 15,    // MX: mail exchange
    Txt = 16,   // TXT: text strings
}

impl_try_from!(Type, u16, {
    A = 1,
    NS = 2,
    MD = 3,
    MF = 4,
    CName = 5,
    Soa = 6,
    MB = 7,
    MG = 8,
    MR = 9,
    Null = 10,
    Wks = 11,
    Ptr = 12,
    HInfo = 13,
    MInfo = 14,
    MX = 15,
    Txt = 16,
});

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct DNSMessage {
    pub(crate) header: Header,
    pub(crate) question: Option<Vec<Question>>,
    pub(crate) answer: Option<Vec<ResourceRecord>>,
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
    pub fn answers(&self) -> usize {
        self.header.an_count as usize
    }
    pub fn questions(&self) -> usize {
        self.header.qd_count as usize
    }
    pub fn from_bytes(buf: &[u8]) -> Result<Self> {
        if buf.len() < 12 {
            return Err(anyhow!(
                "invalid message: expecting at least 12 octets for the header."
            ));
        }
        let header_bytes = buf[0..12].try_into()?;
        let header = Header::from_bytes(header_bytes)?;
        println!("message id: {:?}", header.id);

        let mut raw = RawMessage::new(buf);
        // The 12 bytes of the header are already parsed
        raw.current_pos = 12;

        let question = if header.qd_count != 0 {
            let mut questions = Vec::with_capacity(header.qd_count as usize);
            for i in 0..header.qd_count {
                println!("parsing question: {}", i + 1);
                questions.push(Question::from_bytes(&mut raw)?)
            }
            Some(questions)
        } else {
            None
        };
        let answer = if header.an_count != 0 {
            let mut answers = Vec::with_capacity(header.an_count as usize);
            for i in 0..header.an_count {
                println!("parsing answer: {}", i + 1);
                answers.push(ResourceRecord::from_bytes(&mut raw)?)
            }
            Some(answers)
        } else {
            None
        };

        Ok(Self {
            header,
            question,
            answer,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.to_bytes());
        if let Some(questions) = &self.question {
            for q in questions {
                bytes.extend(q.to_bytes());
            }
        }
        if let Some(answer) = &self.answer {
            for rr in answer {
                bytes.extend(rr.to_bytes());
            }
        }
        bytes
    }

    pub fn build_reply(self) -> Self {
        let mut reply = Self {
            header: self.header.build_reply(),
            ..Default::default()
        };

        if let Some(questions) = &self.question {
            for q in questions {
                let rr = ResourceRecord::answer_by_type(q.qtype, &q.name);
                reply.add_answer(rr)
            }
        }
        reply.question = self.question;
        reply
    }

    pub(crate) fn add_answer(&mut self, rr: ResourceRecord) {
        self.header.an_count += 1;
        match &mut self.answer {
            Some(answers) => answers.push(rr),
            None => self.answer = Some(vec![rr]),
        }
    }
}

fn parse_labels(bytes: &mut RawMessage) -> Result<String> {
    let mut labels = vec![];
    let mut current = bytes.current_pos;
    let mut next_pointer = None;
    let mut jumps = 0;
    while let Ok(len_byte) = bytes.get(current) {
        let len = len_byte as usize;
        if len == 0 {
            current += 1;
            break;
        }
        if let Some(offset) = pointer(len_byte, bytes.get(current + 1)?) {
            if jumps == 0 {
                // Continues reading the question after finishing the labels
                next_pointer = Some(current + 2);
            }
            jumps += 1;
            if jumps > 5 {
                return Err(anyhow!("too many pointers jumps, max: 5"));
            }
            current = offset as usize;
            // Goes back to read the label from the offset
            continue;
        }
        current += 1;

        let label = bytes.get_range(current..current + len)?;
        labels.push(std::str::from_utf8(label)?);
        current += len;
    }

    let name = labels.join(".");
    // Why is rust okay for the mut borrow after another unrelated instruction but not just after the loop?
    // If the next_pointer is set, jumps to that value, otherwise continues with the current index
    bytes.current_pos = next_pointer.unwrap_or(current);
    Ok(name)
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
    use std::net::Ipv4Addr;

    use answer::Data;

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

    #[test]
    fn test_from_bytes_with_answer() -> Result<()> {
        let request: [u8; 512] = [
            219, 180, 129, 0, 0, 1, 0, 1, 0, 0, 0, 0, 12, 99, 111, 100, 101, 99, 114, 97, 102, 116,
            101, 114, 115, 2, 105, 111, 0, 0, 1, 0, 1, 12, 99, 111, 100, 101, 99, 114, 97, 102,
            116, 101, 114, 115, 2, 105, 111, 0, 0, 1, 0, 1, 0, 0, 14, 16, 0, 4, 76, 76, 21, 21, 0,
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
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let message = DNSMessage::from_bytes(&request)?;
        assert!(message.answer.is_some());
        let answer = message.answer.unwrap();
        assert_eq!(1, answer.len());
        assert_eq!("codecrafters.io", answer[0].name);
        assert_eq!(Data::IP(Ipv4Addr::new(76, 76, 21, 21)), answer[0].data);
        Ok(())
    }
}
