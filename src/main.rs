use rand::Rng;
use slint::Weak;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

include!(env!("SLINT_INCLUDE_GENERATED"));

const SMALL_BLIND_CHIPS: u64 = 10;
const BIG_BLIND_CHIPS: u64 = 20;
const INITIAL_CHIPS: u64 = 10000;
const MIN_CHIPS_TO_CONTINUE: u64 = 10;
const MIN_BET_DEFAULT: u64 = 20;
const MAX_BET_DEFAULT: u64 = 500;
const MAX_BET_MULTIPLIER: u64 = 100;
const CALL_AMOUNT_DEFAULT: u64 = 50;
const NUM_PLAYERS: usize = 2;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameStage {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    HandComplete,
    WaitingToStart,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    AllIn,
}

#[derive(Clone, Copy, PartialEq, Debug, PartialOrd, Ord, Eq)]
pub enum HandRank {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}

#[derive(Clone, Copy, PartialEq, Debug, PartialOrd, Ord, Eq, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub fn to_char(self) -> char {
        match self {
            Suit::Spades => '♠',
            Suit::Hearts => '♥',
            Suit::Diamonds => '♦',
            Suit::Clubs => '♣',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '♠' => Some(Suit::Spades),
            '♥' => Some(Suit::Hearts),
            '♦' => Some(Suit::Diamonds),
            '♣' => Some(Suit::Clubs),
            _ => None,
        }
    }

    pub fn is_red(self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, PartialOrd, Ord, Eq)]
pub struct Card {
    pub rank: u8,
    pub suit: Suit,
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rank_str = match self.rank {
            14 => "A",
            13 => "K",
            12 => "Q",
            11 => "J",
            10 => "10",
            n => return write!(f, "{}{}", n, self.suit.to_char()),
        };
        write!(f, "{}{}", rank_str, self.suit.to_char())
    }
}

impl Card {
    pub fn new(rank: u8, suit: Suit) -> Self {
        Card { rank, suit }
    }

    pub fn is_red(self) -> bool {
        self.suit.is_red()
    }

    pub fn from_string(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        let suit_char = s.chars().last()?;
        let suit = Suit::from_char(suit_char)?;

        let last_char_byte_idx = s.char_indices().next_back()?.0;
        let rank_part = &s[..last_char_byte_idx];

        let rank = match rank_part {
            "A" => 14,
            "K" => 13,
            "Q" => 12,
            "J" => 11,
            _ => rank_part.parse().ok()?,
        };

        if !(2..=14).contains(&rank) {
            return None;
        }

        Some(Card::new(rank, suit))
    }
}

