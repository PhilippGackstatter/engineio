use crate::packet::Packet;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct Payload {
    packets: Vec<Packet>,
}

impl Payload {
    pub fn packets(&self) -> &Vec<Packet> {
        &self.packets
    }
}

#[derive(Debug)]
pub struct PayloadDecodeError {}

impl From<std::num::ParseIntError> for PayloadDecodeError {
    fn from(error: std::num::ParseIntError) -> Self {
        println!("{:#?}", error);
        unimplemented!();
    }
}

impl FromStr for Payload {
    type Err = PayloadDecodeError;
    fn from_str(mut string: &str) -> Result<Self, Self::Err> {
        let mut packets = vec![];
        let mut end = 0;

        while end < string.len() + 1 {
            let colon_index = string.find(':').ok_or_else(|| PayloadDecodeError {})?;

            let packet_len_str = &string[..colon_index];
            let packet_len = packet_len_str.parse::<usize>()?;
            end = colon_index + 1 + packet_len;

            let packet: Packet = string[colon_index + 1..end].to_owned().parse().unwrap();

            packets.push(packet);
            string = &string[end..];
        }

        Ok(Payload { packets })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::PacketType;

    #[test]
    fn test_payload_decoding_of_one_packet() {
        let input = r#"96:0{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#;

        let result = input.parse::<Payload>().unwrap();
        let expected = Packet::new( PacketType::Open,
             r#"{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#);

        assert_eq!(*result.packets.first().unwrap(), expected);
    }

    #[test]
    fn test_payload_decoding_of_multiple_packets() {
        let mut input = r#"96:0{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#.to_owned();
        input.push_str(&input.clone());
        input.push_str(&input.clone());
        let result = input.parse::<Payload>().unwrap();
        let expected = Packet::new( PacketType::Open,
            r#"{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#);

        let mut iter_count = 0;
        for packet in result.packets {
            iter_count += 1;
            assert_eq!(packet, expected);
        }
        assert_eq!(iter_count, 4);
    }
}
