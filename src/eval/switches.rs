use std::fmt;
use std::iter::*;

bitflags! {
    pub struct Switches: u16 {
        const NONE = 0;
        const MATERIAL = 1 << 0;
        const MATERIAL_BALANCE = 1 << 1;
        const POSITION = 1 << 2;
        const MOBILITY = 1 << 3;
        const PAWN = 1 << 4;
        const SAFETY = 1 << 5;
        const CONTEMPT = 1 << 6;
        const TEMPO = 1 << 7;

        const ALL_SCORING = Self::MATERIAL.bits | Self::MATERIAL_BALANCE.bits | Self::POSITION.bits | Self::MOBILITY.bits | Self::PAWN.bits | Self::SAFETY.bits | Self::CONTEMPT.bits | Self::TEMPO.bits;

        const CHECKMATE = 1 << 8;
        const STALEMATE = 1 << 9;
        const INSUFFICIENT_MATERIAL = 1 << 10;

    }
}

impl Default for Switches {
    #[inline]
    fn default() -> Self {
        Self::ALL_SCORING
    }
}

impl Switches {
    pub fn parse(s: &str) -> Result<Switches, String> {
        let mut switches = Switches::NONE;
        for ch in s.chars() {
            match ch {
                'M' => switches |= Self::MATERIAL,
                'B' => switches |= Self::MATERIAL_BALANCE,
                'P' => switches |= Self::POSITION,
                'O' => switches |= Self::MOBILITY,
                'W' => switches |= Self::PAWN,
                'S' => switches |= Self::SAFETY,
                'C' => switches |= Self::CONTEMPT,
                'T' => switches |= Self::TEMPO,
                '-' => break,
                'I' => switches |= Self::INSUFFICIENT_MATERIAL,
                _ => return Err(format!("Invalid character '{}' in eval switches '{}'", ch, s)),
            }
        }
        Ok(switches)
    }

    #[inline]
    pub fn index(&self) -> usize {
        Self::all_scoring().iter().position(|s| s == self).expect(&format!("Switches index called on {}", self))
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::MATERIAL => "material",
            Self::MATERIAL_BALANCE => "material balance",
            Self::POSITION => "position",
            Self::MOBILITY => "mobility",
            Self::PAWN => "pawn",
            Self::SAFETY => "safety",
            Self::CONTEMPT => "contempt",
            Self::TEMPO => "tempo",
            Self::INSUFFICIENT_MATERIAL => "insufficient material",
            Self::CHECKMATE => "checkmate",
            Self::STALEMATE => "stalemate",
            _ => "?",
        }
    }

    #[inline]
    pub fn all_scoring() -> &'static [Self] {
        &[
            Self::MATERIAL,
            Self::POSITION,
            Self::MOBILITY,
            Self::PAWN,
            Self::SAFETY,
            Self::CONTEMPT,
            Self::TEMPO,
        ]
    }
}

impl fmt::Display for Switches {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "-");
        }
        if self.contains(Self::MATERIAL) {
            write!(f, "M")?
        }
        if self.contains(Self::MATERIAL_BALANCE) {
            write!(f, "B")?
        }
        if self.contains(Self::POSITION) {
            write!(f, "P")?
        }
        if self.contains(Self::MOBILITY) {
            write!(f, "O")?
        }
        if self.contains(Self::PAWN) {
            write!(f, "W")?
        }
        if self.contains(Self::SAFETY) {
            write!(f, "S")?
        }
        if self.contains(Self::CONTEMPT) {
            write!(f, "C")?
        }
        if self.contains(Self::TEMPO) {
            write!(f, "T")?
        }
        if self.contains(Self::INSUFFICIENT_MATERIAL) {
            write!(f, "I")?
        }
        Ok(())
    }
}
