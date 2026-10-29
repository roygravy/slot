//! The studio's label search: words typed on a phone-style keyboard, the games they find, and
//! the one picked, whose label's versions the label tab then shows.
use std::path::PathBuf;

use slot_input::Millis;
use slot_labels::Game;
use slot_store::Platform;
use slot_ui::{accents, key_rows, Key, SearchScreen};

const OPEN_MS: f32 = 160.0;
const CLOSE_MS: f32 = 120.0;
const BLINK_MS: u64 = 530;
/// The most typed: a long name scrolls in the field, its end showing.
const MAX_TYPED: usize = 80;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Press {
    Up,
    Down,
    Left,
    Right,
    /// A: type the key chosen, or pick the game.
    Choose,
    /// B: delete, or go back.
    Back,
    /// X: the chosen letter's accents, or clear.
    Clear,
    /// Y: a space.
    Space,
    /// SELECT: shift.
    Shift,
    /// START: search.
    Search,
    /// MENU: out of the search at once, whatever is on the screen.
    Exit,
}

pub enum Step {
    None,
    /// Look for these words; the answer comes back with `found`, under this search's number.
    Search(u64, String),
    /// This game's label is wanted: the search closes.
    Pick(Game),
    /// Closed without a pick.
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Stage {
    Typing,
    Searching,
    Results(usize),
    Nothing,
}

pub struct LabelSearch {
    pub rom: PathBuf,
    pub platform: Platform,
    text: String,
    row: usize,
    col: usize,
    numbers: bool,
    shift: bool,
    /// The chosen letter's accents are showing, this one chosen.
    accent: Option<usize>,
    stage: Stage,
    games: Vec<Game>,
    /// The search last asked for; an answer to an older one is let go.
    asked: u64,
    /// Bumped whenever a word on the screen changes, for the faces to be made again.
    rev: u64,
    opened: Millis,
    closed: Option<(Millis, f32)>,
    /// When the caret last moved, so it shows at once and blinks from there.
    typed_at: Millis,
}

/// The cart's name as a person would type it: the bracketed parts dropped, and a trailing
/// ", The" too.
pub fn typed_title(stem: &str) -> String {
    let mut depth = 0u32;
    let bare: String = stem
        .chars()
        .filter(|c| match c {
            '(' | '[' => {
                depth += 1;
                false
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect();
    let bare = bare.replace(", The", "").replace('_', " ");
    bare.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TYPED)
        .collect()
}

impl LabelSearch {
    pub fn open(rom: PathBuf, platform: Platform, stem: &str, now: Millis) -> Self {
        LabelSearch {
            rom,
            platform,
            text: typed_title(stem),
            // On the search key, as the title is there to look for already.
            row: 3,
            col: 2,
            numbers: false,
            shift: false,
            accent: None,
            stage: Stage::Typing,
            games: Vec::new(),
            asked: 0,
            rev: 0,
            opened: now,
            closed: None,
            typed_at: now,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }

    pub fn games(&self) -> &[Game] {
        &self.games
    }

    pub fn found_none(&self) -> bool {
        self.stage == Stage::Nothing
    }

    pub fn screen(&self, now: Millis) -> SearchScreen {
        match self.stage {
            Stage::Typing => SearchScreen::Keyboard {
                row: self.row,
                col: self.col,
                numbers: self.numbers,
                shift: self.shift,
                caret: (now.saturating_sub(self.typed_at) / BLINK_MS) % 2 == 0,
                accent: self.accent,
            },
            Stage::Searching => SearchScreen::Searching,
            Stage::Results(at) => SearchScreen::Results { at },
            Stage::Nothing => SearchScreen::Nothing,
        }
    }

    pub fn openness(&self, now: Millis) -> f32 {
        match self.closed {
            Some((at, from)) => (from - now.saturating_sub(at) as f32 / CLOSE_MS).max(0.0),
            None => (now.saturating_sub(self.opened) as f32 / OPEN_MS).min(1.0),
        }
    }

    pub fn closing(&self) -> bool {
        self.closed.is_some()
    }

    pub fn finished(&self, now: Millis) -> bool {
        self.closed.is_some() && self.openness(now) <= 0.0
    }

    fn close(&mut self, now: Millis) {
        self.closed = Some((now, self.openness(now)));
    }

    fn changed(&mut self, now: Millis) {
        self.rev += 1;
        self.typed_at = now;
    }

    /// The games a search found, or None when it could not be made.
    pub fn found(&mut self, asked: u64, games: Option<Vec<Game>>) -> bool {
        if asked != self.asked || self.stage != Stage::Searching {
            return true;
        }
        let Some(games) = games else {
            self.stage = Stage::Typing;
            self.rev += 1;
            return false;
        };
        self.stage = if games.is_empty() {
            Stage::Nothing
        } else {
            Stage::Results(0)
        };
        self.games = games;
        self.rev += 1;
        true
    }

    fn search(&mut self) -> Step {
        let query = self.text.split_whitespace().collect::<Vec<_>>().join(" ");
        if query.is_empty() {
            return Step::None;
        }
        self.asked += 1;
        self.stage = Stage::Searching;
        self.rev += 1;
        Step::Search(self.asked, query)
    }

    fn type_char(&mut self, c: char, now: Millis) {
        if self.text.chars().count() >= MAX_TYPED {
            return;
        }
        // A capital, then back to small letters, as a phone does.
        if self.shift {
            self.text.extend(c.to_uppercase());
            self.shift = false;
        } else {
            self.text.push(c);
        }
        self.changed(now);
    }

    fn delete(&mut self, now: Millis) {
        if self.text.pop().is_some() {
            self.changed(now);
        }
    }

    /// Up or down a row: the key above or below the middle of the one chosen.
    fn move_row(&mut self, by: isize) {
        let rows = key_rows(self.numbers);
        let to = (self.row as isize + by).clamp(0, rows.len() as isize - 1) as usize;
        if to == self.row {
            return;
        }
        let here = &rows[self.row][self.col];
        let mid = here.x + here.w / 2.0;
        self.col = rows[to]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a.x + a.w / 2.0 - mid).abs();
                let db = (b.x + b.w / 2.0 - mid).abs();
                da.total_cmp(&db)
            })
            .map_or(0, |(i, _)| i);
        self.row = to;
    }

    pub fn press(&mut self, press: Press, now: Millis) -> Step {
        if self.closed.is_some() {
            return Step::None;
        }
        if press == Press::Exit {
            // An answer still on its way is let go.
            self.asked += 1;
            self.close(now);
            return Step::Close;
        }
        match self.stage.clone() {
            Stage::Typing => self.typing(press, now),
            Stage::Searching => {
                if press == Press::Back {
                    // The answer is let go when it comes.
                    self.asked += 1;
                    self.stage = Stage::Typing;
                    self.changed(now);
                }
                Step::None
            }
            Stage::Results(at) => match press {
                Press::Up | Press::Down => {
                    let n = self.games.len();
                    let to = if press == Press::Up {
                        at.saturating_sub(1)
                    } else {
                        (at + 1).min(n.saturating_sub(1))
                    };
                    self.stage = Stage::Results(to);
                    Step::None
                }
                Press::Choose => match self.games.get(at).cloned() {
                    Some(game) => {
                        self.close(now);
                        Step::Pick(game)
                    }
                    None => Step::None,
                },
                Press::Back => {
                    self.stage = Stage::Typing;
                    self.changed(now);
                    Step::None
                }
                _ => Step::None,
            },
            Stage::Nothing => {
                if matches!(press, Press::Back | Press::Choose) {
                    self.stage = Stage::Typing;
                    self.changed(now);
                }
                Step::None
            }
        }
    }

    /// The chosen key's accents, when it is a letter that has any.
    fn marks(&self) -> &'static [char] {
        match key_rows(self.numbers)[self.row][self.col].key {
            Key::Char(c) => accents(c),
            _ => &[],
        }
    }

