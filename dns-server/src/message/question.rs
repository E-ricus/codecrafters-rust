use anyhow::Result;
use std::str;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u16)]
enum QType {
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

impl TryFrom<u16> for QType {
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
            _ => Err(anyhow::anyhow!("invalid value")),
        }
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
            _ => Err(anyhow::anyhow!("invalid value")),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Question {
    // TODO: Remove pub
    pub(super) name: String, // domain name
    qtype: QType,            // 2 bytes
    class: Class,            // 2 bytes
}

impl Default for Question {
    fn default() -> Self {
        Question {
            name: "".to_string(),
            qtype: QType::A,
            class: Class::IN,
        }
    }
}

impl Question {
    pub(super) fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut current = 1;
        let mut labels = vec![];
        let mut len = bytes[0] as usize;

        while len != 0 {
            let label = &bytes[current..current + len];
            labels.push(str::from_utf8(label)?);
            current += len + 1;
            len = bytes[current - 1] as usize;
        }
        let name = labels.join(".");
        let qtype: QType =
            u16::from_be_bytes(bytes[current..current + 2].try_into()?).try_into()?;
        let class: Class =
            u16::from_be_bytes(bytes[current + 2..current + 4].try_into()?).try_into()?;

        Ok(Self { name, qtype, class })
    }

    pub(super) fn to_bytes(self) -> Vec<u8> {
        let mut bytes = self.name.split('.').fold(Vec::new(), |mut bytes, label| {
            let len = label.len() as u8;
            bytes.push(len);
            bytes.extend_from_slice(label.as_bytes());
            bytes
        });
        // Add null termination
        bytes.push(0);

        let qtype = self.qtype as u16;
        bytes.extend_from_slice(&qtype.to_be_bytes());
        let class = self.class as u16;
        bytes.extend_from_slice(&class.to_be_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_from_bytes() -> Result<()> {
        let mut bytes: Vec<u8> = vec![12];
        bytes.extend_from_slice("codecrafters".as_bytes());
        bytes.push(2);
        bytes.extend_from_slice("io".as_bytes());
        bytes.push(0);
        let cname: u16 = 5;
        bytes.extend_from_slice(&cname.to_be_bytes());
        let ch: u16 = 3;
        bytes.extend_from_slice(&ch.to_be_bytes());

        let q = Question::from_bytes(&bytes)?;
        assert_eq!("codecrafters.io".to_string(), q.name);
        assert_eq!(QType::CName, q.qtype);
        assert_eq!(Class::CH, q.class);
        Ok(())
    }

    #[test]
    fn test_question_to_bytes() -> Result<()> {
        let mut question = Question::default();
        question.name = "codecrafters.io".to_string();

        let bytes = question.to_bytes();
        let len = bytes[0];
        assert_eq!(12, len);
        let len_hex = format!("{:#02x}", len);
        assert_eq!("0xc", len_hex);
        let next_index = (len + 1) as usize;
        let label = String::from_utf8_lossy(&bytes[1..next_index]);
        assert_eq!("codecrafters", label);

        let len = bytes[next_index];
        assert_eq!(2, len);
        let len_hex = format!("{:#02x}", len);
        assert_eq!("0x2", len_hex);
        let curr_index = next_index + 1;
        let next_index = (curr_index as u8 + len) as usize;
        let label = String::from_utf8_lossy(&bytes[curr_index..next_index]);
        assert_eq!("io", label);

        // null termination
        assert_eq!(0, bytes[next_index]);
        let array: [u8; 2] = [bytes[next_index + 1], bytes[next_index + 2]];
        assert_eq!(1, u16::from_be_bytes(array));
        let array: [u8; 2] = [bytes[next_index + 3], bytes[next_index + 4]];
        assert_eq!(1, u16::from_be_bytes(array));
        Ok(())
    }
}
