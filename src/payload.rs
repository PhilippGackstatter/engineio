use crate::packet::Packet;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct Payload {
    packets: Vec<Packet>,
}

impl Payload {
    pub fn from_packet(p: Packet) -> Self {
        Self { packets: vec![p] }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for packet in self.packets.iter() {
            let encoded_packet = packet.encode();
            let mut packet_len = encoded_packet.len().to_string();
            packet_len.push(':');
            bytes.extend(packet_len.as_bytes().to_owned());
            bytes.extend(encoded_packet.as_bytes().to_owned());
        }

        bytes
    }

    pub fn new(mut bytes: &[u8]) -> Result<Self, PayloadDecodeError> {
        let mut packets = Vec::new();

        while !bytes.is_empty() {
            let data_type = bytes[0];
            if data_type > 1 {
                return Err(PayloadDecodeError {});
            }
            println!("data type {}", data_type);
            bytes = &bytes[1..];
            let (start, end) = Self::get_next_packet_window(&bytes).unwrap();

            if data_type == 1 {
                let packet = Packet::from_bytes(&bytes[start..end]).unwrap();
                packets.push(packet);
            } else {
                let string = String::from_utf8(bytes[start..end].into()).unwrap();
                packets.push(Packet::from_str(&string).unwrap());
            }

            bytes = &bytes[end..];
        }

        Ok(Payload { packets })
    }

    pub fn packets(&self) -> &Vec<Packet> {
        &self.packets
    }

    pub fn get_next_packet_window(bytes: &[u8]) -> Result<(usize, usize), PayloadDecodeError> {
        let mut packet_len = 0;
        let mut start = 0;
        println!("{:?}", bytes);
        for (index, byte) in bytes.iter().enumerate() {
            if *byte == 255 {
                start = index + 1;
                break;
            } else {
                packet_len = (10 * packet_len) + *byte as usize;
            }
        }

        Ok((start, packet_len + start))
    }

    pub fn from_str_colon_msg_format(mut bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let colon_index = bytes
            .iter()
            .position(|byte| *byte == b':')
            .ok_or(0)
            .unwrap_or_else(|_| panic!("{:?}", bytes));

        let packet_len_str = String::from_utf8(bytes[..(colon_index as usize)].to_vec())
            .unwrap_or_else(|e| panic!(e));
        let packet_len = packet_len_str
            .parse::<usize>()
            .unwrap_or_else(|e| panic!("{:#?} {:#?}", e, packet_len_str));

        let mut end = 0;
        let mut packets = vec![];

        while end < bytes.len() + 1 {
            end = colon_index as usize + 1 + packet_len;
            let packet_bytes = bytes[colon_index as usize + 1..end].to_owned();
            let packet_str = String::from_utf8(packet_bytes)?;
            let packet = Packet::from_str(&packet_str).unwrap();

            packets.push(packet);
            bytes = &bytes[end..];
        }
        Ok(Payload { packets })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::PacketType;

    #[test]
    fn test_payload_decoding_of_one_packet() {
        let input = r#"96:0{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#;

        let result = Payload::from_str_colon_msg_format(&input.as_bytes()).unwrap();
        let expected = Packet::new( PacketType::Open,
             r#"{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#);

        assert_eq!(*result.packets.first().unwrap(), expected);
    }

    #[test]
    fn test_payload_decoding_of_multiple_packets() {
        let mut input = r#"96:0{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#.to_owned();
        input.push_str(&input.clone());
        input.push_str(&input.clone());
        let result = Payload::from_str_colon_msg_format(&input.as_bytes()).unwrap();
        let expected = Packet::new( PacketType::Open,
            r#"{"sid":"d5vWJMbJuMCRZOnuAAAI","upgrades":["websocket"],"pingInterval":25000,"pingTimeout":5000}"#);

        let mut iter_count = 0;
        for packet in result.packets {
            iter_count += 1;
            assert_eq!(packet, expected);
        }
        assert_eq!(iter_count, 4);
    }

    #[test]
    fn test_payload_decoding_of_binary() {
        let bytes = [
            0, 1, 3, 255, 52, 117, 116, 102, 32, 56, 32, 115, 116, 114, 105, 110, 103, 1, 7, 255,
            4, 0, 1, 2, 3, 4, 5,
        ];

        let payload = Payload::new(&bytes).unwrap();
        let mut packets = payload.packets().iter();
        let first_packet = packets.next().unwrap();
        let second_packet = packets.next().unwrap();

        let expected_first_packet = Packet::new(PacketType::Message, "utf 8 string");
        let expected_second_packet =
            Packet::with_bytes(PacketType::Message, vec![0, 1, 2, 3, 4, 5]);

        assert_eq!(*first_packet, expected_first_packet);
        assert_eq!(*second_packet, expected_second_packet);
    }
}
