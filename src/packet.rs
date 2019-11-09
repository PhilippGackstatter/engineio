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

#[derive(Debug, PartialEq)]
pub struct Packet {
    packet_type: PacketType,
    encoded_data: String,
}

impl Packet {
    pub fn new(packet_type: PacketType, encoded_data: &str) -> Self {
        Packet {
            packet_type,
            encoded_data: encoded_data.to_owned(),
        }
    }
    pub fn data(&self) -> &str {
        &self.encoded_data
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
            encoded_data: string[1..].to_owned(),
        })
    }
}
