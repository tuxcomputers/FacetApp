//! The cube's history, from the history characteristic (`...58`): the requests, the frames that answer them, where a
//! fetch resumes from, and which of the frames a fetch brought back may be written.
//!
//! Layout as measured (`docs/timeflip.md` section 5): event number u32 big-endian at 0-3, face at 4 with 128 added
//! while paused, start u64 big-endian seconds at 5-12, duration four bytes at 13-16, and in the streamed form a
//! previous-event pointer at 17-19. A single-event answer is 17 bytes.

/// The most frames one stream is read for.
pub const FRAME_CAP: usize = 2048;

/// Asks for the cube's latest event, answered by a read of the history characteristic.
pub fn request_latest() -> Vec<u8> {
    vec![0x01, 0xFF, 0xFF, 0xFF, 0xFF]
}

/// Asks for the stream of events from `event_number` on, answered by notifications.
pub fn request_from(event_number: u32) -> Vec<u8> {
    let mut bytes = vec![0x02];
    bytes.extend_from_slice(&event_number.to_be_bytes());
    bytes
}

/// One event the cube reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub event_number: u32,
    /// 1 to 12.
    pub face: u8,
    pub is_paused: bool,
    pub start_epoch: u64,
    pub duration_seconds: u32,
}

/// What one frame of history says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryFrameState {
    Event(Frame),
    /// Event number 0: the cube has no such event. On a cube with no history the rest carries its clock, not a
    /// duration (finding 14), so nothing else is read.
    NoSuchEvent,
    /// The first 17 bytes all zero: the end of a stream.
    EndOfStream,
}

/// Reads one frame. `None` for a frame too short, or naming a face that is not 1 to 12.
pub fn parse(bytes: &[u8]) -> Option<HistoryFrameState> {
    if bytes.len() < 17 {
        return None;
    }
    if bytes[..17].iter().all(|byte| *byte == 0) {
        return Some(HistoryFrameState::EndOfStream);
    }
    let event_number = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
    if event_number == 0 {
        return Some(HistoryFrameState::NoSuchEvent);
    }
    let raw_face = bytes[4];
    let (face, is_paused) = if raw_face > 127 { (raw_face - 128, true) } else { (raw_face, false) };
    if !(1..=12).contains(&face) {
        return None;
    }
    let start_epoch = u64::from_be_bytes(bytes[5..13].try_into().ok()?);
    Some(HistoryFrameState::Event(Frame {
        event_number,
        face,
        is_paused,
        start_epoch,
        duration_seconds: duration(bytes[13..17].try_into().ok()?),
    }))
}

/// The duration field: big-endian as measured, read both ways with the smaller non-zero value taken, since firmware
/// has disagreed with the spec about byte order and a plausible duration is small.
fn duration(bytes: [u8; 4]) -> u32 {
    let big = u32::from_be_bytes(bytes);
    let little = u32::from_le_bytes(bytes);
    match (big, little) {
        (0, _) | (_, 0) => big.max(little),
        _ => big.min(little),
    }
}

/// Where a stream starts: at the event on record, when the cube's own latest event is at or after it in both the
/// counter and the clock, and otherwise from 0, the counter having restarted. With no answer from the cube, the
/// event on record stands. `recorded` is `(event_number, start_epoch)`.
pub fn resume_from(recorded: Option<(u32, u64)>, latest: Option<&Frame>) -> u32 {
    match (recorded, latest) {
        (None, _) => 0,
        (Some((number, _)), None) => number,
        (Some((number, start)), Some(latest)) => {
            if latest.event_number >= number && latest.start_epoch >= start { number } else { 0 }
        }
    }
}

