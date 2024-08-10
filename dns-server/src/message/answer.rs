use core::str;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::OnceLock;

use anyhow::Result;

use super::{parse_labels, Class, RawMessage, Type};

#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) enum Data {
    None,
    IP(Ipv4Addr),
}
#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ResourceRecord {
    pub(super) name: String,
    pub(super) atype: Type,
    pub(super) class: Class,
    pub(super) ttl: u32, // The specification asks for a signed int, using a signed one for now.
    pub(super) length: u16,
    pub(super) data: Data, // RDATA
}

impl Default for ResourceRecord {
    fn default() -> Self {
        Self {
            name: "".to_string(),
            atype: Type::A,
            class: Class::IN,
            ttl: 0,
            length: 0,
            data: Data::None,
        }
    }
}

fn domains() -> &'static HashMap<&'static str, &'static str> {
    static DOMAINS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    DOMAINS.get_or_init(|| {
        [
            ("codecrafters.io", "8.8.8.8"),
            ("another.codecrafters.io", "1.1.1.1"),
        ]
        .iter()
        .copied()
        .collect()
    })
}

impl ResourceRecord {
    pub(super) fn answer_by_type(qtype: Type, name: &str) -> Self {
        match qtype {
            Type::A => {
                let ip = match domains().get(name) {
                    Some(ip) => Ipv4Addr::from_str(ip).expect("expected correct ip"),
                    None => Ipv4Addr::new(8, 8, 8, 8),
                };
                // I think that if a dns server doesn't have a domain it should not return it.
                Self {
                    name: name.to_string(),
                    atype: qtype,
                    class: Class::IN,
                    ttl: 60,
                    length: 4,
                    data: Data::IP(ip),
                }
            }
            _ => unimplemented!("not implemented"),
        }
    }

    pub(super) fn from_bytes(bytes: &mut RawMessage) -> Result<Self> {
        let name = parse_labels(bytes)?;
        let atype =
            u16::from_be_bytes(bytes.current_and_advance_range(2)?.try_into()?).try_into()?;
        let class =
            u16::from_be_bytes(bytes.current_and_advance_range(2)?.try_into()?).try_into()?;
        let ttl = u32::from_be_bytes(bytes.current_and_advance_range(4)?.try_into()?);
        let length = u16::from_be_bytes(bytes.current_and_advance_range(2)?.try_into()?);
        let data = bytes.current_and_advance_range(length as usize)?;
        let data = match atype {
            // Only mapping length 4
            // But in theory, it should be fine for a type A
            Type::A => Data::IP(Ipv4Addr::new(data[0], data[1], data[2], data[3])),
            // Unimplemented
            _ => Data::None,
        };

        Ok(Self {
            name,
            atype,
            class,
            ttl,
            length,
            data,
        })
    }

    pub(super) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.name.split('.').fold(Vec::new(), |mut bytes, label| {
            let len = label.len() as u8;
            bytes.push(len);
            bytes.extend_from_slice(label.as_bytes());
            bytes
        });
        // Add null termination
        bytes.push(0);

