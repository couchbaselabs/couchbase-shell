//! Utility functions and statics for interacting with the KV binary protocol

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

pub static HEADER_SIZE: usize = 24;
// pub static ERROR_MAP_VERSION: u16 = 1;

#[derive(Debug)]
pub struct KvRequest {
    opcode: Opcode,
    datatype: u8,
    partition: u16,
    opaque: u32,
    cas: u64,
    key: Option<Bytes>,
    extras: Option<Bytes>,
    body: Option<Bytes>,
    collection_id: u32,
}

impl KvRequest {
    pub fn new(
        opcode: Opcode,
        datatype: u8,
        partition: u16,
        cas: u64,
        key: Option<Bytes>,
        extras: Option<Bytes>,
        body: Option<Bytes>,
        collection_id: u32,
    ) -> Self {
        KvRequest {
            opcode,
            datatype,
            partition,
            cas,
            key,
            extras,
            body,
            opaque: 0,
            collection_id,
        }
    }

    pub fn set_opaque(&mut self, opaque: u32) {
        self.opaque = opaque;
    }

    pub fn opaque(&self) -> u32 {
        self.opaque
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }
}

#[derive(Debug)]
pub struct KvResponse {
    opcode: Opcode,
    // datatype: u8,
    status: Status,
    opaque: u32,
    cas: u64,
    // key: Option<Bytes>,
    // extras: Option<Bytes>,
    body: Option<Bytes>,
}

impl From<&Bytes> for KvResponse {
    fn from(input: &Bytes) -> Self {
        let mut slice = input.slice(0..input.len());

        // 0
        let magic = Magic::from(slice.get_u8());
        let flexible = magic.is_flexible();

        // 1
        let opcode = Opcode::try_from(slice.get_u8()).unwrap();

        let flexible_extras_len = if flexible {
            // 2
            slice.get_u8()
        } else {
            0
        } as usize;

        let key_len = if flexible {
            // 3
            slice.get_u8() as u16
        } else {
            // 2, 3
            slice.get_u16()
        } as usize;

        // 4
        let extras_len = slice.get_u8() as usize;
        // 5
        let _datatype = slice.get_u8();
        // 6, 7
        let status = slice.get_u16();

        // 8, 9
        let total_body_len = slice.get_u32() as usize;
        // 10, 11, 12, 13
        let opaque = slice.get_u32();
        // 14, 15, 16, 17, 18, 19, 20, 21
        let cas = slice.get_u64();
        let body_len = total_body_len - key_len - extras_len - flexible_extras_len;

        let _extras = if extras_len > 0 {
            Some(input.slice(
                (HEADER_SIZE + flexible_extras_len)
                    ..(HEADER_SIZE + flexible_extras_len + extras_len),
            ))
        } else {
            None
        };

        let _key = if key_len > 0 {
            Some(input.slice(
                (HEADER_SIZE + flexible_extras_len + extras_len)
                    ..(HEADER_SIZE + flexible_extras_len + extras_len + key_len),
            ))
        } else {
            None
        };

        let body = if body_len > 0 {
            Some(input.slice((HEADER_SIZE + flexible_extras_len + extras_len + key_len)..))
        } else {
            None
        };

        KvResponse {
            opaque,
            body,
            // extras,
            // key,
            status: Status::from(status),
            // datatype,
            cas,
            opcode,
        }
    }
}

impl KvResponse {
    pub fn opaque(&self) -> u32 {
        self.opaque
    }

