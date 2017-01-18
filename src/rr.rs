//! Base types for dealing with resource records.

use names::{Name, parse_name};

use nom::{be_u8, be_u16, be_u32, be_i32, IResult, Needed, ErrorKind};
use nom::IResult::*;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

/// A `Type` field indicates the structure and content of a resource record.
#[derive(Debug,PartialEq,Clone,Copy)]
pub enum Type {
    /// The `A` resource type, holding an IPv4 host address resource record.
    A,
    /// The `A` resource type, holding an IPv6 host address resource record.
    AAAA,
    /// The `CNAME` resource type, holding the canonical name for an alias.
    CNAME,
    /// The 'SOA' resource type, marks the start of a zone of authority.
    SOA,
    /// Indicates that the type is not known to this parser.
    Unknown {
        /// The value of the unknown type
        value: u16
    },
}

/// Enum for valid `class` values from DNS resource records.
#[derive(Debug,PartialEq,Clone,Copy)]
pub enum Class {
    /// The "Internet" class.
    Internet,
    /// The "CHAOS" class.
    Chaos,
    /// The "Hesoid" class.
    Hesoid,
    /// An unknown class value.
    Unknown {
        /// The value of the unknown type
        value: u16,
    },
}

/// A resource record associates a `Name` within a `Class` with `Type` dependent data.
#[derive(Debug,PartialEq,Clone)]
pub enum ResourceRecord {
    /// An IPv4 host address resource record.
    A {
        /// The `Name` this record applies to.
        name: Name,
        /// The `Class` this record applies to.
        class: Class,
        /// The "time to live" for this data, in seconds.
        ttl: i32,
        /// The IPv4 host address.
        addr: Ipv4Addr,
    },
    /// An IPv6 host address resource record.
    AAAA {
        /// The `Name` this record applies to.
        name: Name,
        /// The `Class` this record applies to.
        class: Class,
        /// The "time to live" for this data, in seconds.
        ttl: i32,
        /// The IPv6 host address.
        addr: Ipv6Addr,
    },
    /// The canonical name for an alias.
    CNAME {
        /// The `Name` this record applies to.
        name: Name,
        /// The `Class` this record applies to.
        class: Class,
        /// The "time to live" for this data, in seconds.
        ttl: i32,
        /// The canonical name for the alias referred to in `name`.
        cname: Name,
    },
    /// The start of a zone of authority.
    SOA {
        /// The `Name` this record applies to.
        name: Name,
        /// The `Class` this record applies to.
        class: Class,
        /// The "time to live" for this data, in seconds.
        ttl: i32,
        /// The <domain-name> of the name server that was the original or primary source of data
        /// for this zone.
        mname: Name,
        /// A <domain-name> which specifies the mailbox of the person responsible for this zone.
        rname: Name,
        /// The unsigned 32 bit version number of the original copy of the zone.
        ///
        ///  Zone transfers preserve this value. This value wraps and should be compared using
        /// sequence space arithmetic.
        serial: u32,
        /// A 32 bit time interval before the zone should be refreshed.
        refresh: u32,
        /// A 32 bit time interval that should elapse before a failed refresh should be retried.
        retry: u32,
        /// A 32 bit time value that specifies the upper limit on the time interval that can elapse
        /// before the zone is no longer authoritative.
        expire: u32,
        /// The unsigned 32 bit minimum TTL field that should be exported with any RR from this
        /// zone.
        minimum: u32,
    },
    /// A yet-unknown type of resource record.
    Unknown {
        /// The `Name` this record applies to.
        name: Name,
        /// The type code for this unknown data.
        rtype: u16,
        /// The `Class` this record applies to.
        class: Class,
        /// The "time to live" for this data, in seconds.
        ttl: i32,
        /// The data contained by the unknown record type.
        data: Vec<u8>,
    },
}

fn class_from(value: u16) -> Class {
    match value {
        1u16 => Class::Internet,
        3u16 => Class::Chaos,
        4u16 => Class::Hesoid,
        _ => Class::Unknown { value: value },
    }
}
named!(pub parse_class<&[u8], Class>,
    map!(be_u16, class_from)
);

pub fn type_from(value: u16) -> Type {
    match value {
        1u16 => Type::A,
        28u16 => Type::AAAA,
        5u16 => Type::CNAME,
        6u16 => Type::SOA,
        _ => Type::Unknown { value: value },
    }
}

named!(pub parse_type(&[u8]) -> Type,
    map!(be_u16, type_from)
);

//                                 1  1  1  1  1  1
//   0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                                               |
// /                                               /
// /                      NAME                     /
// |                                               |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                      TYPE                     |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                     CLASS                     |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                      TTL                      |
// |                                               |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
// |                   RDLENGTH                    |
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--|
// /                     RDATA                     /
// /                                               /
// +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+

