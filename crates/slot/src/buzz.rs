//! Short buzzes outside a game's own rumble: the charger going in, a warning, an achievement
//! (General > Shelf > Vibration, which also
//! turns a game's rumble off). Each is a few pulses of the rumble motor.

use slot_ui::Toast;

use slot_ui::Millis;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Buzz {
    /// One soft tap: power is in.
    Charger,
    /// Two short taps, a "no".
    Warning,
    /// One tap, a little longer and firmer than the charger's.
    Achievement,
    /// One short tap: GRAVY Manager did what the web page asked (Roy).
    Done,
}

/// (start ms, length ms, strength). The motor needs about 100 ms to be felt, so no pulse is
/// shorter than that.
type Pulse = (Millis, Millis, u16);

impl Buzz {
    fn pulses(self) -> &'static [Pulse] {
        match self {
            Buzz::Charger => &[(0, 140, 0xB000)],
            Buzz::Warning => &[(0, 100, 0xC000), (220, 100, 0xC000)],
            Buzz::Achievement => &[(0, 220, 0xC000)],
            Buzz::Done => &[(0, 120, 0xA000)],
        }
    }

    /// The motor's strength `t` ms into the buzz, or `None` once it is over.
    pub fn strength_at(self, t: Millis) -> Option<u16> {
        let pulses = self.pulses();
        let (start, len, _) = *pulses.last()?;
        if t >= start + len {
            return None;
        }
        Some(
            pulses
                .iter()
                .find(|(s, l, _)| (*s..*s + *l).contains(&t))
                .map_or(0, |p| p.2),
        )
    }

    /// The toasts that warn: something did not work. Plain news (a screenshot saved, a label
    /// found) stays quiet.
    pub fn for_toast(toast: Toast) -> Option<Buzz> {
        // GRAVY Manager's news is the exception: the console sits beside the phone, so a tap
        // says the upload landed without looking (Roy 2026-11-13).
        if matches!(
            toast,
            Toast::GameAdded
                | Toast::GamesAdded
                | Toast::BiosAdded
                | Toast::WallpaperAdded
                | Toast::LabelAdded
                | Toast::FileDeleted
                | Toast::FileRenamed
                | Toast::UpdateReady
        ) {
            return Some(Buzz::Done);
        }
        matches!(
            toast,
            Toast::NeedsGpsp
                | Toast::NoLink
                | Toast::LinkEnded
                | Toast::PeerEnded
                | Toast::BiosMismatch
                | Toast::CardUnwritable
                | Toast::LabelFailed
                | Toast::NoConnection
                | Toast::WifiFailed
                | Toast::BtFailed
                | Toast::BtNotReady
                | Toast::PasswordLength
                | Toast::PasswordMarks
                | Toast::WpsFailed
                | Toast::WrongPassword
                | Toast::GameFailed
                | Toast::UploadFailed
        )
        .then_some(Buzz::Warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_taps_twice_then_ends() {
        let b = Buzz::Warning;
        assert_eq!(b.strength_at(50), Some(0xC000));
        assert_eq!(b.strength_at(150), Some(0));
        assert_eq!(b.strength_at(250), Some(0xC000));
        assert_eq!(b.strength_at(320), None);
    }
}
