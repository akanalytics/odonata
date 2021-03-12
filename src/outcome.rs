use crate::types::Color;

#[derive(Copy, Clone, Debug)]
pub enum Outcome {
    InProgress = 0,
    WinWhite,
    WinBlack,
    WinOnTimeWhite,
    WinOnTimeBlack,
    DrawByAgreement,
    DrawStalemate,
    DrawRepitition3,
    DrawInsufficientMaterial,
    DrawRule50,
    DrawRule75,
}

impl Outcome {
    
    fn is_draw(self) -> bool {
        match self {
            Self::InProgress
            | Self::WinWhite
            | Self::WinBlack
            | Self::WinOnTimeWhite
            | Self::WinOnTimeBlack => false,
            _ => true,
        }
    }

    fn winning_color(self) -> Option<Color> {
        match self {
            | Self::WinWhite
            | Self::WinOnTimeWhite
            => Some(Color::White),

            | Self::WinBlack
            | Self::WinOnTimeBlack => Some(Color::Black),
            _ => None,
        }
    }

    fn is_game_over(self) -> bool {
        self.is_draw() | self.winning_color().is_some()
    }

}