        let qtype = self.atype as u16;
        bytes.extend_from_slice(&qtype.to_be_bytes());
        let class = self.class as u16;
        bytes.extend_from_slice(&class.to_be_bytes());
        bytes.extend_from_slice(&self.ttl.to_be_bytes());
        bytes.extend_from_slice(&self.length.to_be_bytes());
        match self.data {
            Data::None => {}
            Data::IP(ip) => {
                bytes.extend_from_slice(&ip.octets());
            }
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_answer_by_type() {
        let expected_answer = ResourceRecord {
            name: "codecrafters.io".to_string(),
            atype: Type::A,
            class: Class::IN,
            ttl: 60,
            length: 4,
            data: Data::IP(Ipv4Addr::from_bits(0x08080808)),
        };
        let answer = ResourceRecord::answer_by_type(Type::A, "codecrafters.io");
        assert_eq!(expected_answer, answer);
    }

    #[test]
    fn test_rr_to_bytes() -> Result<()> {
        let answer = ResourceRecord {
            name: "codecrafters.io".to_string(),
            atype: Type::A,
            class: Class::IN,
            ttl: 60,
            length: 4,
            data: Data::IP(Ipv4Addr::from_bits(0x08080808)),
        };

        let bytes = answer.to_bytes();
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
        let array: [u8; 4] = [
            bytes[next_index + 5],
            bytes[next_index + 6],
            bytes[next_index + 7],
            bytes[next_index + 8],
        ];
        assert_eq!(60, u32::from_be_bytes(array));
        let array: [u8; 2] = [bytes[next_index + 9], bytes[next_index + 10]];
        assert_eq!(4, u16::from_be_bytes(array));
        assert_eq!(&[8, 8, 8, 8], &bytes[next_index + 11..]);
        Ok(())
    }

    #[test]
    fn test_from_bytes_uncompressed() -> Result<()> {
        let mut bytes: Vec<u8> = vec![12];
        bytes.extend_from_slice("codecrafters".as_bytes());
        bytes.push(2);
        bytes.extend_from_slice("io".as_bytes());
        // Null terminated label
        bytes.push(0);
        let typ: u16 = 1;
        bytes.extend_from_slice(&typ.to_be_bytes());
        let class: u16 = 1;
        bytes.extend_from_slice(&class.to_be_bytes());
        let ttl: u32 = 60;
        bytes.extend_from_slice(&ttl.to_be_bytes());
        let length: u16 = 4;
        bytes.extend_from_slice(&length.to_be_bytes());
        let data = [8, 8, 8, 8];
        bytes.extend_from_slice(&data);

        let mut raw = RawMessage::new(&bytes);
        let rr = ResourceRecord::from_bytes(&mut raw)?;
        assert_eq!("codecrafters.io".to_string(), rr.name);
        assert_eq!(Type::A, rr.atype);
        assert_eq!(Class::IN, rr.class);
        assert_eq!(ttl, rr.ttl);
        match rr.data {
            Data::None => panic!("data was not mapped"),
            Data::IP(ip) => assert_eq!(data, ip.octets()),
        }
        Ok(())
    }

    #[test]
    fn test_from_bytes_with_compression() -> Result<()> {
        let mut bytes: Vec<u8> = vec![12];
        // Another question in the message
        bytes.extend_from_slice("codecrafters".as_bytes());
        bytes.push(2);
        bytes.extend_from_slice("io".as_bytes());
        // Null terminated label
        bytes.push(0);
        let typ: u16 = 1;
        bytes.extend_from_slice(&typ.to_be_bytes());
        let ch: u16 = 1;
        bytes.extend_from_slice(&ch.to_be_bytes());

        // the question being parsed
        bytes.push(7);
        bytes.extend_from_slice("another".as_bytes());
        // Pointer byte
        bytes.push(0b11000000);
        // Offsite to the beginning of the slice
        bytes.push(0);
        let typ: u16 = 1;
        bytes.extend_from_slice(&typ.to_be_bytes());
        let class: u16 = 1;
        bytes.extend_from_slice(&class.to_be_bytes());
        let ttl: u32 = 60;
        bytes.extend_from_slice(&ttl.to_be_bytes());
        let length: u16 = 4;
        bytes.extend_from_slice(&length.to_be_bytes());
        let data = [1, 1, 1, 1];
        bytes.extend_from_slice(&data);

        let mut raw = RawMessage::new(&bytes);
        // Start of the question being parsed
        raw.current_pos = 21;
        let rr = ResourceRecord::from_bytes(&mut raw)?;
        assert_eq!("another.codecrafters.io".to_string(), rr.name);
        assert_eq!(Type::A, rr.atype);
        assert_eq!(Class::IN, rr.class);
        assert_eq!(ttl, rr.ttl);
        match rr.data {
            Data::None => panic!("data was not mapped"),
            Data::IP(ip) => assert_eq!(data, ip.octets()),
        }
        Ok(())
    }
}
