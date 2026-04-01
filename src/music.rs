use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
#[repr(C)]
pub struct TimeSignature {
    pub numerator: u32,
    pub denominator: u32,
}

impl fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

impl TimeSignature {
    pub fn common() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }

    pub fn beats_per_measure(&self) -> u32 {
        ((self.numerator as f32 * 4. / (self.denominator.max(1) as f32)).round() as u32).max(1)
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct KeySignature {
    pub root: u32,
    pub mode: KeyMode,
}

impl fmt::Display for KeySignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:?}", note_name(self.root as u8, false), self.mode)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum KeyMode {
    Chromatic,
    Major,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Minor,
    Locrian,
    HarmonicMinor,
    Pentatonic,
    Custom(arrayvec::ArrayVec<i32, 12>),
}

const MAJOR_INTERVALS: [i32; 7] = [2, 2, 1, 2, 2, 2, 1];

fn diatonic_notes(offset: i32) -> arrayvec::ArrayVec<i32, 12> {
    let mut notes = arrayvec::ArrayVec::<i32, 12>::new();
    notes.push(0);
    for i in 0..6 {
        notes.push(notes.last().unwrap() + MAJOR_INTERVALS[((i + offset) % 7) as usize]);
    }
    notes
}

impl KeyMode {
    pub fn parse(s: &str) -> Option<Self> {
        for mode in KeyMode::all() {
            if mode.to_string().to_lowercase() == s.to_lowercase() {
                return Some(mode);
            }
        }

        None
    }

    pub fn to_string(&self) -> String {
        match self {
            KeyMode::Chromatic => "Chrom.",
            KeyMode::Major => "Major",
            KeyMode::Minor => "Minor",
            KeyMode::Dorian => "Dorian",
            KeyMode::Phrygian => "Phrygian",
            KeyMode::Lydian => "Lydian",
            KeyMode::Mixolydian => "Mixol.",
            KeyMode::Locrian => "Locrian",
            KeyMode::HarmonicMinor => "H. Min",
            KeyMode::Pentatonic => "Pent.",
            KeyMode::Custom(_) => "Custom",
        }
        .to_string()
    }

    pub fn all() -> Vec<Self> {
        vec![
            KeyMode::Chromatic,
            KeyMode::Major,
            KeyMode::Minor,
            KeyMode::Dorian,
            KeyMode::Phrygian,
            KeyMode::Lydian,
            KeyMode::Mixolydian,
            KeyMode::Locrian,
            KeyMode::HarmonicMinor,
            KeyMode::Pentatonic,
        ]
    }

    fn intervals(&self) -> arrayvec::ArrayVec<i32, 12> {
        match self {
            KeyMode::Chromatic => (0..12).collect(),
            KeyMode::Major => diatonic_notes(0),
            KeyMode::Minor => diatonic_notes(5),
            KeyMode::Dorian => diatonic_notes(1),
            KeyMode::Phrygian => diatonic_notes(2),
            KeyMode::Lydian => diatonic_notes(3),
            KeyMode::Mixolydian => diatonic_notes(4),
            KeyMode::Locrian => diatonic_notes(6),
            KeyMode::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11].into_iter().collect(),
            KeyMode::Pentatonic => [0, 2, 4, 7, 9].into_iter().collect(),
            KeyMode::Custom(intervals) => intervals.clone(),
        }
    }

    fn degree_to_interval(&self, degree: i32) -> i32 {
        self.intervals()[((degree - 1) % self.len()) as usize]
    }

    fn len(&self) -> i32 {
        self.intervals().len() as i32
    }

    fn interval_to_degree(&self, interval: i32) -> Option<i32> {
        (1..=self.len()).find(|&degree| self.degree_to_interval(degree) == interval)
    }
}

impl KeySignature {
    pub fn new(root: u32, mode: KeyMode) -> Self {
        Self { root, mode }
    }

    pub fn name(&self) -> String {
        format!("{} {:?}", note_name(self.root as u8, false), self.mode)
    }

    pub fn from_degree(&self, mut degree: i32, octave: i32) -> u32 {
        let mut octave_offset = 0;

        let degrees_len = self.mode.len();

        while degree < 1 {
            degree += degrees_len;
            octave_offset -= 1;
        }
        octave_offset += (degree - 1) / degrees_len;

        degree = (degree - 1) % degrees_len + 1;

        let mut note = self.mode.degree_to_interval(degree);

        note += octave_offset * 12;

        let octave = self.root as i32 + octave * 12;
        (note + octave) as u32
    }

