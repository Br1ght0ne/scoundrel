use std::{fmt::Write, ops::Neg};

use cardpack::{Card, Named, Pile, Rank, Suit, CLUBS, HEARTS, SPADES};
use color_eyre::{eyre::bail, Result};
use dialoguer::{Confirm, Select};

const MAX_HEALTH: u8 = 20;
const ROOM_SIZE: usize = 4;

struct GameResult {
    outcome: Outcome,
    score: i32,
}

#[derive(Debug)]
enum Outcome {
    Won,
    Lost,
}

struct Game {
    dungeon: Pile,
    weapon: Option<u8>,
    weakest_killed: Option<u8>,
    just_avoided_room: bool,
    room: Pile,
    health: u8,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("dungeon finished")]
    DungeonFinished,
    #[error("room unfinished")]
    RoomUnfinished,
    #[error("invalid card suit")]
    InvalidCardSuit,
}

fn fold_in(cards: &mut Pile, suits: &[Suit], ranks: &[Rank]) {
    for suit in suits {
        for rank in ranks {
            cards.push(Card::new(*rank, *suit));
        }
    }
}

impl Game {
    fn setup() -> Self {
        use cardpack::cards::{rank::*, suit::*};

        let all_ranks = Rank::generate_french_ranks();
        let numbered_ranks =
            Rank::from_array(&[TEN, NINE, EIGHT, SEVEN, SIX, FIVE, FOUR, THREE, TWO]);
        let black_suits = Suit::from_array(&[SPADES, CLUBS]);
        let red_suits = Suit::from_array(&[HEARTS, DIAMONDS]);

        let mut dungeon: Pile = Pile::default();
        fold_in(&mut dungeon, &black_suits, &all_ranks);
        fold_in(&mut dungeon, &red_suits, &numbered_ranks);

        Self {
            dungeon: dungeon.shuffle(),
            weapon: None,
            weakest_killed: None,
            just_avoided_room: false,
            room: Pile::default(),
            health: MAX_HEALTH,
        }
    }

    fn prompt(&self, question: Option<&str>) -> Result<String> {
        let mut prompt = format!(
            "H: {:>2}, W: {:>2} (M: {:>2}), D: {:>2}, R: {}",
            self.health,
            self.weapon.unwrap_or_default(),
            self.weakest_killed.unwrap_or_default(),
            self.dungeon.len() - (ROOM_SIZE - self.room.len()),
            self.room
        );
        if let Some(q) = question {
            write!(prompt, ", {q}")?;
        }
        Ok(prompt)
    }

    fn play(&mut self) -> Result<GameResult> {
        loop {
            self.enter()?;
            if !self.just_avoided_room {
                let avoid = Confirm::new()
                    .with_prompt(self.prompt(Some("avoid?"))?)
                    .interact()?;
                if avoid {
                    self.dungeon.append(&self.room);
                    self.just_avoided_room = true;
                    self.room = Pile::default();
                    continue;
                }
            }
            self.just_avoided_room = false;
            while self.room.len() > 1 {
                let selection = Select::new()
                    .with_prompt(self.prompt(None)?)
                    .items(self.room.cards())
                    .interact()?;
                let card = self.room.get(selection).unwrap().clone();
                self.apply_card(&card)?;
                if self.health == 0 {
                    return Ok(GameResult {
                        outcome: Outcome::Lost,
                        score: (self
                            .dungeon
                            .cards()
                            .iter()
                            .filter(|card| card.suit.name() == SPADES || card.suit.name() == CLUBS)
                            .map(weight)
                            .collect::<Result<Vec<_>>>()?
                            .into_iter()
                            .sum::<u8>() as i32)
                            .neg(),
                    });
                }
                self.room.remove(selection);
                if self.dungeon.is_empty() && self.room.is_empty() {
                    return Ok(GameResult {
                        outcome: Outcome::Won,
                        score: (self.health
                            + (if card.suit.name() == HEARTS {
                                weight(&card)?
                            } else {
                                0
                            })) as i32,
                    });
                }
            }
        }
    }

    fn enter(&mut self) -> Result<()> {
        if self.room.len() > 1 {
            bail!(Error::RoomUnfinished);
        }
        if self.dungeon.is_empty() {
            bail!(Error::DungeonFinished);
        }
        let new_cards = self
            .dungeon
            .draw(self.dungeon.len().min(ROOM_SIZE - self.room.len()))
            .unwrap();
        for card in new_cards {
            self.room.push(card);
        }
        debug_assert_eq!(self.room.len(), ROOM_SIZE);
        Ok(())
    }

    fn apply_card(&mut self, card: &Card) -> Result<()> {
        match card.suit.name.index_default().as_str() {
            "D" => self.equip(weight(card)?),
            "S" | "C" => self.fight(weight(card)?),
            "H" => self.heal(weight(card)?),
            _ => bail!(Error::InvalidCardSuit),
        }
    }

    fn equip(&mut self, weapon: u8) -> Result<()> {
        self.weapon = Some(weapon);
        self.weakest_killed = None;
        Ok(())
    }

    fn fight(&mut self, monster: u8) -> Result<()> {
        let blocked = if let Some(weapon) = self.weapon {
            if self
                .weakest_killed
                .map(|prev| monster < prev)
                .unwrap_or(true)
                && Confirm::new()
                    .with_prompt(self.prompt(Some("use weapon?"))?)
                    .interact()?
            {
                self.weakest_killed = Some(monster);
                weapon
            } else {
                0
            }
        } else {
            0
        };

        let damage = monster.saturating_sub(blocked);
        self.health = self.health.saturating_sub(damage);
        Ok(())
    }

    fn heal(&mut self, potion: u8) -> Result<()> {
        self.health = MAX_HEALTH.min(self.health + potion);
        Ok(())
    }
}

fn weight(card: &Card) -> Result<u8> {
    let weight = match card.rank.index_default().as_str() {
        "T" => 10,
        "J" => 11,
        "Q" => 12,
        "K" => 13,
        "A" => 14,
        r => r.parse()?,
    };
    Ok(weight)
}

fn main() -> Result<()> {
    let mut game = Game::setup();
    let result = game.play()?;
    println!("{:?}! Score: {}", result.outcome, result.score);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup() {
        let game = Game::setup();
        assert_eq!(game.dungeon.len(), 44);
    }
}