/// Parses a byte stream into a `ResourceRecord`
pub fn parse_record<'a>(data: &'a [u8], i: &'a [u8]) -> IResult<&'a [u8], ResourceRecord> {
    let name = parse_name(data, i);
    if let Done(output, name) = name {
        let rtype = parse_type(output);
        if let Done(output, rtype) = rtype {
            return match rtype {
                Type::A => parse_a(output, name),
                Type::AAAA => parse_aaaa(output, name),
                Type::CNAME => parse_cname(data, output, name),
                Type::SOA => parse_soa(data, output, name),
                Type::Unknown { value: a } => parse_unknown(output, name, a),
            };
        };
        match rtype {
            Done(_, _) => unreachable!(),
            Error(e) => return Error(error_node_position!(ErrorKind::Custom(401), i, e)),
            Incomplete(e) => return Incomplete(e),
        }
    };
    match name {
        Done(_, _) => unreachable!(),
        Error(e) => return Error(e),
        Incomplete(e) => return Incomplete(e),
    }
}

fn parse_unknown<'a>(i: &'a [u8], name: Name, rtype: u16) -> IResult<&'a [u8], ResourceRecord> {
    parse_body_unknown(i).map(|args: (Class, i32, &[u8])| {
        let mut vec = Vec::with_capacity(args.2.len());
        vec.extend(args.2.iter().cloned());
        ResourceRecord::Unknown {
            name: name,
            rtype: rtype,
            class: args.0,
            ttl: args.1,
            data: vec,
        }
    })
}

named!(parse_body_unknown<&[u8], (Class, i32, &[u8])>,
    do_parse!(
        class: parse_class >>
        ttl: be_i32 >>
        length: be_u16 >>
        data: take!( length ) >>
        ((class, ttl, data))
    )
);

fn parse_a<'a>(i: &'a [u8], name: Name) -> IResult<&'a [u8], ResourceRecord> {
    parse_body_a(i).map(|args: (Class, i32, Ipv4Addr)| {
        ResourceRecord::A {
            name: name,
            class: args.0,
            ttl: args.1,
            addr: args.2,
        }
    })
}

named!(parse_body_a<&[u8], (Class, i32, Ipv4Addr)>,
    do_parse!(
        class: parse_class >>
        ttl: be_i32 >>
        length: add_return_error!(ErrorKind::Custom(403), tag!( b"\x00\x04" )) >>
        a: be_u8 >>
        b: be_u8 >>
        c: be_u8 >>
        d: be_u8 >>
        ((class, ttl, Ipv4Addr::new(a, b, c, d)))
    )
);

// named!(parse_aaaa<&[u8], ResourceRecord>,
// map!(parse_body_aaaa, |args: (Class, i32, Ipv6Addr)| {
//     ResourceRecord::AAAA {
//         name: name,
//         class: args.0,
//         ttl: args.1,
//         addr: args.2,
//     }
// })
// );
fn parse_aaaa<'a>(i: &'a [u8], name: Name) -> IResult<&'a [u8], ResourceRecord> {
    parse_body_aaaa(i).map(|args: (Class, i32, Ipv6Addr)| {
        ResourceRecord::AAAA {
            name: name,
            class: args.0,
            ttl: args.1,
            addr: args.2,
        }
    })
}

named!(parse_body_aaaa<&[u8], (Class, i32, Ipv6Addr)>,
    do_parse!(
        class: parse_class >>
        ttl: be_i32 >>
        length: add_return_error!(ErrorKind::Custom(405), tag!( "\x00\x10" )) >>
        a: be_u16 >>
        b: be_u16 >>
        c: be_u16 >>
        d: be_u16 >>
        e: be_u16 >>
        f: be_u16 >>
        g: be_u16 >>
        h: be_u16 >>
        ((class, ttl, Ipv6Addr::new(a, b, c, d, e, f, g, h)))
    )
);

fn parse_cname<'a>(data: &'a [u8], i: &'a [u8], name: Name) -> IResult<&'a [u8], ResourceRecord> {
    let body = parse_body_cname(i);
    if let Done(output, (class, ttl, length)) = body {
        if output.len() < length {
            let size = length - output.len();
            return Incomplete(Needed::Size(size));
        };
        let cname = parse_name(data, output);
        if let Done(_, cname) = cname {
            return Done(&output[length..], ResourceRecord::CNAME {
                name: name,
                class: class,
                ttl: ttl,
                cname: cname,
            });
        };
        match cname {
            Done(_, _) => unreachable!(),
            Error(e) => return Error(e),
            Incomplete(e) => return Incomplete(e),
        }
    };
    match body {
        Done(_, _) => unreachable!(),
        Error(e) => return Error(e),
        Incomplete(e) => return Incomplete(e),
    }
}

named!(parse_body_cname<&[u8], (Class, i32, usize)>,
    do_parse!(
        class: parse_class >>
        ttl: be_i32 >>
        length: be_u16 >>
        ((class, ttl, length as usize))
    )
);

