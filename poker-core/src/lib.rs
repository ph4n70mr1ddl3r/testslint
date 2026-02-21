use rand::Rng;
use std::collections::HashMap;

pub const SMALL_BLIND_CHIPS: u64 = 10;
pub const BIG_BLIND_CHIPS: u64 = 20;
pub const INITIAL_CHIPS: u64 = 10000;
pub const MIN_CHIPS_TO_CONTINUE: u64 = 10;
pub const MIN_BET_DEFAULT: u64 = 20;
pub const MAX_BET_DEFAULT: u64 = 500;
pub const MAX_BET_MULTIPLIER: u64 = 100;
pub const CALL_AMOUNT_DEFAULT: u64 = 50;
pub const NUM_PLAYERS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStage {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    HandComplete,
    WaitingToStart,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    AllIn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    #[must_use]
    pub fn to_char(self) -> char {
        match self {
            Self::Spades => '♠',
            Self::Hearts => '♥',
            Self::Diamonds => '♦',
            Self::Clubs => '♣',
        }
    }

    #[must_use]
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '♠' => Some(Self::Spades),
            '♥' => Some(Self::Hearts),
            '♦' => Some(Self::Diamonds),
            '♣' => Some(Self::Clubs),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_red(self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
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
            n => return write!(f, "{n}{}", self.suit.to_char()),
        };
        write!(f, "{rank_str}{}", self.suit.to_char())
    }
}

impl Card {
    #[must_use]
    pub fn new(rank: u8, suit: Suit) -> Self {
        Card { rank, suit }
    }

    #[must_use]
    pub fn is_red(self) -> bool {
        self.suit.is_red()
    }

    #[must_use]
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
    #[must_use]
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

    #[must_use]
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    #[must_use]
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
    folded: bool,
    all_in: bool,
    acted: bool,
}

impl Player {
    #[must_use]
    pub fn new(name: String, chips: u64) -> Self {
        Player {
            name,
            chips,
            hole_cards: Vec::with_capacity(2),
            current_bet: 0,
            folded: false,
            all_in: false,
            acted: false,
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
            self.all_in = true;
        }
        self.acted = true;
        Ok(amount)
    }

    pub fn collect_pot(&mut self, amount: u64) {
        self.chips += amount;
    }

    pub fn reset_for_new_hand(&mut self) {
        self.hole_cards.clear();
        self.current_bet = 0;
        self.folded = false;
        self.all_in = false;
        self.acted = false;
    }

    #[must_use]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn get_chips(&self) -> u64 {
        self.chips
    }

    #[must_use]
    pub fn get_current_bet(&self) -> u64 {
        self.current_bet
    }

    #[must_use]
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    pub fn set_folded(&mut self, folded: bool) {
        self.folded = folded;
    }

    #[must_use]
    pub fn is_all_in(&self) -> bool {
        self.all_in
    }

    #[must_use]
    pub fn has_acted(&self) -> bool {
        self.acted
    }

    pub fn set_has_acted(&mut self, acted: bool) {
        self.acted = acted;
    }

    #[must_use]
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
    #[must_use]
    pub fn new(rank: HandRank, primary_values: Vec<u8>, kickers: Vec<u8>) -> Self {
        EvaluatedHand {
            rank,
            primary_values,
            kickers,
        }
    }
}

pub struct PokerHandEvaluator;

impl PokerHandEvaluator {
    #[must_use]
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

        let rank_counts: HashMap<u8, usize> =
            ranks.iter().fold(HashMap::new(), |mut map, &rank| {
                *map.entry(rank).or_insert(0) += 1;
                map
            });

        if let Some(four_rank) = rank_counts
            .iter()
            .find_map(|(&rank, &count)| (count == 4).then_some(rank))
        {
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

        let suit_counts: HashMap<Suit, usize> =
            all_cards.iter().fold(HashMap::new(), |mut map, &card| {
                *map.entry(card.suit).or_insert(0) += 1;
                map
            });

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

            if let Some(straight_ranks) = Self::find_straight(&flush_cards) {
                if straight_ranks[0] == 14 && straight_ranks[1] == 13 {
                    return EvaluatedHand::new(HandRank::RoyalFlush, vec![14], Vec::new());
                }
                return EvaluatedHand::new(HandRank::StraightFlush, straight_ranks, Vec::new());
            }

            let top_flush_cards: Vec<u8> = flush_cards.into_iter().take(5).collect();
            return EvaluatedHand::new(HandRank::Flush, top_flush_cards, Vec::new());
        }

