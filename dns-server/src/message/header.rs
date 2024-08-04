use anyhow::Result;

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
enum MessageType {
    Query = 0,
    Response = 1,
}

impl TryFrom<u8> for MessageType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Query),
            1 => Ok(Self::Response),
            _ => Err(anyhow::anyhow!("invalid value")),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
enum OpCode {
    Query = 0,
    IQuery = 1,
    Status = 2,
}

// TODO: Maybe a small macro to implement this for all enums?
impl TryFrom<u8> for OpCode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Query),
            1 => Ok(Self::IQuery),
            2 => Ok(Self::Status),
            _ => Err(anyhow::anyhow!("invalid value")),
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
enum ResponseCode {
    NoError = 0,
    FormatError = 1,
    ServerFailure = 2,
    NameError = 3,
    NotImplemented = 4,
    Refused = 5,
}

impl TryFrom<u8> for ResponseCode {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NoError),
            1 => Ok(Self::FormatError),
            2 => Ok(Self::ServerFailure),
            3 => Ok(Self::NameError),
            4 => Ok(Self::NotImplemented),
            5 => Ok(Self::Refused),
            _ => Err(anyhow::anyhow!("invalid value")),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Header {
    // TODO: pub for now, maybe move creating the reply header here.
    id: u16,                     // ID: 16 bits big endian
    message_type: MessageType,   // QR: 1 bit
    op_code: OpCode,             // OPCODE: 4 bits
    auth_answer: bool,           // AA (The response server owns the domain): 1 bit
    truncation: bool,            // TC: 1 bit
    recursion_desired: bool,     // RD: 1 bit
    recursion_available: bool,   // RA: 1 bit
    z: u8,                       // reserverd: 3 bits
    response_code: ResponseCode, // RCODE: 4 bits
    pub(super) qd_count: u16,    // QDCOUNT: 16 bits big endian
    an_count: u16,               // ANCOUNT: 16 bits big endian
    ns_count: u16,               // NSCOUNT: 16 bits big endian
    ar_count: u16,               // ARCOUNT : 16 bits big endian
}

impl Default for Header {
    fn default() -> Self {
        Self {
            id: 0,
            message_type: MessageType::Query,
            op_code: OpCode::Query,
            auth_answer: false,
            truncation: false,
            recursion_desired: false,
            recursion_available: false,
            z: 0,
            response_code: ResponseCode::NoError,
            qd_count: 0,
            an_count: 0,
            ns_count: 0,
            ar_count: 0,
        }
    }
}

impl Header {
    pub(super) fn read_bytes(&mut self, buf: [u8; 12]) -> Result<()> {
        let id: [u8; 2] = buf[0..2].try_into()?;
        self.id = u16::from_be_bytes(id);

        let bit_qr = (buf[2] & 0b10000000) >> 7;
        self.message_type = bit_qr.try_into()?;
        let bits_op = (buf[2] & 0b01111000) >> 3;
        self.op_code = bits_op.try_into()?;
        self.auth_answer = (buf[2] & 0b00000100) >> 2 != 0;
        self.truncation = (buf[2] & 0b00000010) >> 1 != 0;
        self.recursion_desired = (buf[2] & 0b00000001) != 0;

        self.recursion_available = (buf[3] & 0b10000000) >> 7 != 0;
        self.z = (buf[3] & 0b01110000) >> 4;
        let bits_rc = buf[3] & 0b00001111;
        self.response_code = bits_rc.try_into()?;

        self.qd_count = u16::from_be_bytes(buf[4..6].try_into()?);
        self.an_count = u16::from_be_bytes(buf[6..8].try_into()?);
        self.ns_count = u16::from_be_bytes(buf[8..10].try_into()?);
        self.ar_count = u16::from_be_bytes(buf[10..12].try_into()?);
        Ok(())
    }

    pub(super) fn build_reply(&self) -> Self {
        let mut reply = self.clone();
        reply.message_type = MessageType::Response;
        reply
    }

    pub(super) fn to_bytes(self) -> [u8; 12] {
        let mut buf = [0; 12];
        buf[0..2].copy_from_slice(&self.id.to_be_bytes());

        let bit_qr = self.message_type as u8;
        let bits_op = self.op_code as u8;
        let bit_aa = self.auth_answer as u8;
        let bit_tr = self.truncation as u8;
        let bit_rd = self.recursion_desired as u8;
        // Combine all bits into a single u8 using bitwise operations
        buf[2] = (bit_qr << 7) | (bits_op << 3) | (bit_aa << 2) | (bit_tr << 1) | bit_rd;

        let bit_ra = self.recursion_available as u8;
        let bits_z = self.z & 0b111; // Ensure we only use the least significant 3 bits
        let bits_rc = self.response_code as u8;
        // Combine all bits into a single u8 using bitwise operations
        buf[3] = (bit_ra << 7) | (bits_z << 4) | bits_rc;

        buf[4..6].copy_from_slice(&self.qd_count.to_be_bytes());
        buf[6..8].copy_from_slice(&self.an_count.to_be_bytes());
        buf[8..10].copy_from_slice(&self.ns_count.to_be_bytes());
        buf[10..12].copy_from_slice(&self.ar_count.to_be_bytes());

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_to_bytes() {
        let h = Header {
            id: 1234,
            message_type: MessageType::Response,
            op_code: OpCode::Status,
            auth_answer: true,
            truncation: false,
            recursion_desired: true,
            recursion_available: true,
            z: 3,
            response_code: ResponseCode::Refused,
            qd_count: 0,
            an_count: 0,
            ns_count: 0,
            ar_count: 0,
        };
        let bytes = h.to_bytes();

        let id: [u8; 2] = bytes[0..2].try_into().expect("invalid length");
        assert_eq!(1234, u16::from_be_bytes(id));
        assert_eq!(bytes[2], 0b1001_0101);
        assert_eq!(bytes[3], 0b1011_0101);
    }

    #[test]
    fn test_header_from_bytes() -> Result<()> {
        let mut buf: [u8; 12] = [0; 12];
        buf[0] = 0b0000_0100;
        buf[1] = 0b1101_0010;
        buf[2] = 0b1001_0101;
        buf[3] = 0b1011_0101;
        buf[6] = 0b0000_0010;
        buf[7] = 0b0000_1000;
        buf[11] = 0b0000_1100;

        let mut h = Header::default();
        h.read_bytes(buf)?;

        assert_eq!(1234, h.id);
        assert_eq!(MessageType::Response, h.message_type);
        assert_eq!(OpCode::Status, h.op_code);
        assert!(h.auth_answer);
        assert!(h.recursion_available);
        assert_eq!(520, h.an_count);
        assert_eq!(0, h.ns_count);
        assert_eq!(12, h.ar_count);
        Ok(())
    }
}