    pub fn to_degree(&self, note: i32) -> Option<Degree> {
        let note = note - self.root as i32;

        let octave = note / 12;
        let note = note % 12;
        let degree = self.mode.interval_to_degree(note)?;

        Some(Degree { degree, octave })
    }

    pub fn scale(&self) -> Vec<u32> {
        let mut scale = vec![];

        for i in 0..self.mode.len() {
            scale.push(self.from_degree(i + 1, 0));
        }

        scale
    }

    pub fn to_roman(&self, note: i32) -> String {
        if self.mode == KeyMode::Chromatic {
            return "".to_string();
        }

        if let Some(degree) = self.to_degree(note) {
            let chord_type = self.chord_type_of_degree(degree.degree);

            let mut numeral =
                roman_numeral(degree.degree as u32, chord_type.is_lower_case_notation());

            if chord_type.is_dimished() {
                numeral = format!("{}°", numeral);
            }

            numeral
        } else {
            "".to_string()
        }
    }

    pub fn chord_type_of_degree(&self, degree: i32) -> ChordType {
        match self.mode {
            KeyMode::Major => match degree {
                1 => ChordType::Major,
                2 => ChordType::Minor,
                3 => ChordType::Minor,
                4 => ChordType::Major,
                5 => ChordType::Major,
                6 => ChordType::Minor,
                7 => ChordType::Diminished,
                _ => panic!("Invalid degree"),
            },
            KeyMode::Minor => {
                let ks = KeySignature::new(self.root, KeyMode::Major);
                let degree = (degree + 4) % 7 + 1;
                ks.chord_type_of_degree(degree)
            }
            _ => ChordType::Other,
        }
    }

    pub fn from_notes(notes: &Vec<i32>, modes: Vec<KeyMode>) -> Vec<Self> {
        let mut notes: Vec<i32> = notes.iter().map(|note| *note % 12).collect();
        notes.dedup();

        let mut key_signatures = vec![];

        for root in 0..12 {
            for mode in &modes {
                let key = Self {
                    root,
                    mode: mode.clone(),
                };

                let scale = key.scale().iter().map(|note| note % 12).collect::<Vec<_>>();

                if notes
                    .iter()
                    .all(|note| scale.contains(&(*note).try_into().unwrap_or_default()))
                {
                    key_signatures.push(key);
                }
            }
        }

        key_signatures
    }
}

pub enum ChordType {
    Major,
    Minor,
    Diminished,
    Augmented,
    Suspended2,
    Suspended4,
    Dominant7,
    Major7,
    Minor7,
    Diminished7,
    HalfDiminished7,
    Augmented7,
    AugmentedMajor7,
    Other,
}

impl ChordType {
    fn is_lower_case_notation(&self) -> bool {
        match self {
            ChordType::Major => false,
            ChordType::Minor => true,
            ChordType::Diminished => true,
            ChordType::Augmented => false,
            ChordType::Suspended2 => false,
            ChordType::Suspended4 => false,
            ChordType::Dominant7 => true,
            ChordType::Major7 => false,
            ChordType::Minor7 => true,
            ChordType::Diminished7 => true,
            ChordType::HalfDiminished7 => true,
            ChordType::Augmented7 => false,
            ChordType::AugmentedMajor7 => false,
            ChordType::Other => false,
        }
    }

    fn is_dimished(&self) -> bool {
        match self {
            ChordType::Diminished => true,
            ChordType::Diminished7 => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Degree {
    pub degree: i32,
    pub octave: i32,
}

pub fn parse_note_name(note: &str) -> Option<u8> {
    let upper = note.to_uppercase();
    let mut chars = upper.trim().chars();

    let note = chars.next()?;
    let sharp = chars.next().unwrap_or(' ');

    let mut note = match note {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };

    if sharp == '#' || sharp == '♯' {
        note += 1;
    } else if sharp == 'B' || sharp == '♭' {
        note -= 1;
    } else if sharp != ' ' {
        return None;
    }

    Some(note % 12)
}

pub fn note_name(note: u8, show_octave: bool) -> String {
    let note_names = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];

    let octave = note / 12;
    let note = note % 12;

    if show_octave {
        format!("{}{}", note_names[note as usize], octave)
    } else {
        note_names[note as usize].to_string()
    }
}

pub fn is_black_key(note: i32) -> bool {
    let note = note % 12;
    note == 1 || note == 3 || note == 6 || note == 8 || note == 10
}

pub fn roman_numeral(number: u32, lower_case: bool) -> String {
    let numerals = [
        "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII",
    ];

    let number = number - 1;
    let numeral = numerals.get(number as usize).unwrap_or(&">X");

    if lower_case {
        numeral.to_lowercase()
    } else {
        numeral.to_string()
    }
}