        if let Some(straight_ranks) = Self::find_straight(&ranks_dedup) {
            return EvaluatedHand::new(HandRank::Straight, straight_ranks, Vec::new());
        }

        if let Some(three_rank) = rank_counts
            .iter()
            .find_map(|(&rank, &count)| (count == 3).then_some(rank))
        {
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
            return EvaluatedHand::new(HandRank::TwoPair, vec![first_pair, second_pair], kicker);
        }

        if let Some(pair_rank) = two_pair_ranks.first() {
            let kickers: Vec<u8> = ranks
                .iter()
                .copied()
                .filter(|&r| r != *pair_rank)
                .take(3)
                .collect();
            return EvaluatedHand::new(HandRank::Pair, vec![*pair_rank], kickers);
        }

        let kickers: Vec<u8> = ranks.into_iter().take(5).collect();
        EvaluatedHand::new(HandRank::HighCard, Vec::new(), kickers)
    }

    fn find_straight(ranks: &[u8]) -> Option<Vec<u8>> {
        if ranks.len() < 5 {
            return None;
        }

        let mut sorted_ranks = ranks.to_vec();
        sorted_ranks.sort_unstable();
        sorted_ranks.dedup();

        for i in 0..=sorted_ranks.len().saturating_sub(5) {
            let window = &sorted_ranks[i..i + 5];
            if window.windows(2).all(|w| w[1] == w[0] + 1) {
                return Some(window.iter().rev().copied().collect());
            }
        }

        if sorted_ranks.contains(&14) {
            let mut ace_low_ranks: Vec<u8> = sorted_ranks
                .iter()
                .copied()
                .map(|r| if r == 14 { 1 } else { r })
                .collect();
            ace_low_ranks.sort_unstable();
            ace_low_ranks.dedup();

            for i in 0..=ace_low_ranks.len().saturating_sub(5) {
                let window = &ace_low_ranks[i..i + 5];
                if window.windows(2).all(|w| w[1] == w[0] + 1) {
                    return Some(vec![5, 4, 3, 2, 1]);
                }
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct PokerGameState {
    pub deck: Deck,
    pub players: Vec<Player>,
    pub community_cards: Vec<Card>,
    pub pot: u64,
    pub stage: GameStage,
    pub dealer_position: usize,
    pub current_player: usize,
    pub to_call: u64,
    pub pending_action: bool,
    pub bet_amount: u64,
    pub min_bet: u64,
    pub max_bet: u64,
    pub pot_odds: f32,
}

impl PokerGameState {
    #[must_use]
    pub fn new() -> Self {
        let mut deck = Deck::new();
        deck.shuffle();

        let mut players = Vec::with_capacity(NUM_PLAYERS);
        players.push(Player::new("Alice".to_string(), INITIAL_CHIPS));
        players.push(Player::new("Bob".to_string(), INITIAL_CHIPS));

        PokerGameState {
            deck,
            players,
            community_cards: Vec::with_capacity(5),
            pot: 0,
            stage: GameStage::WaitingToStart,
            dealer_position: 0,
            current_player: 0,
            to_call: 0,
            pending_action: false,
            bet_amount: CALL_AMOUNT_DEFAULT,
            min_bet: MIN_BET_DEFAULT,
            max_bet: MAX_BET_DEFAULT,
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

        self.post_blinds()?;

        self.stage = GameStage::Preflop;
        self.current_player = (self.dealer_position + 3) % self.players.len();
        self.to_call = BIG_BLIND_CHIPS;
        self.pending_action = true;
        self.update_action_bounds();

        Ok(())
    }

    fn post_blinds(&mut self) -> Result<(), &'static str> {
        let sb_position = (self.dealer_position + 1) % self.players.len();
        let bb_position = (self.dealer_position + 2) % self.players.len();

        self.players[sb_position].bet(SMALL_BLIND_CHIPS)?;
        self.players[bb_position].bet(BIG_BLIND_CHIPS)?;

        self.pot += SMALL_BLIND_CHIPS + BIG_BLIND_CHIPS;

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

        self.max_bet = player_chips.min(MAX_BET_DEFAULT.saturating_mul(MAX_BET_MULTIPLIER));

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
    pub fn perform_action(&mut self, action: PlayerAction) -> Result<String, &'static str> {
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

        let message = match action {
            PlayerAction::Fold => {
                self.players[player_idx].set_folded(true);
                format!("{player_name} folded")
            }

            PlayerAction::Check => {
                if call_amount > 0 {
                    return Err("Cannot check when a bet is pending");
                }
                self.players[player_idx].set_has_acted(true);
                format!("{player_name} checked")
            }

            PlayerAction::Call => {
                let actual_call = call_amount.min(player.get_chips());
                self.players[player_idx].bet(actual_call)?;
                self.pot += actual_call;
                self.players[player_idx].set_has_acted(true);
                format!("{player_name} called {actual_call}")
            }

            PlayerAction::Bet => {
                if call_amount > 0 {
                    return Err("Use Raise action instead of Bet when a bet is pending");
                }
                let bet_amount = self.bet_amount.min(player.get_chips());
                self.players[player_idx].bet(bet_amount)?;
                self.to_call = bet_amount;
                self.pot += bet_amount;
                format!("{player_name} bet {bet_amount}")
            }

            PlayerAction::Raise => {
                let raise_amount = self.bet_amount.min(player.get_chips());
                let total_bet = current_bet + raise_amount;
                if total_bet <= self.to_call {
                    return Err("Raise must be greater than current bet");
                }
                self.players[player_idx].bet(raise_amount)?;
                self.to_call = total_bet;
                self.pot += raise_amount;
                format!("{player_name} raised to {total_bet}")
            }

            PlayerAction::AllIn => {
                let all_in_amount = player.get_chips();
                self.players[player_idx].bet(all_in_amount)?;
                self.pot += all_in_amount;
                if current_bet + all_in_amount > self.to_call {
                    self.to_call = current_bet + all_in_amount;
                }
                format!("{player_name} went all-in with {all_in_amount}")
            }
        };

        self.advance_to_next_player();

        Ok(message)
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

    fn get_active_players(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_folded())
            .map(|(i, _)| i)
            .collect()
    }

    fn get_betting_players(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_folded() && !p.is_all_in())
            .map(|(i, _)| i)
            .collect()
    }

    fn check_street_complete(&mut self) {
        let active_players = self.get_active_players();

        if active_players.len() == 1 {
            let winner_idx = active_players[0];
            self.players[winner_idx].collect_pot(self.pot);
            self.end_hand();
            return;
        }

        let betting_players = self.get_betting_players();

        if betting_players.is_empty() {
            self.run_out_board();
            self.determine_winner();
            return;
        }

        let all_acted = betting_players.iter().all(|&i| self.players[i].has_acted());

        let bets_equal = betting_players
            .iter()
            .all(|&i| self.players[i].get_current_bet() == self.to_call);

        if all_acted && bets_equal {
            self.advance_street();
        }
    }

    fn run_out_board(&mut self) {
        while self.community_cards.len() < 5 {
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
                _ => break,
            }
        }
        self.stage = GameStage::Showdown;
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
                self.end_hand();
            }
        }

        self.current_player = (self.dealer_position + 1) % self.players.len();
        self.pending_action = true;
        self.update_action_bounds();
    }

    fn deal_community_cards(&mut self, count: usize) {
        self.deck.burn();
        if let Some(cards) = self.deck.deal(count) {
            self.community_cards.extend(cards);
        }
    }

    fn determine_winner(&mut self) {
        let active_players = self.get_active_players();

        if active_players.len() == 1 {
            let winner_idx = active_players[0];
            let winnings = self.pot;
            self.players[winner_idx].collect_pot(winnings);
            self.end_hand();
            return;
        }

        let mut best_hand: Option<EvaluatedHand> = None;
        let mut winners: Vec<usize> = Vec::new();

        for &player_idx in &active_players {
            let hand = PokerHandEvaluator::evaluate(
                self.players[player_idx].get_hole_cards(),
                &self.community_cards,
            );

            match best_hand.as_ref() {
                Some(best) => match hand.cmp(best) {
                    std::cmp::Ordering::Greater => {
                        best_hand = Some(hand);
                        winners.clear();
                        winners.push(player_idx);
                    }
                    std::cmp::Ordering::Equal => {
                        winners.push(player_idx);
                    }
                    std::cmp::Ordering::Less => {}
                },
                None => {
                    best_hand = Some(hand);
                    winners.push(player_idx);
                }
            }
        }

        if winners.is_empty() {
            self.end_hand();
            return;
        }

        let split_amount = self.pot / winners.len() as u64;
        let remainder = self.pot % winners.len() as u64;

        for &winner_idx in &winners {
            self.players[winner_idx].collect_pot(split_amount);
        }

        if remainder > 0 {
            self.players[winners[0]].collect_pot(remainder);
        }

        self.end_hand();
    }

    fn end_hand(&mut self) {
        self.stage = GameStage::HandComplete;
        self.dealer_position = (self.dealer_position + 1) % self.players.len();
    }

    #[must_use]
    pub fn get_stage_string(&self) -> &'static str {
        match self.stage {
            GameStage::Preflop => "Preflop",
            GameStage::Flop => "Flop",
            GameStage::Turn => "Turn",
            GameStage::River => "River",
            GameStage::Showdown => "Showdown",
            GameStage::HandComplete => "Complete",
            GameStage::WaitingToStart => "Waiting",
        }
    }

    pub fn set_bet_amount(&mut self, amount: f32) {
        let amount = amount as u64;
        if amount >= self.min_bet && amount <= self.max_bet {
            self.bet_amount = amount;
        }
    }

    #[must_use]
    pub fn is_pending_action(&self) -> bool {
        self.pending_action
    }

    #[must_use]
    pub fn get_current_player(&self) -> usize {
        self.current_player
    }

    #[must_use]
    pub fn can_check(&self) -> bool {
        self.players
            .get(self.current_player)
            .is_some_and(|p| !p.is_folded() && p.get_current_bet() == self.to_call)
    }

    #[must_use]
    pub fn can_call(&self) -> bool {
        self.players.get(self.current_player).is_some_and(|p| {
            !p.is_folded() && p.get_chips() > 0 && p.get_current_bet() < self.to_call
        })
    }

    #[must_use]
    pub fn can_bet(&self) -> bool {
        self.players
            .get(self.current_player)
            .is_some_and(|p| !p.is_folded() && p.get_chips() > 0 && self.to_call == 0)
    }

    #[must_use]
    pub fn can_raise(&self) -> bool {
        self.players.get(self.current_player).is_some_and(|p| {
            !p.is_folded()
                && p.get_chips() > 0
                && self.to_call > 0
                && p.get_current_bet() < self.to_call + self.min_bet
        })
    }

    #[must_use]
    pub fn can_fold(&self) -> bool {
        self.players
            .get(self.current_player)
            .is_some_and(|p| !p.is_folded())
    }

    #[must_use]
    pub fn get_call_amount(&self) -> u64 {
        self.players
            .get(self.current_player)
            .map_or(0, |p| self.to_call.saturating_sub(p.get_current_bet()))
    }

    pub fn update_pot_odds(&mut self) {
        let call_amount = self.get_call_amount();
        let total_pot = self.pot.saturating_add(call_amount);
        self.pot_odds = if call_amount > 0 && total_pot > 0 {
            call_amount as f32 / total_pot as f32
        } else {
            0.0
        };
    }
}

impl Default for PokerGameState {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_hand_evaluator_straight_with_higher_non_straight_cards() {
        let hole_cards = vec![Card::new(14, Suit::Spades), Card::new(13, Suit::Hearts)];
        let community_cards = vec![
            Card::new(11, Suit::Diamonds),
            Card::new(10, Suit::Clubs),
            Card::new(9, Suit::Spades),
            Card::new(8, Suit::Hearts),
            Card::new(7, Suit::Diamonds),
        ];

        let evaluated = PokerHandEvaluator::evaluate(&hole_cards, &community_cards);
        assert_eq!(evaluated.rank, HandRank::Straight);
        assert_eq!(evaluated.primary_values, vec![11, 10, 9, 8, 7]);
    }
}
