use super::reader::{DecodeError, Reader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqHeader {
    pub resolution: u16,
    pub tempo_microseconds: u32,
    pub time_signature: [u8; 2],
}

impl SeqHeader {
    /// Original SDK initialization rounds BPM to the nearest integer, ties down.
    /// A zero tempo is retained as data but cannot initialize the original clock.
    pub fn initial_tempo_bpm(&self) -> Option<u32> {
        let tempo = self.tempo_microseconds;
        if tempo == 0 {
            return None;
        }
        let quotient = 60_000_000 / tempo;
        let remainder = 60_000_000 % tempo;
        Some(quotient + u32::from(remainder > tempo / 2))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeqEventKind {
    Channel { status: u8, data: [u8; 2], len: u8 },
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeqEvent {
    pub delta_offset: usize,
    pub after_delta: usize,
    pub delta: u32,
    pub tick: u64,
    pub kind: SeqEventKind,
}

impl SeqEvent {
    /// The SDK delta reader uses a wrapping 32-bit counter in tenths of ticks.
    pub fn sdk_delta(&self) -> u32 {
        self.delta.wrapping_mul(10)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seq {
    pub header: SeqHeader,
    pub events: Vec<SeqEvent>,
}

fn delta(r: Reader<'_>, cursor: &mut usize) -> Result<u32, DecodeError> {
    let start = *cursor;
    let mut value = 0;
    for _ in 0..4 {
        let byte = r.u8(*cursor)?;
        *cursor += 1;
        value = (value << 7) | u32::from(byte & 127);
        if byte < 128 {
            return Ok(value);
        }
    }
    Err(DecodeError::new(start, "SEQ delta exceeds four bytes"))
}

impl Seq {
    /// Decode version-one SEQ channel events and the end marker used by LSD.
    /// Other system/meta events produce an explicit error until recovered.
    pub fn parse(bytes: &[u8]) -> Result<Self, DecodeError> {
        let r = Reader(bytes);
        if r.bytes(0, 4)? != b"pQES" {
            return Err(DecodeError::new(0, "invalid SEQ signature"));
        }
        if r.bytes(4, 4)? != [0, 0, 0, 1] {
            return Err(DecodeError::new(4, "unsupported SEQ version"));
        }
        let resolution = u16::from_be_bytes(r.bytes(8, 2)?.try_into().unwrap());
        if resolution == 0 || resolution > i16::MAX as u16 {
            return Err(DecodeError::new(8, "invalid SEQ resolution"));
        }
        let tempo = r.bytes(10, 3)?;
        let header = SeqHeader {
            resolution,
            tempo_microseconds: u32::from_be_bytes([0, tempo[0], tempo[1], tempo[2]]),
            time_signature: r.bytes(13, 2)?.try_into().unwrap(),
        };
        let mut cursor = 15;
        let mut running = None;
        let mut tick = 0_u64;
        let mut events = Vec::new();
        loop {
            let delta_offset = cursor;
            let delta = delta(r, &mut cursor)?;
            let after_delta = cursor;
            tick = tick
                .checked_add(u64::from(delta))
                .ok_or_else(|| DecodeError::new(delta_offset, "SEQ tick overflow"))?;
            let first = r.u8(cursor)?;
            let status = if first >= 128 {
                cursor += 1;
                first
            } else {
                running
                    .ok_or_else(|| DecodeError::new(cursor, "SEQ running status has no status"))?
            };
            let kind = if (0x80..=0xef).contains(&status) {
                running = Some(status);
                let len = if matches!(status >> 4, 12 | 13) { 1 } else { 2 };
                let mut data = [0; 2];
                for byte in &mut data[..len] {
                    *byte = r.u8(cursor)?;
                    if *byte >= 128 {
                        return Err(DecodeError::new(cursor, "invalid SEQ channel data byte"));
                    }
                    cursor += 1;
                }
                SeqEventKind::Channel {
                    status,
                    data,
                    len: len as u8,
                }
            } else if status == 0xff && r.bytes(cursor, 2)? == [0x2f, 0] {
                cursor += 2;
                if cursor != bytes.len() {
                    return Err(DecodeError::new(cursor, "trailing data after SEQ end"));
                }
                SeqEventKind::End
            } else {
                return Err(DecodeError::new(cursor - 1, "unsupported SEQ system event"));
            };
            let ended = kind == SeqEventKind::End;
            events.push(SeqEvent {
                delta_offset,
                after_delta,
                delta,
                tick,
                kind,
            });
            if ended {
                return Ok(Self { header, events });
            }
        }
    }
}
