use crate::time::Beats;

#[derive(Clone, Debug)]
pub struct MidiEvent {
    pub project_time: Beats,
    pub block_time_seconds: f64,
    pub data: MidiEventData,
}

type NoteId = u32;

#[derive(Clone, Debug)]
pub enum MidiEventData {
    NoteOn { note: NoteEvent },
    NoteOff { note: NoteEvent },
    PitchBend { value: u32 },
    ControlChange { controller: u32, value: u32 },
    ProgramChange { program: u32 },
    NoteTuning { note_id: NoteId, tuning: f64 },
    NotePanning { note_id: NoteId, panning: f64 },
    Unknown,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct NoteEvent {
    pub id: u32,
    pub note: u32,
    pub velocity: u32,
    pub voice: u32,
}

impl MidiEvent {
    pub fn new_immediate(data: MidiEventData) -> Self {
        Self {
            project_time: Beats::zero(),
            block_time_seconds: 0.,
            data,
        }
    }

    pub fn new_at(data: MidiEventData, block_time_seconds: f64) -> Self {
        Self {
            project_time: Beats::zero(),
            block_time_seconds,
            data,
        }
    }

    pub fn status_byte(&self) -> Option<u8> {
        match self.data {
            MidiEventData::NoteOn { .. } => Some(0x90),
            MidiEventData::NoteOff { .. } => Some(0x80),
            MidiEventData::PitchBend { .. } => Some(0xE0),
            MidiEventData::ControlChange { .. } => Some(0xB0),
            MidiEventData::ProgramChange { .. } => Some(0xC0),
            _ => None,
        }
    }

    pub fn note(&self) -> Option<NoteEvent> {
        match self.data {
            MidiEventData::NoteOn { note } => Some(note),
            MidiEventData::NoteOff { note } => Some(note),
            _ => None,
        }
    }

    pub fn to_raw_midi_event(&self) -> Option<[u8; 3]> {
        let mut event = [0; 3];

        event[0] = self.status_byte()?;

        match self.data {
            MidiEventData::NoteOn { note } => {
                event[1] = note.note as u8;
                event[2] = note.velocity as u8;
            }
            MidiEventData::NoteOff { note } => {
                event[1] = note.note as u8;
                event[2] = note.velocity as u8;
            }
            MidiEventData::PitchBend { value } => {
                event[1] = (value & 0x7F) as u8;
                event[2] = ((value >> 7) & 0x7F) as u8;
            }
            MidiEventData::ControlChange { controller, value } => {
                event[1] = controller as u8;
                event[2] = value as u8;
            }
            MidiEventData::ProgramChange { program } => {
                event[1] = program as u8;
            }
            _ => {}
        }

        Some(event)
    }
}
