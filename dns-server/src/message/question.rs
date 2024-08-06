use anyhow::Result;
use std::str;

use super::{Class, Type};

#[derive(Debug, PartialEq, Clone)]
pub(super) struct Question {
    pub(super) name: String, // domain name
    pub(super) qtype: Type,  // 2 bytes
    pub(super) class: Class, // 2 bytes
}

impl Default for Question {
    fn default() -> Self {
        Question {
            name: "".to_string(),
            qtype: Type::A,
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
        let qtype: Type = u16::from_be_bytes(bytes[current..current + 2].try_into()?).try_into()?;
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
        assert_eq!(Type::CName, q.qtype);
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