#[derive(Clone)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs] {
            for rank in 2..=14 {
                cards.push(Card::new(rank, suit));
            }
        }
        Deck { cards }
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        for i in (1..self.cards.len()).rev() {
            let j = rng.gen_range(0..=i);
            self.cards.swap(i, j);
        }
    }

    pub fn deal(&mut self, count: usize) -> Option<Vec<Card>> {
        if count > self.cards.len() {
            return None;
        }
        Some(self.cards.drain(0..count).collect())
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn burn(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}

#[derive(Clone)]
pub struct Player {
    name: String,
    chips: u64,
    hole_cards: Vec<Card>,
    current_bet: u64,
    is_folded: bool,
    is_all_in: bool,
    has_acted: bool,
}

impl Player {
    pub fn new(name: String, chips: u64) -> Self {
        Player {
            name,
            chips,
            hole_cards: Vec::with_capacity(2),
            current_bet: 0,
            is_folded: false,
            is_all_in: false,
            has_acted: false,
        }
    }

    pub fn receive_cards(&mut self, cards: Vec<Card>) {
        self.hole_cards.extend(cards);
    }

    /// Place a bet, deducting chips from player's stack.
    ///
    /// # Errors
    ///
    /// Returns `Err("Insufficient chips")` if the bet amount exceeds available chips.
    pub fn bet(&mut self, amount: u64) -> Result<u64, &'static str> {
        if amount > self.chips {
            return Err("Insufficient chips");
        }
        self.chips -= amount;
        self.current_bet += amount;
        if self.chips == 0 {
            self.is_all_in = true;
        }
        self.has_acted = true;
        Ok(amount)
    }

    pub fn collect_pot(&mut self, amount: u64) {
        self.chips += amount;
    }

    pub fn reset_for_new_hand(&mut self) {
        self.hole_cards.clear();
        self.current_bet = 0;
        self.is_folded = false;
        self.is_all_in = false;
        self.has_acted = false;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_chips(&self) -> u64 {
        self.chips
    }

    pub fn get_current_bet(&self) -> u64 {
        self.current_bet
    }

    pub fn is_folded(&self) -> bool {
        self.is_folded
    }

    pub fn set_folded(&mut self, folded: bool) {
        self.is_folded = folded;
    }

    pub fn is_all_in(&self) -> bool {
        self.is_all_in
    }

    pub fn has_acted(&self) -> bool {
        self.has_acted
    }

    pub fn set_has_acted(&mut self, acted: bool) {
        self.has_acted = acted;
    }

    pub fn get_hole_cards(&self) -> &[Card] {
        &self.hole_cards
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvaluatedHand {
    pub rank: HandRank,
    pub primary_values: Vec<u8>,
    pub kickers: Vec<u8>,
}

impl EvaluatedHand {
    pub fn new(rank: HandRank, primary_values: Vec<u8>, kickers: Vec<u8>) -> Self {
        EvaluatedHand {
            rank,
            primary_values,
            kickers,
        }
    }

    pub fn compare(&self, other: &EvaluatedHand) -> std::cmp::Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| self.primary_values.cmp(&other.primary_values))
            .then_with(|| self.kickers.cmp(&other.kickers))
    }
}

pub struct PokerHandEvaluator;

impl PokerHandEvaluator {
    pub fn evaluate(hole_cards: &[Card], community_cards: &[Card]) -> EvaluatedHand {
        let mut all_cards: Vec<Card> = Vec::with_capacity(hole_cards.len() + community_cards.len());
        all_cards.extend(hole_cards);
        all_cards.extend(community_cards);

        if all_cards.len() < 5 {
            return EvaluatedHand::new(HandRank::HighCard, Vec::new(), Vec::new());
        }

        let mut ranks: Vec<u8> = all_cards.iter().map(|c| c.rank).collect();
        ranks.sort_unstable_by(|a, b| b.cmp(a));
        let mut ranks_dedup = ranks.clone();
        ranks_dedup.dedup();
        let suits: Vec<Suit> = all_cards.iter().map(|c| c.suit).collect();

        let suit_counts: HashMap<Suit, usize> =
            suits.iter().fold(HashMap::new(), |mut map, &suit| {
                *map.entry(suit).or_insert(0) += 1;
                map
            });

        let rank_counts: HashMap<u8, usize> =
            ranks.iter().fold(HashMap::new(), |mut map, &rank| {
                *map.entry(rank).or_insert(0) += 1;
                map
            });

        let four_of_kind: Vec<u8> = rank_counts
            .iter()
            .filter(|(_, &count)| count == 4)
            .map(|(&rank, _)| rank)
            .collect();
        if !four_of_kind.is_empty() {
            let four_rank = four_of_kind[0];
            let kickers: Vec<u8> = ranks
                .iter()
                .copied()
                .filter(|&r| r != four_rank)
                .take(1)
                .collect();
            return EvaluatedHand::new(HandRank::FourOfAKind, vec![four_rank], kickers);
        }

        let full_house_ranks: Vec<u8> = rank_counts
            .iter()
            .filter(|(_, &count)| count >= 2)
            .map(|(&rank, _)| rank)
            .collect();
        if full_house_ranks.len() >= 2 {
            let three_rank = full_house_ranks.iter().find(|&&r| rank_counts[&r] >= 3);
            if let Some(&three) = three_rank {
                let pair_rank = full_house_ranks
                    .iter()
                    .find(|&&r| r != three && rank_counts[&r] >= 2);
                if let Some(&pair) = pair_rank {
                    return EvaluatedHand::new(HandRank::FullHouse, vec![three, pair], Vec::new());
                }
            }
        }

        let flush_suit = suit_counts
            .iter()
            .find(|(_, &count)| count >= 5)
            .map(|(&suit, _)| suit);
        if let Some(suit) = flush_suit {
            let mut flush_cards: Vec<u8> = all_cards
                .iter()
                .filter(|c| c.suit == suit)
                .map(|c| c.rank)
                .collect();
            flush_cards.sort_unstable_by(|a, b| b.cmp(a));

            let is_straight = Self::check_straight(&flush_cards);
            let top_flush_cards: Vec<u8> = flush_cards.into_iter().take(5).collect();

            if is_straight {
                if top_flush_cards[0] == 14 && top_flush_cards[1] == 13 {
                    return EvaluatedHand::new(HandRank::RoyalFlush, vec![14], Vec::new());
                }
                return EvaluatedHand::new(HandRank::StraightFlush, top_flush_cards, Vec::new());
            }

            return EvaluatedHand::new(HandRank::Flush, top_flush_cards, Vec::new());
        }

        let straight = Self::check_straight(&ranks_dedup);
        if straight {
            let primary_values =
                if ranks_dedup.contains(&14) && ranks_dedup.iter().take(5).any(|&r| r < 5) {
                    vec![5, 4, 3, 2, 1]
                } else {
                    ranks_dedup.iter().take(5).copied().collect()
                };
            return EvaluatedHand::new(HandRank::Straight, primary_values, Vec::new());
        }

        let three_of_kind: Vec<u8> = rank_counts
            .iter()
            .filter(|(_, &count)| count == 3)
            .map(|(&rank, _)| rank)
            .collect();
        if !three_of_kind.is_empty() {
            let three_rank = three_of_kind[0];
            let kickers: Vec<u8> = ranks
                .iter()
                .copied()
                .filter(|&r| r != three_rank)
                .take(2)
                .collect();
            return EvaluatedHand::new(HandRank::ThreeOfAKind, vec![three_rank], kickers);
        }

        let two_pair_ranks: Vec<u8> = rank_counts
            .iter()
            .filter(|(_, &count)| count == 2)
            .map(|(&rank, _)| rank)
            .collect();
        if !two_pair_ranks.is_empty() {
            if two_pair_ranks.len() >= 2 {
                let mut pairs = two_pair_ranks;
                pairs.sort_unstable_by(|a, b| b.cmp(a));
                let first_pair = pairs[0];
                let second_pair = pairs[1];
                let kicker: Vec<u8> = ranks
                    .iter()
                    .copied()
                    .filter(|&r| r != first_pair && r != second_pair)
                    .take(1)
                    .collect();
                return EvaluatedHand::new(
                    HandRank::TwoPair,
                    vec![first_pair, second_pair],
                    kicker,
                );
            }

            let pair_rank = two_pair_ranks[0];
            let kickers: Vec<u8> = ranks
                .iter()
                .copied()
                .filter(|&r| r != pair_rank)
                .take(3)
                .collect();
            return EvaluatedHand::new(HandRank::Pair, vec![pair_rank], kickers);
        }

        let kickers: Vec<u8> = ranks.into_iter().take(5).collect();
        EvaluatedHand::new(HandRank::HighCard, Vec::new(), kickers)
    }

    fn check_straight(ranks: &[u8]) -> bool {
        if ranks.len() < 5 {
            return false;
        }

        let mut sorted_ranks = ranks.to_vec();
        sorted_ranks.sort_unstable();
        sorted_ranks.dedup();

        if Self::has_consecutive_window(&sorted_ranks, 5) {
            return true;
        }

        if sorted_ranks.contains(&14) {
            let mut ace_low_ranks: Vec<u8> = sorted_ranks
                .iter()
                .copied()
                .map(|r| if r == 14 { 1 } else { r })
                .collect();
            ace_low_ranks.sort_unstable();
            ace_low_ranks.dedup();
            if Self::has_consecutive_window(&ace_low_ranks, 5) {
                return true;
            }
        }

        false
    }

    fn has_consecutive_window(ranks: &[u8], window_size: usize) -> bool {
        for i in 0..=ranks.len().saturating_sub(window_size) {
            let window = &ranks[i..i + window_size];
            if window.windows(2).all(|w| w[1] == w[0] + 1) {
                return true;
            }
        }
        false
    }
}

#[derive(Clone)]
pub struct PokerGame {
    deck: Deck,
    players: Vec<Player>,
    community_cards: Vec<Card>,
    pot: u64,
    stage: GameStage,
    dealer_position: usize,
    current_player: usize,
    to_call: u64,
    game_weak: Weak<PokerApp>,
    pending_action: bool,
    game_rc: Option<Rc<RefCell<PokerGame>>>,
    bet_amount: u64,
    min_bet: u64,
    max_bet: u64,
    last_aggressor: Option<usize>,
    pot_commitments: Vec<u64>,
    pot_odds: f32,
}

impl PokerGame {
    pub fn new(game_weak: Weak<PokerApp>) -> Self {
        let mut deck = Deck::new();
        deck.shuffle();

        let mut players = Vec::with_capacity(NUM_PLAYERS);
        players.push(Player::new("Alice".to_string(), INITIAL_CHIPS));
        players.push(Player::new("Bob".to_string(), INITIAL_CHIPS));

        PokerGame {
            deck,
            players,
            community_cards: Vec::with_capacity(5),
            pot: 0,
            stage: GameStage::WaitingToStart,
            dealer_position: 0,
            current_player: 0,
            to_call: 0,
            game_weak,
            pending_action: false,
            game_rc: None,
            bet_amount: CALL_AMOUNT_DEFAULT,
            min_bet: MIN_BET_DEFAULT,
            max_bet: MAX_BET_DEFAULT,
            last_aggressor: None,
            pot_commitments: vec![0; NUM_PLAYERS],
            pot_odds: 0.0,
        }
    }

    /// Start a new hand, dealing cards to all players.
    ///
    /// # Errors
    ///
    /// Returns `Err("Not enough players with sufficient chips")` if fewer than 2 players
    /// have at least MIN_CHIPS_TO_CONTINUE chips.
    pub fn start_new_hand(&mut self) -> Result<(), &'static str> {
        if self
            .players
            .iter()
            .filter(|p| p.get_chips() >= MIN_CHIPS_TO_CONTINUE)
            .count()
            < 2
        {
            return Err("Not enough players with sufficient chips");
        }

        for player in &mut self.players {
            player.reset_for_new_hand();
        }

        self.deck = Deck::new();
        self.deck.shuffle();

        self.community_cards.clear();

        self.deck.burn();

        for player in &mut self.players {
            if let Some(cards) = self.deck.deal(2) {
                player.receive_cards(cards);
            } else {
                return Err("Failed to deal hole cards");
            }
        }

        self.pot = 0;
        self.pot_commitments = vec![0; self.players.len()];
        self.last_aggressor = None;

        self.post_blinds()?;

        self.stage = GameStage::Preflop;
        self.current_player = (self.dealer_position + 3) % self.players.len();
        self.to_call = BIG_BLIND_CHIPS;
        self.pending_action = true;
        self.update_action_bounds();

        self.update_ui("New hand started".to_string());

        Ok(())
    }

    fn post_blinds(&mut self) -> Result<(), &'static str> {
        let sb_position = (self.dealer_position + 1) % self.players.len();
        let bb_position = (self.dealer_position + 2) % self.players.len();

        self.players[sb_position].bet(SMALL_BLIND_CHIPS)?;
        self.players[bb_position].bet(BIG_BLIND_CHIPS)?;

        self.pot += SMALL_BLIND_CHIPS + BIG_BLIND_CHIPS;
        self.pot_commitments[sb_position] += SMALL_BLIND_CHIPS;
        self.pot_commitments[bb_position] += BIG_BLIND_CHIPS;

        Ok(())
    }

    fn update_action_bounds(&mut self) {
        if self.players.is_empty() || self.current_player >= self.players.len() {
            return;
        }

        let current_call = self.to_call;
        let player = &self.players[self.current_player];

        let min_raise = current_call.saturating_mul(2);
        let player_chips = player.get_chips();

        self.min_bet = if current_call == 0 {
            BIG_BLIND_CHIPS
        } else {
            min_raise
        };

        self.max_bet = std::cmp::min(
            player_chips,
            MAX_BET_DEFAULT.saturating_mul(MAX_BET_MULTIPLIER),
        );

        if self.bet_amount < self.min_bet {
            self.bet_amount = self.min_bet;
        }
        if self.bet_amount > self.max_bet {
            self.bet_amount = self.max_bet;
        }
    }

    /// Process a player's action (fold, check, call, bet, raise, all-in).
    ///
    /// # Errors
    ///
    /// Returns various errors based on the action type and game state.
    pub fn perform_action(&mut self, action: PlayerAction) -> Result<(), &'static str> {
        if !self.pending_action {
            return Err("No pending action");
        }

        let player_idx = self.current_player;
        let player_name = self.players[player_idx].get_name().to_string();
        let player = &self.players[player_idx];

        if player.is_folded() || player.is_all_in() {
            return Err("Player cannot act");
        }

        let current_bet = player.get_current_bet();
        let call_amount = self.to_call.saturating_sub(current_bet);

        match action {
            PlayerAction::Fold => {
                self.players[player_idx].set_folded(true);
                self.update_ui(format!("{player_name} folded"));
            }

            PlayerAction::Check => {
                if call_amount > 0 {
                    return Err("Cannot check when a bet is pending");
                }
                self.players[player_idx].set_has_acted(true);
                self.update_ui(format!("{player_name} checked"));
            }

            PlayerAction::Call => {
                let actual_call = std::cmp::min(call_amount, player.get_chips());
                self.players[player_idx].bet(actual_call)?;
                self.pot += actual_call;
                self.pot_commitments[player_idx] += actual_call;
                self.players[player_idx].set_has_acted(true);
                self.update_ui(format!("{player_name} called {actual_call}"));
            }

            PlayerAction::Bet => {
                if call_amount > 0 {
                    return Err("Use Raise action instead of Bet when a bet is pending");
                }
                let bet_amount = std::cmp::min(self.bet_amount, player.get_chips());
                self.players[player_idx].bet(bet_amount)?;
                self.to_call = bet_amount;
                self.pot += bet_amount;
                self.pot_commitments[player_idx] += bet_amount;
                self.last_aggressor = Some(player_idx);
                self.update_ui(format!("{player_name} bet {bet_amount}"));
            }

            PlayerAction::Raise => {
                let raise_amount = std::cmp::min(self.bet_amount, player.get_chips());
                let total_bet = current_bet + raise_amount;
                if total_bet <= self.to_call {
                    return Err("Raise must be greater than current bet");
                }
                self.players[player_idx].bet(raise_amount)?;
                self.to_call = total_bet;
                self.pot += raise_amount;
                self.pot_commitments[player_idx] += raise_amount;
                self.last_aggressor = Some(player_idx);
                self.update_ui(format!("{player_name} raised to {total_bet}"));
            }

            PlayerAction::AllIn => {
                let all_in_amount = player.get_chips();
                self.players[player_idx].bet(all_in_amount)?;
                self.pot += all_in_amount;
                self.pot_commitments[player_idx] += all_in_amount;
                if current_bet + all_in_amount > self.to_call {
                    self.to_call = current_bet + all_in_amount;
                    self.last_aggressor = Some(player_idx);
                }
                self.update_ui(format!("{player_name} went all-in with {all_in_amount}"));
            }
        }

        self.advance_to_next_player();

        Ok(())
    }

    fn advance_to_next_player(&mut self) {
        let player_count = self.players.len();
        let mut attempts = 0;

        while attempts < player_count {
            self.current_player = (self.current_player + 1) % player_count;
            let player = &self.players[self.current_player];

            if player.is_folded() || player.is_all_in() {
                attempts += 1;
                continue;
            }

            break;
        }

        self.check_street_complete();
    }

    fn check_street_complete(&mut self) {
        let active_players: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_folded() && !p.is_all_in())
            .map(|(i, _)| i)
            .collect();

        let all_folded = active_players.len() <= 1;

        if all_folded {
            self.end_hand("All opponents folded".to_string());
            return;
        }

        let all_acted = active_players.iter().all(|&i| self.players[i].has_acted());

        let bets_equal = active_players
            .iter()
            .all(|&i| self.players[i].get_current_bet() == self.to_call);

        if all_acted && bets_equal {
            self.advance_street();
        }
    }

    fn advance_street(&mut self) {
        for player in &mut self.players {
            player.set_has_acted(false);
        }

        self.to_call = 0;

        match self.stage {
            GameStage::Preflop => {
                self.deal_community_cards(3);
                self.stage = GameStage::Flop;
            }
            GameStage::Flop => {
                self.deal_community_cards(1);
                self.stage = GameStage::Turn;
            }
            GameStage::Turn => {
                self.deal_community_cards(1);
                self.stage = GameStage::River;
            }
            GameStage::River => {
                self.stage = GameStage::Showdown;
                self.determine_winner();
            }
            _ => {
                self.end_hand("Hand complete".to_string());
            }
        }

        self.current_player = (self.dealer_position + 1) % self.players.len();
        self.pending_action = true;
        self.update_action_bounds();

        let stage_name = match self.stage {
            GameStage::Flop => "Flop",
            GameStage::Turn => "Turn",
            GameStage::River => "River",
            _ => "Unknown",
        };
        self.update_ui(format!("{} - {}", stage_name, self.get_stage_string()));
    }

    fn deal_community_cards(&mut self, count: usize) {
        self.deck.burn();
        for _ in 0..count {
            if let Some(card) = self.deck.deal(1) {
                self.community_cards.extend(card);
            }
        }
    }

    fn determine_winner(&mut self) {
        let active_players: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_folded())
            .map(|(i, _)| i)
            .collect();

        if active_players.len() == 1 {
            let winner_idx = active_players[0];
            let winnings = self.pot;
            self.players[winner_idx].collect_pot(winnings);
            self.update_ui(format!(
                "{} wins {} (all folded)",
                self.players[winner_idx].get_name(),
                winnings
            ));
            self.end_hand("Hand complete".to_string());
        }

        let mut best_hand: Option<EvaluatedHand> = None;
        let mut winners: Vec<usize> = Vec::new();

        for &player_idx in &active_players {
            let hand = PokerHandEvaluator::evaluate(
                self.players[player_idx].get_hole_cards(),
                &self.community_cards,
            );

            if let Some(ref best) = best_hand {
                match hand.cmp(best) {
                    std::cmp::Ordering::Greater => {
                        best_hand = Some(hand);
                        winners.clear();
                        winners.push(player_idx);
                    }
                    std::cmp::Ordering::Equal => {
                        winners.push(player_idx);
                    }
                    std::cmp::Ordering::Less => {}
                }
            } else {
                best_hand = Some(hand);
                winners.push(player_idx);
            }
        }

        if winners.is_empty() {
            self.update_ui("Error determining winner".to_string());
            self.end_hand("Hand complete".to_string());
        }

        let split_amount = self.pot / winners.len() as u64;
        let remainder = self.pot % winners.len() as u64;

        for &winner_idx in &winners {
            self.players[winner_idx].collect_pot(split_amount);
        }

        let winner_names: Vec<String> = winners
            .iter()
            .map(|&i| self.players[i].get_name().to_string())
            .collect();

        if winners.len() == 1 {
            let winner_idx = winners[0];
            let actual_winnings = split_amount + remainder;
            self.players[winner_idx].collect_pot(remainder);
            self.update_ui(format!(
                "{} wins {} with {:?}",
                winner_names.join(", "),
                actual_winnings,
                best_hand.unwrap().rank
            ));
        } else {
            self.update_ui(format!(
                "{} split the pot ({})",
                winner_names.join(" & "),
                split_amount
            ));
        }

        self.end_hand("Hand complete".to_string());
    }

    fn end_hand(&mut self, message: String) {
        self.stage = GameStage::HandComplete;
        self.dealer_position = (self.dealer_position + 1) % self.players.len();
        self.update_ui(message);
    }

    pub fn update_ui(&mut self, message: String) {
        if let Some(ui) = self.game_weak.upgrade() {
            if !self.players.is_empty() {
                ui.set_player1_name(self.players[0].get_name().into());
                ui.set_player1_chips(self.players[0].get_chips() as f32);
                ui.set_p1_current_bet(self.players[0].get_current_bet() as f32);
                ui.set_p1_folded(self.players[0].is_folded());
            }
            if self.players.len() >= 2 {
                ui.set_player2_name(self.players[1].get_name().into());
                ui.set_player2_chips(self.players[1].get_chips() as f32);
                ui.set_p2_current_bet(self.players[1].get_current_bet() as f32);
                ui.set_p2_folded(self.players[1].is_folded());
            }
            ui.set_pot_size(self.pot as f32);
            ui.set_pot_odds(self.pot_odds);
            ui.set_game_stage(self.get_stage_string().into());
            ui.set_dealer_position(self.dealer_position as i32);
            ui.set_message(message.into());

            self.update_player_cards(&ui);
            self.update_community_cards(&ui);
            self.update_player_status(&ui);
            self.update_action_controls(&ui);
        }
    }

    fn get_stage_string(&self) -> String {
        match self.stage {
            GameStage::Preflop => "Preflop",
            GameStage::Flop => "Flop",
            GameStage::Turn => "Turn",
            GameStage::River => "River",
            GameStage::Showdown => "Showdown",
            GameStage::HandComplete => "Complete",
            GameStage::WaitingToStart => "Waiting",
        }
        .to_string()
    }

    fn update_player_cards(&self, ui: &PokerApp) {
        if !self.players.is_empty() {
            ui.set_p1_card1(self.hole_card_string(0, 0));
            ui.set_p1_card2(self.hole_card_string(0, 1));
            ui.set_p1_card1_red(self.hole_card_red(0, 0));
            ui.set_p1_card2_red(self.hole_card_red(0, 1));
        }
        if self.players.len() >= 2 {
            ui.set_p2_card1(self.hole_card_string(1, 0));
            ui.set_p2_card2(self.hole_card_string(1, 1));
            ui.set_p2_card1_red(self.hole_card_red(1, 0));
            ui.set_p2_card2_red(self.hole_card_red(1, 1));
        }
    }

    fn get_hole_card(&self, player_idx: usize, card_idx: usize) -> Option<&Card> {
        self.players
            .get(player_idx)
            .and_then(|p| p.get_hole_cards().get(card_idx))
    }

    fn hole_card_string(&self, player_idx: usize, card_idx: usize) -> slint::SharedString {
        self.get_hole_card(player_idx, card_idx)
            .map(ToString::to_string)
            .unwrap_or_default()
            .into()
    }

    fn hole_card_red(&self, player_idx: usize, card_idx: usize) -> bool {
        self.get_hole_card(player_idx, card_idx)
            .map(|c| c.is_red())
            .unwrap_or(false)
    }

    fn update_community_cards(&self, ui: &PokerApp) {
        ui.set_flop1(self.community_card_string(0));
        ui.set_flop2(self.community_card_string(1));
        ui.set_flop3(self.community_card_string(2));
        ui.set_turn(self.community_card_string(3));
        ui.set_river(self.community_card_string(4));

        ui.set_flop1_red(self.community_card_red(0));
        ui.set_flop2_red(self.community_card_red(1));
        ui.set_flop3_red(self.community_card_red(2));
        ui.set_turn_red(self.community_card_red(3));
        ui.set_river_red(self.community_card_red(4));
    }

    fn get_community_card(&self, card_idx: usize) -> Option<&Card> {
        self.community_cards.get(card_idx)
    }

    fn community_card_string(&self, card_idx: usize) -> slint::SharedString {
        self.get_community_card(card_idx)
            .map(ToString::to_string)
            .unwrap_or_default()
            .into()
    }

    fn community_card_red(&self, card_idx: usize) -> bool {
        self.get_community_card(card_idx)
            .map(|c| c.is_red())
            .unwrap_or(false)
    }

    fn update_player_status(&self, ui: &PokerApp) {
        if !self.players.is_empty() {
            ui.set_p1_acting(self.current_player == 0 && !self.players[0].is_folded());
        }
        if self.players.len() >= 2 {
            ui.set_p2_acting(self.current_player == 1 && !self.players[1].is_folded());
        }
    }

    fn update_action_controls(&mut self, ui: &PokerApp) {
        let player = self.players.get(self.current_player);
        let can_check =
            player.is_some_and(|p| !p.is_folded() && p.get_current_bet() == self.to_call);
        let can_call = player.is_some_and(|p| {
            !p.is_folded() && p.get_chips() > 0 && p.get_current_bet() < self.to_call
        });
        let can_bet =
            player.is_some_and(|p| !p.is_folded() && p.get_chips() > 0 && self.to_call == 0);
        let can_raise = player.is_some_and(|p| {
            !p.is_folded()
                && p.get_chips() > 0
                && self.to_call > 0
                && p.get_current_bet() < self.to_call + self.min_bet
        });
        let can_fold = player.is_some_and(|p| !p.is_folded());

        ui.set_can_check(can_check);
        ui.set_can_call(can_call);
        ui.set_can_bet(can_bet);
        ui.set_can_raise(can_raise);
        ui.set_can_fold(can_fold);

        let call_amount = player.map_or(0, |p| self.to_call.saturating_sub(p.get_current_bet()));
        ui.set_call_amount(call_amount as f32);
        ui.set_bet_amount(self.bet_amount as f32);
        ui.set_min_bet(self.min_bet as f32);
        ui.set_max_bet(self.max_bet as f32);

        let total_pot = self.pot.saturating_add(call_amount);
        if call_amount > 0 && total_pot > 0 {
            self.pot_odds = call_amount as f32 / total_pot as f32;
        } else {
            self.pot_odds = 0.0;
        }
        ui.set_pot_odds(self.pot_odds);
    }

    pub fn set_bet_amount(&mut self, amount: f32) {
        let amount = amount as u64;
        if amount >= self.min_bet && amount <= self.max_bet {
            self.bet_amount = amount;
        }
    }

    pub fn is_pending_action(&self) -> bool {
        self.pending_action
    }

    pub fn get_current_player(&self) -> usize {
        self.current_player
    }
}