    /// The accents showing: left and right choose, A types one, anything else puts them away.
    fn picking(&mut self, at: usize, press: Press, now: Millis) -> Step {
        let n = self.marks().len();
        match press {
            Press::Left => self.accent = Some((at + n - 1) % n),
            Press::Right => self.accent = Some((at + 1) % n),
            Press::Choose => {
                let c = self.marks()[at];
                self.accent = None;
                self.type_char(c, now);
            }
            Press::Search => {
                self.accent = None;
                return self.search();
            }
            _ => self.accent = None,
        }
        Step::None
    }

    fn typing(&mut self, press: Press, now: Millis) -> Step {
        if let Some(at) = self.accent {
            return self.picking(at, press, now);
        }
        match press {
            Press::Left | Press::Right => {
                let n = key_rows(self.numbers)[self.row].len();
                self.col = if press == Press::Left {
                    (self.col + n - 1) % n
                } else {
                    (self.col + 1) % n
                };
            }
            Press::Up => self.move_row(-1),
            Press::Down => self.move_row(1),
            Press::Back => {
                if self.text.is_empty() {
                    self.close(now);
                    return Step::Close;
                }
                self.delete(now);
            }
            Press::Clear if !self.marks().is_empty() => self.accent = Some(0),
            Press::Clear => {
                if !self.text.is_empty() {
                    self.text.clear();
                    self.changed(now);
                }
            }
            Press::Space => self.type_char(' ', now),
            Press::Shift => self.shift = !self.shift,
            Press::Search => return self.search(),
            Press::Choose => {
                let key = key_rows(self.numbers)[self.row][self.col].key;
                match key {
                    Key::Char(c) => self.type_char(c, now),
                    Key::Shift => self.shift = !self.shift,
                    Key::Delete => self.delete(now),
                    Key::Space => self.type_char(' ', now),
                    Key::Search => return self.search(),
                    Key::Layer => {
                        self.numbers = !self.numbers;
                        self.shift = false;
                    }
                }
            }
            // Taken before the stages.
            Press::Exit => {}
        }
        Step::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(stem: &str) -> LabelSearch {
        LabelSearch::open(PathBuf::from("rom"), Platform::Gb, stem, 0)
    }

    fn game(id: u64) -> Game {
        Game {
            id,
            name: format!("Game {id}"),
            platform: "Game Boy".into(),
        }
    }

    #[test]
    fn the_field_opens_with_the_title_as_people_would_type_it() {
        assert_eq!(
            typed_title("Legend of Zelda, The - Link's Awakening (USA, Europe) [!]"),
            "Legend of Zelda - Link's Awakening"
        );
        assert_eq!(search("Tetris (World) (Rev 1)").text(), "Tetris");
        // A long name is kept whole: the field shows its end.
        assert_eq!(
            typed_title("A Very Long Game Name That Goes On And On (USA)"),
            "A Very Long Game Name That Goes On And On"
        );
    }

    #[test]
    fn keys_type_shift_makes_one_capital_and_b_deletes_then_closes() {
        let mut s = search("");
        s.press(Press::Up, 0);
        s.press(Press::Up, 0);
        s.press(Press::Up, 0);
        // Top row, under the search key's middle: "o".
        s.press(Press::Shift, 0);
        s.press(Press::Choose, 0);
        s.press(Press::Choose, 0);
        assert_eq!(s.text(), "Oo");
        s.press(Press::Space, 0);
        assert_eq!(s.text(), "Oo ");
        s.press(Press::Back, 0);
        // X clears from a key without accents: "p".
        s.press(Press::Right, 0);
        s.press(Press::Clear, 0);
        assert_eq!(s.text(), "");
        assert!(matches!(s.press(Press::Back, 0), Step::Close));
    }

    #[test]
    fn x_shows_a_letters_accents_and_a_types_the_one_chosen() {
        let mut s = search("");
        s.text.clear();
        (s.row, s.col) = (0, 2); // e
        s.press(Press::Clear, 0);
        s.press(Press::Right, 0);
        s.press(Press::Choose, 0);
        assert_eq!(s.text(), "é");
        assert_eq!(s.accent, None);
        // On a key without accents X still clears.
        (s.row, s.col) = (0, 4); // t
        s.press(Press::Clear, 0);
        assert_eq!(s.text(), "");
    }

    #[test]
    fn left_and_right_wrap_around_the_row() {
        let mut s = search("");
        s.press(Press::Right, 0);
        s.press(Press::Choose, 0);
        assert_eq!(
            s.screen(0),
            SearchScreen::Keyboard {
                row: 3,
                col: 0,
                numbers: true,
                shift: false,
                caret: true,
                accent: None,
            }
        );
    }

    #[test]
    fn a_search_lists_what_it_found_and_a_pick_closes_it() {
        let mut s = search("Tetris");
        let Step::Search(asked, words) = s.press(Press::Search, 0) else {
            panic!("a search");
        };
        assert_eq!(words, "Tetris");
        assert_eq!(s.screen(0), SearchScreen::Searching);
        s.found(asked, Some(vec![game(1), game(2)]));
        s.press(Press::Down, 0);
        assert!(matches!(s.press(Press::Choose, 0), Step::Pick(g) if g.id == 2));
        assert!(s.closing());
    }

    #[test]
    fn an_answer_to_a_search_backed_out_of_is_let_go() {
        let mut s = search("Tetris");
        let Step::Search(asked, _) = s.press(Press::Search, 0) else {
            panic!("a search");
        };
        s.press(Press::Back, 0);
        s.found(asked, Some(vec![game(1)]));
        assert!(matches!(s.screen(0), SearchScreen::Keyboard { .. }));
        let Step::Search(again, _) = s.press(Press::Search, 0) else {
            panic!("a search");
        };
        s.found(again, Some(vec![]));
        assert!(s.found_none());
        s.press(Press::Back, 0);
        assert_eq!(
            s.text(),
            "Tetris",
            "back from nothing found keeps the words"
        );
    }
}