fn parse_soa<'a>(data: &'a [u8], i: &'a [u8], name: Name) -> IResult<&'a [u8], ResourceRecord> {
    let body = parse_body_soa_1(i);
    if let Done(output, (class, ttl, length)) = body {
        if output.len() < length {
            let size = length - output.len();
            return Incomplete(Needed::Size(size));
        };
        let mname = parse_name(data, output);
        if let Done(o2, mname) = mname {
            let rname = parse_name(data, o2);
            if let Done(o3, rname) = rname {
                let args = parse_body_soa_2(o3);
                if let Done(_, args) = args {
                    return Done(&output[length..], ResourceRecord::SOA {
                        name: name,
                        class: class,
                        ttl: ttl,
                        mname: mname,
                        rname: rname,
                        serial: args.0,
                        refresh: args.1,
                        retry: args.2,
                        expire: args.3,
                        minimum: args.4
                    });
                };
                match args {
                    Done(_, _) => unreachable!(),
                    Error(e) => return Error(e),
                    Incomplete(e) => return Incomplete(e),
                }
            };
            match rname {
                Done(_, _) => unreachable!(),
                Error(e) => return Error(e),
                Incomplete(e) => return Incomplete(e),
            }
        };
        match mname {
            Done(_, _) => unreachable!(),
            Error(e) => return Error(e),
            Incomplete(e) => return Incomplete(e),
        }
    };
    match body {
        Done(_, _) => unreachable!(),
        Error(e) => return Error(e),
        Incomplete(e) => return Incomplete(e),
    }
}

named!(parse_body_soa_1<&[u8], (Class, i32, usize)>,
    do_parse!(
        class: parse_class >>
        ttl: be_i32 >>
        length: be_u16 >>
        ((class, ttl, length as usize))
    )
);

named!(parse_body_soa_2<&[u8], (u32, u32, u32, u32, u32)>,
    do_parse!(
        serial: be_u32 >>
        refresh: be_u32 >>
        retry: be_u32 >>
        expire: be_u32 >>
        minimum: be_u32 >>
        ((serial, refresh, retry, expire, minimum))
    )
);


impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Class::Internet => write!(f, "IN"),
            Class::Chaos => write!(f, "CH"),
            Class::Hesoid => write!(f, "HS"),
            Class::Unknown { value: x } => write!(f, "Unkown({})", x),
        }
    }
}

#[cfg(test)]
mod tests {
    use nom::IResult::Done;
    use std::net::Ipv4Addr;
    use super::*;

    #[test]
    fn parse_type_bytes() {
        let a = b"\x00\x01abcd";
        let aaaa = b"\x00\x1cabcd";
        let cname = b"\x00\x05abcd";
        let md_deprecated = b"\x00\x03abcd";

        assert_eq!(parse_type(&a[..]), Done(&b"abcd"[..], Type::A));
        assert_eq!(parse_type(&aaaa[..]), Done(&b"abcd"[..], Type::AAAA));
        assert_eq!(parse_type(&cname[..]), Done(&b"abcd"[..], Type::CNAME));
        assert_eq!(parse_type(&md_deprecated[..]), Done(&b"abcd"[..], Type::Unknown { value: 3 }));
    }

    #[test]
    fn parse_class_bytes() {
        let a = b"\x00\x01abcd";
        let b = b"\x00\x02abcd";
        let c = b"\x00\x03abcd";
        let d = b"\x00\x04abcd";

        assert_eq!(parse_class(&a[..]), Done(&b"abcd"[..], Class::Internet));
        assert_eq!(parse_class(&c[..]), Done(&b"abcd"[..], Class::Chaos));
        assert_eq!(parse_class(&d[..]), Done(&b"abcd"[..], Class::Hesoid));
        assert_eq!(parse_class(&b[..]), Done(&b"abcd"[..], Class::Unknown { value: 2 }));
    }

    #[test]
    fn parse_a_record() {
        let src = b"\x03FOO\x03BAR\x00\x00\x01\x00\x01\x00\x00\x0e\x10\x00\x04\x7f\x00\x00\x01";
        assert_eq!(parse_record(&src[..], &src[..]),
                    Done(&b""[..], ResourceRecord::A {
                        name: "FOO.BAR.".parse().unwrap(),
                        class: Class::Internet,
                        ttl: 3600,
                        addr: Ipv4Addr::new(127, 0, 0, 1)
                    }));
    }

    #[test]
    fn parse_aaaa_record() {
        let src = b"\x03FOO\x03BAR\x00\x00\x1c\x00\x01\x00\x00\x0e\x10\x00\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";
        assert_eq!(parse_record(&src[..], &src[..]),
                    Done(&b""[..], ResourceRecord::AAAA {
                        name: "FOO.BAR.".parse().unwrap(),
                        class: Class::Internet,
                        ttl: 3600,
                        addr: "::1".parse().unwrap()
                    }));
    }
}
