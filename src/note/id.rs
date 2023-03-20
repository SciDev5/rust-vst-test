

pub struct NoteId {
    pub midi_note: u8,
    pub voice_id: i32,
    pub channel: u8,
}

impl Default for NoteId {
    fn default() -> Self {
        Self {
            midi_note: 0,
            voice_id: 0,
            channel: 0,
        }
    }
}