fn main() {
    let app = PokerApp::new().unwrap();

    app.set_player1_name("Alice".into());
    app.set_player2_name("Bob".into());
    app.set_player1_chips(INITIAL_CHIPS as f32);
    app.set_player2_chips(INITIAL_CHIPS as f32);
    app.set_pot_size(0.0);
    app.set_game_stage("Waiting".into());
    app.set_message("Click Start to begin".into());
    app.set_bet_amount(CALL_AMOUNT_DEFAULT as f32);
    app.set_min_bet(MIN_BET_DEFAULT as f32);
    app.set_max_bet(MAX_BET_DEFAULT as f32);
    app.set_pot_odds(0.0);
    app.set_call_amount(0.0);
    app.set_can_check(false);
    app.set_can_call(false);
    app.set_can_bet(false);
    app.set_can_raise(false);
    app.set_can_fold(false);
    app.set_dealer_position(0);
    app.set_p1_card1("".into());
    app.set_p1_card2("".into());
    app.set_p2_card1("".into());
    app.set_p2_card2("".into());
    app.set_flop1("".into());
    app.set_flop2("".into());
    app.set_flop3("".into());
    app.set_turn("".into());
    app.set_river("".into());

    let game_weak = app.as_weak();
    let game = Rc::new(RefCell::new(PokerGame::new(game_weak.clone())));
    game.borrow_mut().game_rc = Some(game.clone());

    let game1 = game.clone();
    app.on_bet_changed(move |value| {
        let mut g = game1.borrow_mut();
        g.set_bet_amount(value);
    });

    let game2 = game.clone();
    app.on_start_game(move || {
        let mut g = game2.borrow_mut();
        g.start_new_hand().ok();
    });

    let game3 = game.clone();
    app.on_fold(move || {
        let mut g = game3.borrow_mut();
        if g.is_pending_action() {
            g.perform_action(PlayerAction::Fold).ok();
        }
    });

    let game4 = game.clone();
    app.on_check(move || {
        let mut g = game4.borrow_mut();
        if g.is_pending_action() {
            g.perform_action(PlayerAction::Check).ok();
        }
    });

    let game5 = game.clone();
    app.on_call(move || {
        let mut g = game5.borrow_mut();
        if g.is_pending_action() {
            g.perform_action(PlayerAction::Call).ok();
        }
    });

    let game6 = game.clone();
    app.on_bet(move || {
        let mut g = game6.borrow_mut();
        if g.is_pending_action() {
            g.perform_action(PlayerAction::Bet).ok();
        }
    });

    let game7 = game.clone();
    app.on_raise(move || {
        let mut g = game7.borrow_mut();
        if g.is_pending_action() {
            g.perform_action(PlayerAction::Raise).ok();
        }
    });

    if let Err(e) = app.run() {
        eprintln!("Failed to start UI: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deck_creation() {
        let deck = Deck::new();
        assert_eq!(deck.len(), 52);
    }

    #[test]
    fn test_deck_shuffle() {
        let mut deck1 = Deck::new();
        let mut deck2 = Deck::new();
        deck1.shuffle();
        deck2.shuffle();

        let cards1: Vec<String> = deck1.cards.iter().map(|c| c.to_string()).collect();
        let cards2: Vec<String> = deck2.cards.iter().map(|c| c.to_string()).collect();

        assert_ne!(cards1, cards2);
    }

    #[test]
    fn test_deck_deal() {
        let mut deck = Deck::new();
        let cards = deck.deal(5).unwrap();
        assert_eq!(cards.len(), 5);
        assert_eq!(deck.len(), 47);
    }

    #[test]
    fn test_deck_deal_insufficient() {
        let mut deck = Deck::new();
        let _ = deck.deal(50).unwrap();
        assert!(deck.deal(5).is_none());
    }

    #[test]
    fn test_card_to_string() {
        let ace_spades = Card::new(14, Suit::Spades);
        assert_eq!(ace_spades.to_string(), "A♠");

        let ten_hearts = Card::new(10, Suit::Hearts);
        assert_eq!(ten_hearts.to_string(), "10♥");

        let two_clubs = Card::new(2, Suit::Clubs);
        assert_eq!(two_clubs.to_string(), "2♣");
    }

    #[test]
    fn test_card_from_string() {
        assert_eq!(Card::from_string("A♠"), Some(Card::new(14, Suit::Spades)));
        assert_eq!(Card::from_string("10♥"), Some(Card::new(10, Suit::Hearts)));
        assert_eq!(Card::from_string("2♣"), Some(Card::new(2, Suit::Clubs)));
        assert_eq!(Card::from_string("invalid"), None);
    }

    #[test]
    fn test_card_is_red() {
        assert!(Card::new(5, Suit::Hearts).is_red());
        assert!(Card::new(5, Suit::Diamonds).is_red());
        assert!(!Card::new(5, Suit::Spades).is_red());
        assert!(!Card::new(5, Suit::Clubs).is_red());
    }

    #[test]
    fn test_player_betting() {
        let mut player = Player::new("Test".to_string(), 1000);
        assert_eq!(player.get_chips(), 1000);

        let result = player.bet(500);
        assert!(result.is_ok());
        assert_eq!(player.get_chips(), 500);
        assert_eq!(player.get_current_bet(), 500);
        assert!(!player.is_all_in());

        let result = player.bet(500);
        assert!(result.is_ok());
        assert_eq!(player.get_chips(), 0);
        assert!(player.is_all_in());

        let result = player.bet(100);
        assert!(result.is_err());
    }

    #[test]
    fn test_hand_evaluator_high_card() {
        let hole_cards = vec![Card::new(2, Suit::Spades), Card::new(5, Suit::Hearts)];
        let community_cards = vec![
            Card::new(8, Suit::Diamonds),
            Card::new(10, Suit::Clubs),
            Card::new(12, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::HighCard);
    }

    #[test]
    fn test_hand_evaluator_pair() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(10, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Diamonds),
            Card::new(8, Suit::Clubs),
            Card::new(12, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::Pair);
    }

    #[test]
    fn test_hand_evaluator_flush() {
        let hole_cards = vec![Card::new(2, Suit::Hearts), Card::new(7, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Hearts),
            Card::new(10, Suit::Hearts),
            Card::new(12, Suit::Hearts),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::Flush);
    }

    #[test]
    fn test_hand_evaluator_straight() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(8, Suit::Hearts)];
        let community_cards = vec![
            Card::new(6, Suit::Diamonds),
            Card::new(7, Suit::Clubs),
            Card::new(9, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::Straight);
    }

    #[test]
    fn test_hand_evaluator_full_house() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(5, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Diamonds),
            Card::new(10, Suit::Clubs),
            Card::new(10, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::FullHouse);
    }

    #[test]
    fn test_hand_evaluator_four_of_a_kind() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(5, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Diamonds),
            Card::new(5, Suit::Clubs),
            Card::new(10, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::FourOfAKind);
    }

    #[test]
    fn test_hand_evaluator_straight_flush() {
        let hole_cards = vec![Card::new(6, Suit::Hearts), Card::new(7, Suit::Hearts)];
        let community_cards = vec![
            Card::new(8, Suit::Hearts),
            Card::new(9, Suit::Hearts),
            Card::new(10, Suit::Hearts),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::StraightFlush);
    }

    #[test]
    fn test_hand_evaluator_royal_flush() {
        let hole_cards = vec![Card::new(10, Suit::Hearts), Card::new(14, Suit::Hearts)];
        let community_cards = vec![
            Card::new(11, Suit::Hearts),
            Card::new(12, Suit::Hearts),
            Card::new(13, Suit::Hearts),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::RoyalFlush);
    }

    #[test]
    fn test_hand_evaluator_two_pair() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(10, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Diamonds),
            Card::new(10, Suit::Clubs),
            Card::new(12, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::TwoPair);
    }

    #[test]
    fn test_hand_evaluator_three_of_a_kind() {
        let hole_cards = vec![Card::new(5, Suit::Spades), Card::new(8, Suit::Hearts)];
        let community_cards = vec![
            Card::new(5, Suit::Diamonds),
            Card::new(5, Suit::Clubs),
            Card::new(12, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::ThreeOfAKind);
    }

    #[test]
    fn test_hand_evaluator_ace_low_straight() {
        let hole_cards = vec![Card::new(2, Suit::Spades), Card::new(3, Suit::Hearts)];
        let community_cards = vec![
            Card::new(4, Suit::Diamonds),
            Card::new(5, Suit::Clubs),
            Card::new(14, Suit::Spades),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::Straight);
        assert_eq!(evaluated.primary_values, vec![5, 4, 3, 2, 1]);
    }
}
