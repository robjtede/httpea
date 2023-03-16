//! QPACK encoder and decoder.

#![allow(unused, dead_code)]

use std::{collections::VecDeque, num::NonZeroU64};

mod static_table;
mod var_int;

use self::static_table::STATIC_TABLE;
pub use self::var_int::VarInt;

#[derive(Debug)]
enum DecodeError {
    MoreStreamsBlockedThanSupported,
    RequiredInsertCountTooSmall,
    EvictedReference,
    ReferenceIndexTooHigh,
    InvalidStaticTableReference,
    EntryWouldExceedDynamicTableCapacity,
}

#[derive(Debug)]
enum DecoderInstruction {
    /// https://datatracker.ietf.org/doc/html/rfc9204#section-2.2.2.1
    SectionAck,

    /// https://datatracker.ietf.org/doc/html/rfc9204#section-2.2.2.2
    StreamCancellation,

    /// https://datatracker.ietf.org/doc/html/rfc9204#section-2.2.2.3
    InsertCountIncrement,
}

#[derive(Debug)]
struct Decoder {
    max_dynamic_table_capacity: usize,

    /// [(absolute_index, (field name, field value))]
    table: VecDeque<(u64, (String, String))>,

    insert_count: usize,

    base: usize,
}

impl Decoder {
    pub fn new(max_dynamic_table_capacity: usize) -> Self {
        Self {
            max_dynamic_table_capacity,
            table: VecDeque::new(),
            insert_count: 0,
            base: 0,
        }
    }

    fn decode(&mut self, packet: &[u8]) -> Vec<(String, String)> {
        vec![]
    }
}

#[derive(Debug)]
enum EncoderError {}

#[derive(Debug)]
enum EncoderInstruction {
    SetDynamicTableCapacity(NonZeroU64),
}

#[derive(Debug)]
struct Encoder {
    decoder_dynamic_table_capacity: usize,
    known_received_count: usize,
}

impl Encoder {
    fn new_zero_rtt() -> Self {
        Self {
            decoder_dynamic_table_capacity: 0,
            known_received_count: 0,
        }
    }
}