    // body takes the body from the response.
    pub fn body(&mut self) -> Option<Bytes> {
        self.body.take()
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn cas(&self) -> u64 {
        self.cas
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }
}

/// Creates a regular, non-flex request with all fields necessary.
pub fn request(req: KvRequest, collections_enabled: bool) -> BytesMut {
    let key = match req.key {
        Some(k) => {
            if collections_enabled {
                let cid = make_uleb128_32(k, req.collection_id);
                Some(cid)
            } else {
                Some(k)
            }
        }
        None => None,
    };

    let key_size = key.as_ref().map(|b| b.len()).unwrap_or_default();
    let extras_size = req.extras.as_ref().map(|b| b.len()).unwrap_or_default();
    let total_body_size =
        key_size + extras_size + req.body.as_ref().map(|b| b.len()).unwrap_or_default();

    let mut builder = BytesMut::with_capacity(HEADER_SIZE + total_body_size);
    builder.put_u8(Magic::Request.encoded());
    builder.put_u8(req.opcode.encoded());
    builder.put_u16(key_size as u16);
    builder.put_u8(extras_size as u8);
    builder.put_u8(req.datatype);
    builder.put_u16(req.partition);
    builder.put_u32(total_body_size as u32);
    builder.put_u32(req.opaque);
    builder.put_u64(req.cas);

    if let Some(extras) = req.extras {
        builder.put(extras);
    }

    if let Some(k) = key {
        builder.put(k);
    }

    if let Some(body) = req.body {
        builder.put(body);
    }

    builder
}

// Creates a flexible request with optional framing extras
pub fn _flexible_request(
    opcode: Opcode,
    datatype: u8,
    partition: u16,
    opaque: u32,
    cas: u64,
    key: Option<Bytes>,
    framing_extras: Option<Bytes>,
    extras: Option<Bytes>,
    body: Option<Bytes>,
) -> BytesMut {
    let key_size = key.as_ref().map(|b| b.len()).unwrap_or_default();
    let extras_size = extras.as_ref().map(|b| b.len()).unwrap_or_default();
    let framing_extras_size = framing_extras.as_ref().map(|b| b.len()).unwrap_or_default();
    let total_body_size = key_size
        + extras_size
        + framing_extras_size
        + body.as_ref().map(|b| b.len()).unwrap_or_default();

    let mut builder = BytesMut::with_capacity(HEADER_SIZE + total_body_size);
    builder.put_u8(Magic::FlexibleRequest.encoded());
    builder.put_u8(opcode.encoded());
    builder.put_u8(framing_extras_size as u8);
    builder.put_u8(key_size as u8);
    builder.put_u8(extras_size as u8);
    builder.put_u8(datatype);
    builder.put_u16(partition);
    builder.put_u32(total_body_size as u32);
    builder.put_u32(opaque);
    builder.put_u64(cas);

    if let Some(framing_extras) = framing_extras {
        builder.put(framing_extras);
    }

    if let Some(extras) = extras {
        builder.put(extras);
    }

    if let Some(key) = key {
        builder.put(key);
    }

    if let Some(body) = body {
        builder.put(body);
    }

    builder
}

/// Creates a regular, non-flex response with all fields necessary.
pub fn _response(
    opcode: Opcode,
    datatype: u8,
    status: u16,
    opaque: u32,
    cas: u64,
    key: Option<Bytes>,
    extras: Option<Bytes>,
    body: Option<Bytes>,
) -> BytesMut {
    let key_size = key.as_ref().map(|b| b.len()).unwrap_or_default();
    let extras_size = extras.as_ref().map(|b| b.len()).unwrap_or_default();
    let total_body_size =
        key_size + extras_size + body.as_ref().map(|b| b.len()).unwrap_or_default();

    let mut builder = BytesMut::with_capacity(HEADER_SIZE + total_body_size);
    builder.put_u8(Magic::Response.encoded());
    builder.put_u8(opcode.encoded());
    builder.put_u16(key_size as u16);
    builder.put_u8(extras_size as u8);
    builder.put_u8(datatype);
    builder.put_u16(status);
    builder.put_u32(total_body_size as u32);
    builder.put_u32(opaque);
    builder.put_u64(cas);

    if let Some(extras) = extras {
        builder.put(extras);
    }

    if let Some(key) = key {
        builder.put(key);
    }

    if let Some(body) = body {
        builder.put(body);
    }

    builder
}

/// Takes a full packet and extracts the body as a slice if possible.
pub fn _body(input: &Bytes) -> Option<Bytes> {
    let mut slice = input.slice(0..input.len());

    let flexible = Magic::from(slice.get_u8()).is_flexible();

    let flexible_extras_len = if flexible {
        slice.advance(1);
        slice.get_u8()
    } else {
        0
    } as usize;
    let key_len = if flexible {
        slice.get_u8() as u16
    } else {
        slice.advance(1);
        slice.get_u16()
    } as usize;
    let extras_len = slice.get_u8() as usize;
    slice.advance(3);
    let total_body_len = slice.get_u32() as usize;
    let body_len = total_body_len - key_len - extras_len - flexible_extras_len;

    if body_len > 0 {
        Some(input.slice((HEADER_SIZE + flexible_extras_len + extras_len + key_len)..))
    } else {
        None
    }
}

/// Dumps a packet into a easily debuggable string format.
///
/// Note that this is only really suitable when you want to println a full
/// packet, but nonetheless it is helpful for testing.
pub fn _dump(input: &Bytes) -> String {
    if input.len() < HEADER_SIZE {
        return "Received less bytes than a KV header, invalid data?".into();
    }

    let mut slice = input.slice(0..input.len());

    let mut output = String::new();
    output.push_str("--- Packet Dump Info --\n");
    let magic = slice.get_u8();
    output.push_str(&format!(
        "     Magic: 0x{:x} ({:?})\n",
        magic,
        Magic::from(magic)
    ));
    let opcode = slice.get_u8();
    output.push_str(&format!(
        "    Opcode: 0x{:x} ({:?})\n",
        opcode,
        Opcode::try_from(opcode).unwrap()
    ));
    let key_size = slice.get_u16();
    output.push_str(&format!("   Key Len: {} bytes\n", key_size));
    let extras_size = slice.get_u8();
    output.push_str(&format!("Extras Len: {} bytes\n", extras_size));
    let datatype = slice.get_u8();
    output.push_str(&format!("  Datatype: 0x{:x}\n", datatype));
    let partition = slice.get_u16();
    output.push_str(&format!(" Partition: 0x{:x}\n", partition));

    if let Some(body) = _body(input) {
        output.push_str(&format!("      Body: {:?}\n", body));
    }

    output.push_str("-----------------------\n");

    output
}

#[derive(Debug, Copy, Clone)]
pub enum Opcode {
    Get,
    Set,
    Add,
    Replace,
    Remove,
    Hello,
    Noop,
    ErrorMap,
    Auth,
    SelectBucket,
}

impl Opcode {
    pub fn encoded(&self) -> u8 {
        match self {
            Self::Get => 0x00,
            Self::Set => 0x01,
            Self::Add => 0x02,
            Self::Replace => 0x03,
            Self::Remove => 0x04,
            Self::Noop => 0x0A,
            Self::Hello => 0x1F,
            Self::Auth => 0x21,
            Self::SelectBucket => 0x89,
            Self::ErrorMap => 0xFE,
        }
    }
}

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#04x}", self.encoded())
    }
}

impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(input: u8) -> Result<Self, Self::Error> {
        Ok(match input {
            0x00 => Opcode::Get,
            0x01 => Opcode::Set,
            0x02 => Opcode::Add,
            0x03 => Opcode::Replace,
            0x04 => Opcode::Remove,
            0x0A => Opcode::Noop,
            0x1F => Opcode::Hello,
            0x21 => Opcode::Auth,
            0x89 => Opcode::SelectBucket,
            0xFE => Opcode::ErrorMap,
            _ => return Err(input),
        })
    }
}

#[derive(Debug)]
pub enum Magic {
    Request,
    FlexibleRequest,
    Response,
    FlexibleResponse,
    Unknown,
}

impl Magic {
    pub fn encoded(&self) -> u8 {
        match self {
            Self::FlexibleRequest => 0x08,
            Self::Request => 0x80,
            Self::FlexibleResponse => 0x18,
            Self::Response => 0x81,
            Self::Unknown => panic!("Cannot convert unknown magic"),
        }
    }

    pub fn is_flexible(&self) -> bool {
        matches!(self, Self::FlexibleRequest | Self::FlexibleResponse)
    }
}

impl From<u8> for Magic {
    fn from(input: u8) -> Magic {
        match input {
            0x80 => Magic::Request,
            0x08 => Magic::FlexibleRequest,
            0x81 => Magic::Response,
            0x18 => Magic::FlexibleResponse,
            _ => Magic::Unknown,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Status {
    Success,
    AuthError,
    AccessError,
    KeyNotFound,
    KeyExists,
    CollectionUnknown,
    ScopeUnknown,
    Unknown(u16),
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl Status {
    pub fn as_string(&self) -> String {
        match self {
            Status::Success => "success".into(),
            Status::AuthError => "authentication error".into(),
            Status::AccessError => "access error".into(),
            Status::KeyNotFound => "key not found".into(),
            Status::KeyExists => "key already exists".into(),
            Status::CollectionUnknown => "collection unknown".into(),
            Status::ScopeUnknown => "scope unknown".into(),
            Status::Unknown(status) => format!("{:#04x}", status),
        }
    }
}

impl From<u16> for Status {
    fn from(input: u16) -> Status {
        match input {
            0x00 => Status::Success,
            0x01 => Status::KeyNotFound,
            0x02 => Status::KeyExists,
            0x88 => Status::CollectionUnknown,
            0x8c => Status::ScopeUnknown,
            0x20 => Status::AuthError,
            0x24 => Status::AccessError,
            _ => Status::Unknown(input),
        }
    }
}

fn make_uleb128_32(key: Bytes, collection_id: u32) -> Bytes {
    let mut cid = collection_id;
    let mut builder = BytesMut::with_capacity(key.len() + 5);
    loop {
        let mut c: u8 = (cid & 0x7f) as u8;
        cid >>= 7;
        if cid != 0 {
            c |= 0x80;
        }

        builder.put_u8(c);
        if c & 0x80 == 0 {
            break;
        }
    }
    for k in key {
        builder.put_u8(k);
    }

    builder.freeze()
}
