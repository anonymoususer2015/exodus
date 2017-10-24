
use byteorder::{BigEndian, ReadBytesExt};

use std::mem::transmute;

use super::Options;

/// [RFC-791](https://tools.ietf.org/html/rfc791#page-11) , September 1981
///
/// 3.1.  Internet Header Format
///
///   A summary of the contents of the internet header follows:
///
///
///     0                   1                   2                   3
///     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |Version|  IHL  |Type of Service|          Total Length         |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |         Identification        |Flags|      Fragment Offset    |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |  Time to Live |    Protocol   |         Header Checksum       |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |                       Source Address                          |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |                    Destination Address                        |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///     |                    Options                    |    Padding    |
///     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
///                     Example Internet Datagram Header
///                               Figure 4.
///
///  Note that each tick mark represents one bit position.
///
///
///
/// Internet Protocol version 4 (IPv4)
/// 
/// https://en.wikipedia.org/wiki/IPv4#Packet_structure
/// 
/// Min: 160 bits   Max: 256 bits
#[derive(Debug, PartialEq, Eq)]
pub struct Ipv4Packet<'a, 'b> {
    version_ihl: u8,            //  4 bits, 4 bits
    /// Differentiated Services Code Point (DSCP) & Explicit Congestion Notification (ECN),
    /// Originally defined as the Type of service (ToS) field.
    dscp_ecn: u8,               //  6 bits, 2 bits
    total_length: u16,          // 16 bits
    identification: u16,        // 16 bits
    flags_fragment_offset: u16, //  3 bits, 13 bits
    time_to_live: u8,    //  8 bits
    protocol: u8,        //  8 bits
    header_checksum: u16,// 16 bits
    src_ip: u32,         // 32 bits
    dst_ip: u32,         // 32 bits
    options: Option<Options<'a>>, // 0 - 96 bits, start 160, end 256, if IHL >= 5
    payload: &'b [u8]
}

/// Internet Protocol version 6 (IPv6)
/// 
/// https://en.wikipedia.org/wiki/IPv6_packet
#[derive(Debug, PartialEq, Eq)]
pub struct Ipv6Packet<'a> {
    version: u8,         //  4 bits
    traffic_class: u8,   //  8 bits
    flow_label: u32,     // 20 bits
    payload_length: u16, // 16 bits
    next_header: u8,     //  8 bits
    hoplimit: u8,        //  8 bits
    src_ip  : u128,      // 128 bits
    dst_ip  : u128,      // 128 bits
    payload : &'a [u8]
}

/// OSI Model Layer 4 
#[derive(Debug, PartialEq, Eq)]
pub enum Packet<'a, 'b> {
    V4(Ipv4Packet<'a, 'b>),
    V6(Ipv6Packet<'a>)
}


impl <'a, 'b>Ipv4Packet<'a, 'b> {
    #[allow(unused_variables)]
    pub fn from_bytes(payload: &[u8]) -> Result<Self, ::std::io::Error> {
        if payload.len() < Ipv4Packet::min_size() {
            return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "size error ..."));
        }
        let version_ihl = payload[0];

        if (version_ihl >> 4) != 4 {
            return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "size error ..."));
        }
        let ihl = u8::from_str_radix(&format!("{:08b}", version_ihl)[4..8], 2).unwrap();

        let dscp_ecn    = payload[1];

        let mut total_length_bytes = &payload[2..4];
        let total_length: u16          = total_length_bytes.read_u16::<BigEndian>().unwrap();
        let mut identification_bytes = &payload[4..6];
        let identification: u16        = identification_bytes.read_u16::<BigEndian>().unwrap();

        let mut flags_fragment_offset_bytes = &payload[6..8];
        let flags_fragment_offset: u16 = flags_fragment_offset_bytes.read_u16::<BigEndian>().unwrap();

        let time_to_live = payload[8];
        let protocol     = payload[9];

        let mut header_checksum = &payload[10..12];
        let header_checksum: u16 = header_checksum.read_u16::<BigEndian>().unwrap();

        let mut src_ip_bytes = &[payload[12], payload[13], payload[14], payload[15]][..];
        let src_ip: u32 = src_ip_bytes.read_u32::<BigEndian>().unwrap();
        let mut dst_ip_bytes = &[payload[16], payload[17], payload[18], payload[19]][..];
        let dst_ip: u32 = dst_ip_bytes.read_u32::<BigEndian>().unwrap();
        
        let options: Option<Options<'a>>;
        let header_length: usize;
        if ihl >= 5 {
            if payload.len() < (Ipv4Packet::min_size() + 2) {
                return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "size error ..."));
            }

            match Options::from_bytes(&payload[20..]) {
                Ok(ipv4_options) => {
                    let ip_v4_options_length = ipv4_options.len();
                    if payload.len() < (Ipv4Packet::min_size() + ip_v4_options_length) {    
                        return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "size error ..."));
                    }
                    options = Some(ipv4_options);
                    header_length = 20 + ip_v4_options_length;
                }
                Err(e) => return Err(e)
            };
        } else {
            header_length = 20;
            options = None;
        }

        if payload.len() != total_length as usize {
            return Err(::std::io::Error::new(::std::io::ErrorKind::Other, "size error ..."));
        }
        println!("Ipv4 Header length: {:?}", header_length);
        Ok(Ipv4Packet {
            version_ihl   : version_ihl,
            dscp_ecn      : dscp_ecn,
            total_length  : total_length,
            identification: identification,
            flags_fragment_offset: flags_fragment_offset,
            time_to_live  : time_to_live,
            protocol: protocol,
            header_checksum: header_checksum,
            src_ip: src_ip,
            dst_ip: dst_ip,
            options: options,
            payload: unsafe {transmute(&payload[header_length..])}
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        unimplemented!();
    }

    pub fn min_size() -> usize {
        20
    }

    pub fn version(&self) -> u8 {
        self.version_ihl >> 4
    }
    pub fn ihl(&self) -> u8 {
        u8::from_str_radix(&format!("{:08b}", self.version_ihl)[4..8], 2).unwrap()
    }
    pub fn dscp(&self) -> u8 {
        u8::from_str_radix(&format!("{:08b}", self.dscp_ecn)[0..6], 2).unwrap()
    }
    pub fn ecn(&self) -> u8 {
        u8::from_str_radix(&format!("{:08b}", self.dscp_ecn)[6..8], 2).unwrap()
    }
    pub fn total_length(&self) -> u16 {
        self.total_length
    }
    pub fn flags(&self) -> u8 {
        u8::from_str_radix(&format!("{:016b}", self.flags_fragment_offset)[0..3], 2).unwrap()
    }
    pub fn fragment_offset(&self) -> u16 {
        u16::from_str_radix(&format!("{:016b}", self.flags_fragment_offset)[3..16], 2).unwrap()
    }
    pub fn time_to_live(&self) -> u8 {
        self.time_to_live
    }
    /// [Protocol number](https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml)
    /// of the header following the IPv4 header.
    pub fn protocol(&self) -> u8 {
        self.protocol
    }
    pub fn header_checksum(&self) -> u16 {
        self.header_checksum
    }
    pub fn src_ip(&self) -> u32 {
        self.src_ip
    }
    pub fn dst_ip(&self) -> u32 {
        self.dst_ip
    }
    pub fn options(&self) -> &'a Option<Options> {
        &self.options
    }
    pub fn payload(&self) -> &'b [u8] {
        self.payload
    }
    
    pub fn verifying(&self) -> bool {
        unimplemented!();
    }
}