/// The frames of a stream to write, in ascending event order with the last one open, or `None` when the batch is to
/// be withheld: its newest event is short of the cube's own `latest`, so the stream was cut short and the last frame
/// may not be the current one. Repeats of an event number keep the copy that came last.
pub fn plan(frames: &[Frame], latest: Option<u32>) -> Option<Vec<Frame>> {
    let mut kept: Vec<Frame> = Vec::new();
    for frame in frames {
        match kept.iter_mut().find(|held| held.event_number == frame.event_number) {
            Some(held) => *held = *frame,
            None => kept.push(*frame),
        }
    }
    kept.sort_by_key(|frame| frame.event_number);
    match (kept.last(), latest) {
        (Some(last), Some(latest)) if last.event_number < latest => None,
        _ => Some(kept),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        text.split_whitespace().map(|pair| u8::from_str_radix(pair, 16).expect("hex")).collect()
    }

    #[test]
    fn the_measured_frames_read_as_measured() {
        // Evidence rows 7353, 7387 and 19561.
        assert_eq!(
            parse(&hex("00 00 00 01 08 00 00 00 00 6A 6E 76 C8 00 00 00 00")),
            Some(HistoryFrameState::Event(Frame {
                event_number: 1,
                face: 8,
                is_paused: false,
                start_epoch: 0x6A6E_76C8,
                duration_seconds: 0
            }))
        );
        let Some(HistoryFrameState::Event(paused)) =
            parse(&hex("00 00 00 02 88 00 00 00 00 6A 6E 76 CA 00 00 00 01 00 00 00"))
        else {
            panic!("a paused frame")
        };
        assert_eq!((paused.face, paused.is_paused, paused.duration_seconds), (8, true, 1));
        let Some(HistoryFrameState::Event(long)) =
            parse(&hex("00 00 00 08 06 00 00 00 00 6A 7D 28 59 00 00 4F 3D"))
        else {
            panic!("a long frame")
        };
        assert_eq!((long.event_number, long.face, long.duration_seconds), (8, 6, 20285));
    }

    #[test]
    fn an_empty_cube_and_the_sentinel_end_a_stream() {
        // Finding 14: an empty cube answers event 0 with its clock where a duration would be.
        assert_eq!(
            parse(&hex("00 00 00 00 00 00 00 00 00 00 00 00 00 6A 6E 76 C8")),
            Some(HistoryFrameState::NoSuchEvent)
        );
        assert_eq!(parse(&[0; 20]), Some(HistoryFrameState::EndOfStream));
        let mut sentinel = vec![0; 17];
        sentinel.extend_from_slice(&[0, 0, 2]);
        assert_eq!(parse(&sentinel), Some(HistoryFrameState::EndOfStream));
        assert_eq!(parse(&[0; 16]), None);
        assert_eq!(parse(&hex("00 00 00 03 42 00 00 00 00 6A 6E 76 CA 00 00 00 01")), None);
    }

    #[test]
    fn the_requests_carry_their_event_number() {
        assert_eq!(request_latest(), vec![0x01, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(request_from(0), vec![0x02, 0, 0, 0, 0]);
        assert_eq!(request_from(258), vec![0x02, 0, 0, 1, 2]);
    }

    fn frame(event_number: u32, start_epoch: u64) -> Frame {
        Frame { event_number, face: 2, is_paused: false, start_epoch, duration_seconds: 10 }
    }

    #[test]
    fn a_stream_resumes_at_the_event_on_record_only_when_the_cube_can_reach_it() {
        assert_eq!(resume_from(None, Some(&frame(9, 900))), 0);
        assert_eq!(resume_from(Some((5, 500)), None), 5);
        assert_eq!(resume_from(Some((5, 500)), Some(&frame(9, 900))), 5);
        // The counter restarted: a lower number, or a later event that began earlier.
        assert_eq!(resume_from(Some((5, 500)), Some(&frame(2, 900))), 0);
        assert_eq!(resume_from(Some((5, 500)), Some(&frame(5, 400))), 0);
    }

    #[test]
    fn a_batch_is_sorted_deduplicated_and_withheld_when_cut_short() {
        let later = Frame { duration_seconds: 20, ..frame(3, 300) };
        let planned = plan(&[frame(3, 300), frame(1, 100), later, frame(2, 200)], Some(3)).expect("complete");
        assert_eq!(planned.iter().map(|frame| frame.event_number).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(planned[2].duration_seconds, 20);
        assert_eq!(plan(&[frame(1, 100), frame(2, 200)], Some(3)), None);
        assert_eq!(plan(&[], None), Some(vec![]));
    }
}
