use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum PacketType {
    Open,
    Close,
    Ping,
    Pong,
    Message,
    Upgrade,
    Noop,
}

impl From<char> for PacketType {
    fn from(c: char) -> Self {
        use PacketType::*;
        match c {
            '0' => Open,
            '1' => Close,
            '2' => Ping,
            '3' => Pong,
            '4' => Message,
            '5' => Upgrade,
            '6' => Noop,
            _ => unreachable!(),
        }
    }
}

impl PacketType {
    fn to_char(&self) -> char {
        use PacketType::*;
        match self {
            Open => '0',
            Close => '1',
            Ping => '2',
            Pong => '3',
            Message => '4',
            Upgrade => '5',
            Noop => '6',
        }
    }
}

#[derive(PartialEq)]
pub struct Packet {
    packet_type: PacketType,
    encoded_data: PacketData,
}

impl std::fmt::Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} - {:?}", self.packet_type(), self.data())
    }
}

#[derive(Debug, PartialEq)]
pub enum PacketData {
    Str(String),
    Bytes(Vec<u8>),
}

impl Packet {
    pub fn new(packet_type: PacketType, encoded_data: &str) -> Self {
        Packet {
            packet_type,
            encoded_data: PacketData::Str(encoded_data.to_owned()),
        }
    }

    #[allow(dead_code)]
    pub fn with_bytes(packet_type: PacketType, encoded_data: Vec<u8>) -> Self {
        Packet {
            packet_type,
            encoded_data: PacketData::Bytes(encoded_data),
        }
    }

    pub fn data(&self) -> &PacketData {
        &self.encoded_data
    }

    pub fn packet_type(&self) -> &PacketType {
        &self.packet_type
    }

    pub fn encode(&self) -> String {
        match &self.encoded_data {
            PacketData::Str(string) => format!("{}{}", self.packet_type.to_char(), string),
            _ => unimplemented!(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PacketDecodeError> {
        // Convert byte to char, e.g. 4u8 -> '4'
        let packet_type = bytes
            .iter()
            .next()
            .unwrap()
            .to_string()
            .chars()
            .next()
            .unwrap();
        Ok(Packet {
            packet_type: packet_type.into(),
            encoded_data: PacketData::Bytes(bytes[1..].to_owned()),
        })
    }
}

#[derive(Debug)]
pub struct PacketDecodeError {}

impl FromStr for Packet {
    type Err = PacketDecodeError;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let packet_type = string.chars().next().unwrap().into();
        Ok(Packet {
            packet_type,
            encoded_data: PacketData::Str(string[1..].to_owned()),
        })
    }
}
