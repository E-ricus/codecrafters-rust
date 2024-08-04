use anyhow::Result;

#[derive(Debug, PartialEq)]
#[repr(u8)]
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

impl TryFrom<u8> for QType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
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

#[derive(Debug, PartialEq)]
#[repr(u8)]
enum Class {
    IN = 1, // IN: Internet
    CS = 2, // CSNET (obsolete)
    CH = 3, // CH: Chaos class
    HS = 4, // HS: Hesiod
}

impl TryFrom<u8> for Class {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::IN),
            2 => Ok(Self::CS),
            3 => Ok(Self::CH),
            4 => Ok(Self::HS),
            _ => Err(anyhow::anyhow!("invalid value")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct Question<'a> {
    // TODO: Remove pub
    pub(super) name: &'a str, // domain name
    qtype: QType,             // 2 bytes
    class: Class,             // 2 bytes
}

impl Default for Question<'_> {
    fn default() -> Self {
        Question {
            name: "",
            qtype: QType::A,
            class: Class::IN,
        }
    }
}

impl Question<'_> {
    pub(super) fn from_bytes(_bytes: Vec<u8>) -> Result<Self> {
        // let mut current: u8 = 0;
        // for byte in bytes {}
        unimplemented!("not implemented");
    }
    pub(super) fn to_bytes(self) -> Vec<u8> {
        let mut bytes = self.name.split('.').fold(Vec::new(), |mut bytes, label| {
            let len = label.len() as u8;
            bytes.push(len);
            bytes.extend_from_slice(label.as_bytes());
            bytes
        });
        bytes.push(self.qtype as u8);
        bytes.push(self.class as u8);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_to_bytes() {
        let mut question = Question::default();
        question.name = "codecrafters.io";

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

        assert_eq!(1, bytes[next_index]);
        assert_eq!(1, bytes[next_index + 1]);
    }
}