impl <'a>Ipv6Packet<'a> {
    #[allow(unused_variables)]
    pub fn from_bytes(payload: &[u8]) -> Result<Self, ::std::io::Error> {
        println!("[WARN] 检测到不支持的 IPv6 Packet: {:?}", payload);
        unimplemented!();
    }

    pub fn as_bytes(&self) -> &[u8] {
        unimplemented!();
    }
    pub fn payload(&self) -> &'a [u8] {
        &self.payload
    }

    pub fn verifying(&self) -> bool {
        unimplemented!();
    }
}

impl <'a, 'b>Packet<'a, 'b> {
    pub fn from_bytes(payload: &[u8]) -> Result<Self, ::std::io::Error> {
        // let ver = u8::from_str_radix(&format!("{:08b}", payload[0])[0..4], 2).unwrap();
        let ver = payload[0] >> 4;
        match ver {
            // TODO: TCP/IP/ICMPv6 checksum
            4u8 => match Ipv4Packet::from_bytes(payload) {
                Ok(packet) => Ok(Packet::V4(packet)),
                Err(e)     => Err(e)
            },
            6u8 => match Ipv6Packet::from_bytes(payload) {
                Ok(packet) => Ok(Packet::V6(packet)),
                Err(e)     => Err(e)
            },
            version @ _ => {
                println!("RawPacket: {:?}", payload);
                println!("IP Version: {:?}", version);
                Err(::std::io::Error::new(::std::io::ErrorKind::Other, "IP Version Error!"))
            }
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        match *self {
            Packet::V4(ref packet) => packet.as_bytes(),
            Packet::V6(ref packet) => packet.as_bytes()
        }
    }

    pub fn payload(&self) -> &[u8] {
        match *self {
            Packet::V4(ref packet) => packet.payload(),
            Packet::V6(ref packet) => packet.payload()
        }
    }
    
    pub fn tcp_ip_checksum(&self) -> bool {
        // https://en.wikipedia.org/wiki/Transmission_Control_Protocol#Checksum_computation
        #[derive(Debug, PartialEq, Eq)]
        pub struct TcpIpv4PseudoHeader{
            src_ip  : u32,
            dst_ip  : u32,
            zeroes  : u8,
            protocol: u8,
            tcp_length: u16,
            // TCP Packet
            // ...
        }

        #[derive(Debug, PartialEq, Eq)]
        pub struct TcpIpv6PseudoHeader{
            src_ip     : u128,
            dst_ip     : u128,
            tcp_length : u32,
            zeroes     : u32,   // 24 bits
            next_header: u8,
            // TCP Packet
            // ...
        }
        unimplemented!();
    }
    pub fn udp_ip_checksum(&self) -> bool {
        // https://en.wikipedia.org/wiki/User_Datagram_Protocol#Checksum_computation
        #[derive(Debug, PartialEq, Eq)]
        pub struct UdpIpv4PseudoHeader{
            src_ip  : u32,
            dst_ip  : u32,
            zeroes  : u8,
            protocol: u8,
            udp_length: u16,
            // UDP Packet
            // ...
        }

        #[derive(Debug, PartialEq, Eq)]
        pub struct UdpIpv6PseudoHeader{
            src_ip     : u128,
            dst_ip     : u128,
            udp_length : u32,
            zeroes     : u32,   // 24 bits
            next_header: u8,
            // UDP Packet
            // ...
        }
        unimplemented!();
    }
    /// https://en.wikipedia.org/wiki/Internet_Control_Message_Protocol_version_6#Message_checksum
    pub fn icmp_ip_checksum(&self) -> bool {
        #[derive(Debug, PartialEq, Eq)]
        pub struct IcmpIpv6PseudoHeader {
            src_ip        : u128,
            dst_ip        : u128,
            icmp_v6_length: u32,
            zeros         : u16,
            next_header   : u16
        }
        unimplemented!();
    }
}