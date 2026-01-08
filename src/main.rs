use rand::Rng;
use slint::{Timer, TimerMode, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

include!(env!("SLINT_INCLUDE_GENERATED"));

const SMALL_BLIND: f64 = 10.0;
const BIG_BLIND: f64 = 20.0;
const INITIAL_CHIPS: f64 = 10000.0;
const MIN_CHIPS_TO_CONTINUE: f64 = 10.0;
const START_DELAY_MS: u64 = 1500;
const ACTION_DELAY_MIN_MS: u64 = 1000;
const ACTION_DELAY_VAR_MS: u64 = 1500;
const STREET_DELAY_MS: u64 = 2000;
const SHOWDOWN_DELAY_MS: u64 = 3000;
const HAND_COMPLETE_DELAY_MS: u64 = 3000;

#[derive(Clone, Copy, PartialEq, Debug)]
enum GameStage {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    HandComplete,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PlayerAction {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    AllIn,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HandRank {
    HighCard,
    Pair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
}

#[derive(Clone)]
struct Card {
    suit: char,
    rank: u8,
}

impl Card {
    fn new(suit: char, rank: u8) -> Self {
        Card { suit, rank }
    }

    fn to_string(&self) -> String {
        let rank_str = match self.rank {
            14 => "A".to_string(),
            13 => "K".to_string(),
            12 => "Q".to_string(),
            11 => "J".to_string(),
            10 => "10".to_string(),
            _ => self.rank.to_string(),
        };
        format!("{}{}", rank_str, self.suit)
    }

    fn is_red(&self) -> bool {
        self.suit == '♥' || self.suit == '♦'
    }
}

struct Deck {
    cards: Vec<Card>,
}

impl Deck {
    fn new() -> Self {
        let suits = ['♠', '♥', '♦', '♣'];
        let mut cards = Vec::with_capacity(52);
        for suit in suits {
            for rank in 2..=14 {
                cards.push(Card::new(suit, rank));
            }
        }
        Deck { cards }
    }

    fn shuffle(&mut self) {
        let mut rng = rand::thread_rng();
        for i in (1..self.cards.len()).rev() {
            let j = rng.gen_range(0..=i);
            self.cards.swap(i, j);
        }
    }

    fn deal(&mut self, count: usize) -> Vec<Card> {
        self.cards.drain(0..count).collect()
    }

    fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Clone)]
struct Player {
    name: String,
    chips: f64,
    hole_cards: Vec<Card>,
    current_bet: f64,
    is_folded: bool,
    is_all_in: bool,
}

impl Player {
    fn new(name: String, chips: f64) -> Self {
        Player {
            name,
            chips,
            hole_cards: Vec::with_capacity(2),
            current_bet: 0.0,
            is_folded: false,
            is_all_in: false,
        }
    }

    fn receive_cards(&mut self, cards: Vec<Card>) {
        self.hole_cards.extend(cards);
    }

    fn bet(&mut self, amount: f64) -> f64 {
        let actual_bet = amount.min(self.chips);
        self.chips -= actual_bet;
        self.current_bet += actual_bet;
        if self.chips < 0.01 {
            self.is_all_in = true;
        }
        actual_bet
    }

    fn collect_pot(&mut self, amount: f64) {
        self.chips += amount;
    }

    fn reset_for_new_hand(&mut self) {
        self.hole_cards.clear();
        self.current_bet = 0.0;
        self.is_folded = false;
        self.is_all_in = false;
    }
}

#[derive(Clone)]
struct EvaluatedHand {
    rank: HandRank,
    primary_values: Vec<u8>,
    kickers: Vec<u8>,
}

struct PokerGame {
    deck: Deck,
    players: Vec<Player>,
    community_cards: Vec<Card>,
    pot: f64,
    stage: GameStage,
    dealer_position: usize,
    current_player: usize,
    to_call: f64,
    game_weak: Weak<PokerApp>,
    next_action_time: Option<Instant>,
    pending_action: bool,
    timer: Option<Timer>,
    game_rc: Option<Rc<RefCell<PokerGame>>>,
    all_in_this_street: bool,
    bet_amount: f64,
}

impl PokerGame {
    fn new(game_weak: Weak<PokerApp>) -> Self {
        let deck = Deck::new();

        let mut players = Vec::with_capacity(2);
        players.push(Player::new("Alice".to_string(), INITIAL_CHIPS));
        players.push(Player::new("Bob".to_string(), INITIAL_CHIPS));

        PokerGame {
            deck,
            players,
            community_cards: Vec::with_capacity(5),
            pot: 0.0,
            stage: GameStage::HandComplete,
            dealer_position: 0,
            current_player: 0,
            to_call: 0.0,
            game_weak,
            next_action_time: None,
            pending_action: false,
            timer: None,
            game_rc: None,
            all_in_this_street: false,
            bet_amount: 0.0,
        }
    }

    fn start_new_hand(&mut self) {
        for player in &mut self.players {
            player.reset_for_new_hand();
        }

        if self.deck.len() < 10 {
            self.deck = Deck::new();
            self.deck.shuffle();
        } else {
            self.deck.shuffle();
        }

        self.community_cards.clear();
        self.pot = 0.0;
        self.stage = GameStage::Preflop;
        self.all_in_this_street = false;

        let sb_player = self.dealer_position;
        let bb_player = (self.dealer_position + 1) % 2;

        self.players[sb_player].bet(SMALL_BLIND);
        self.players[bb_player].bet(BIG_BLIND);
        self.pot += SMALL_BLIND + BIG_BLIND;

        self.to_call = BIG_BLIND - SMALL_BLIND;
        self.current_player = bb_player;

        for _ in 0..2 {
            for player in &mut self.players {
                let cards = self.deck.deal(1);
                player.receive_cards(cards);
            }
        }

        self.update_ui("Dealing hole cards...".to_string());

        self.schedule_action(START_DELAY_MS);
    }

    fn get_random_action(&self) -> PlayerAction {
        let mut rng = rand::thread_rng();
        let player = &self.players[self.current_player];

        let can_check = player.current_bet >= self.to_call || self.to_call == 0.0;

        if player.is_all_in || player.is_folded {
            return PlayerAction::Check;
        }

        let actions: Vec<PlayerAction> = if can_check {
            vec![
                PlayerAction::Check,
                PlayerAction::Bet,
                PlayerAction::Raise,
                PlayerAction::Fold,
            ]
        } else {
            vec![
                PlayerAction::Call,
                PlayerAction::Raise,
                PlayerAction::Fold,
                PlayerAction::AllIn,
            ]
        };

        actions[rng.gen_range(0..actions.len())]
    }

    fn execute_action(&mut self, action: PlayerAction) -> bool {
        let player_name = self.players[self.current_player].name.clone();
        let player = &mut self.players[self.current_player];
        let mut action_taken = false;

        match action {
            PlayerAction::Fold => {
                player.is_folded = true;
                self.update_ui(format!("{} folds", player_name));
                action_taken = true;
            }
            PlayerAction::Check => {
                if player.current_bet >= self.to_call {
                    self.update_ui(format!("{} checks", player_name));
                    action_taken = true;
                }
            }
            PlayerAction::Call => {
                let call_amount = self.to_call - player.current_bet;
                if player.chips >= call_amount {
                    player.bet(call_amount);
                    self.pot += call_amount;
                    self.update_ui(format!("{} calls {}", player_name, call_amount));
                    action_taken = true;
                }
            }
            PlayerAction::Bet => {
                let mut rng = rand::thread_rng();
                let bet_amount =
                    (self.to_call * (1.0 + rng.gen_range(0.5..2.0))).max(self.to_call * 2.0);
                let actual_bet = player.bet(bet_amount);
                self.pot += actual_bet;
                self.to_call = player.current_bet;
                self.update_ui(format!("{} bets {}", player_name, actual_bet));
                action_taken = true;
            }
            PlayerAction::Raise => {
                let mut rng = rand::thread_rng();
                let raise_amount =
                    (self.to_call * (1.0 + rng.gen_range(1.0..3.0))).max(self.to_call * 2.0);
                let actual_bet = player.bet(raise_amount - player.current_bet + self.to_call);
                self.pot += actual_bet;
                self.to_call = player.current_bet;
                self.update_ui(format!("{} raises to {}", player_name, actual_bet));
                action_taken = true;
            }
            PlayerAction::AllIn => {
                let all_in_amount = player.chips;
                let actual_bet = player.bet(all_in_amount);
                self.pot += actual_bet;
                self.to_call = player.current_bet;
                self.update_ui(format!("{} goes all-in {}", player_name, actual_bet));
                action_taken = true;
                self.all_in_this_street = true;
            }
        }

        action_taken
    }

    fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % 2;
    }

    fn check_betting_complete(&self) -> bool {
        let p1_bet = self.players[0].current_bet;
        let p2_bet = self.players[1].current_bet;

        (p1_bet == p2_bet || self.players[0].is_all_in || self.players[1].is_all_in)
            && (self.players[0].current_bet >= self.to_call || self.players[0].is_folded)
            && (self.players[1].current_bet >= self.to_call || self.players[1].is_folded)
    }

    fn evaluate_hand(&self, hole_cards: &[Card], community_cards: &[Card]) -> EvaluatedHand {
        let mut all_cards: Vec<Card> = hole_cards
            .iter()
            .chain(community_cards.iter())
            .cloned()
            .collect();
        all_cards.sort_by_key(|c| c.rank);

        let suits: Vec<char> = all_cards.iter().map(|c| c.suit).collect();
        let ranks: Vec<u8> = all_cards.iter().map(|c| c.rank).collect();

        let is_flush = suits
            .windows(5)
            .any(|window| window.iter().all(|&s| s == window[0]));

        let mut unique_ranks: Vec<u8> = ranks.iter().copied().collect();
        unique_ranks.dedup();

        let is_straight = if unique_ranks.len() >= 5 {
            let mut straight_count = 1;
            for i in 0..unique_ranks.len() - 1 {
                if unique_ranks[i + 1] == unique_ranks[i] + 1 {
                    straight_count += 1;
                } else {
                    straight_count = 1;
                }
                if straight_count >= 5 {
                    return EvaluatedHand {
                        rank: if is_flush {
                            HandRank::StraightFlush
                        } else {
                            HandRank::Straight
                        },
                        primary_values: vec![unique_ranks[i + 1]],
                        kickers: Vec::new(),
                    };
                }
            }
            if unique_ranks == [2, 3, 4, 5, 14] {
                return EvaluatedHand {
                    rank: if is_flush {
                        HandRank::StraightFlush
                    } else {
                        HandRank::Straight
                    },
                    primary_values: vec![5],
                    kickers: Vec::new(),
                };
            }
            false
        } else {
            false
        };

        if is_flush && !is_straight {
            let flush_suit = suits
                .iter()
                .find(|&&s| suits.iter().filter(|&&other| other == s).count() >= 5)
                .copied()
                .unwrap();

            let mut flush_cards: Vec<u8> = all_cards
                .iter()
                .filter(|c| c.suit == flush_suit)
                .map(|c| c.rank)
                .collect();
            flush_cards.sort();
            flush_cards.reverse();

            return EvaluatedHand {
                rank: HandRank::Flush,
                primary_values: flush_cards.into_iter().take(5).collect(),
                kickers: Vec::new(),
            };
        }

        let rank_counts: Vec<(u8, usize)> = {
            let mut counts = Vec::new();
            let mut seen = Vec::new();
            for &rank in &ranks {
                if !seen.contains(&rank) {
                    let count = ranks.iter().filter(|&&r| r == rank).count();
                    counts.push((rank, count));
                    seen.push(rank);
                }
            }
            counts.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
            counts
        };

        if rank_counts.len() >= 4 {
            let four_kind = rank_counts.iter().find(|&&(_, count)| count == 4);
            let trips = rank_counts.iter().find(|&&(_, count)| count == 3);
            let pairs: Vec<(u8, usize)> = rank_counts
                .iter()
                .filter(|&&(_, count)| count == 2)
                .cloned()
                .collect();

            if let Some((fk_rank, _)) = four_kind {
                let kicker = rank_counts
                    .iter()
                    .find(|&&(r, _)| r != *fk_rank)
                    .map(|(r, _)| *r)
                    .unwrap_or(0);
                return EvaluatedHand {
                    rank: HandRank::FourOfAKind,
                    primary_values: vec![*fk_rank, kicker],
                    kickers: Vec::new(),
                };
            }

            if let Some((tp_rank, _)) = trips {
                if !pairs.is_empty() {
                    let (pair_rank, _) = pairs[0];
                    return EvaluatedHand {
                        rank: HandRank::FullHouse,
                        primary_values: vec![*tp_rank, pair_rank],
                        kickers: Vec::new(),
                    };
                }
            }

            if let Some((tp_rank, _)) = trips {
                let mut kickers: Vec<u8> = rank_counts
                    .iter()
                    .filter(|&&(r, _)| r != *tp_rank)
                    .map(|(r, _)| *r)
                    .collect();
                kickers.sort();
                kickers.reverse();
                kickers.truncate(2);

                return EvaluatedHand {
                    rank: HandRank::ThreeOfAKind,
                    primary_values: vec![*tp_rank],
                    kickers,
                };
            }

            if pairs.len() >= 2 {
                let top_pair = pairs[0].0;
                let second_pair = pairs[1].0;
                let kicker = rank_counts
                    .iter()
                    .find(|&&(r, _)| r != top_pair && r != second_pair)
                    .map(|(r, _)| *r)
                    .unwrap_or(0);
                return EvaluatedHand {
                    rank: HandRank::TwoPair,
                    primary_values: vec![top_pair, second_pair, kicker],
                    kickers: Vec::new(),
                };
            }

            if let Some((pair_rank, _)) = pairs.first() {
                let mut kickers: Vec<u8> = rank_counts
                    .iter()
                    .filter(|&&(r, _)| r != *pair_rank)
                    .map(|(r, _)| *r)
                    .collect();
                kickers.sort();
                kickers.reverse();
                kickers.truncate(3);

                return EvaluatedHand {
                    rank: HandRank::Pair,
                    primary_values: vec![*pair_rank],
                    kickers,
                };
            }
        }

        let mut kickers: Vec<u8> = ranks.iter().copied().collect();
        kickers.sort();
        kickers.reverse();
        kickers.truncate(5);

        EvaluatedHand {
            rank: HandRank::HighCard,
            primary_values: kickers.clone(),
            kickers: Vec::new(),
        }
    }

    fn compare_hands(&self, hand1: &EvaluatedHand, hand2: &EvaluatedHand) -> i8 {
        fn rank_value(rank: HandRank) -> u8 {
            match rank {
                HandRank::HighCard => 0,
                HandRank::Pair => 1,
                HandRank::TwoPair => 2,
                HandRank::ThreeOfAKind => 3,
                HandRank::Straight => 4,
                HandRank::Flush => 5,
                HandRank::FullHouse => 6,
                HandRank::FourOfAKind => 7,
                HandRank::StraightFlush => 8,
            }
        }

        let rank_cmp = rank_value(hand1.rank).cmp(&rank_value(hand2.rank));
        if rank_cmp != std::cmp::Ordering::Equal {
            return rank_cmp as i8;
        }

        for (v1, v2) in hand1.primary_values.iter().zip(hand2.primary_values.iter()) {
            if v1 != v2 {
                return if v1 > v2 { 1 } else { -1 };
            }
        }

        for (k1, k2) in hand1.kickers.iter().zip(hand2.kickers.iter()) {
            if k1 != k2 {
                return if k1 > k2 { 1 } else { -1 };
            }
        }

        0
    }

    fn end_hand(&mut self) {
        let active_players: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.is_folded)
            .map(|(i, _)| i)
            .collect();

        if active_players.len() == 1 {
            let winner_idx = active_players[0];
            let winner_name = self.players[winner_idx].name.clone();
            self.players[winner_idx].collect_pot(self.pot);
            self.pot = 0.0;
            self.update_ui(format!("{} wins - opponent folded!", winner_name));
        } else {
            let hand1 = self.evaluate_hand(&self.players[0].hole_cards, &self.community_cards);
            let hand2 = self.evaluate_hand(&self.players[1].hole_cards, &self.community_cards);

            let cmp = self.compare_hands(&hand1, &hand2);
            let winner_idx = if cmp > 0 { 0 } else { 1 };
            let winner_name = self.players[winner_idx].name.clone();

            let hand_name = match self.players[winner_idx].hole_cards.len() {
                2 => {
                    let hand = if winner_idx == 0 { &hand1 } else { &hand2 };
                    match hand.rank {
                        HandRank::HighCard => "high card",
                        HandRank::Pair => "a pair",
                        HandRank::TwoPair => "two pair",
                        HandRank::ThreeOfAKind => "three of a kind",
                        HandRank::Straight => "a straight",
                        HandRank::Flush => "a flush",
                        HandRank::FullHouse => "a full house",
                        HandRank::FourOfAKind => "four of a kind",
                        HandRank::StraightFlush => "a straight flush!",
                    }
                }
                _ => "",
            };

            self.players[winner_idx].collect_pot(self.pot);
            self.pot = 0.0;

            if hand_name.is_empty() {
                self.update_ui(format!("{} wins the pot!", winner_name));
            } else {
                self.update_ui(format!("{} wins with {}!", winner_name, hand_name));
            }
        }

        if self.players[0].chips < MIN_CHIPS_TO_CONTINUE
            || self.players[1].chips < MIN_CHIPS_TO_CONTINUE
        {
            self.update_ui("Game Over! Resetting chips...".to_string());
            self.players[0].chips = INITIAL_CHIPS;
            self.players[1].chips = INITIAL_CHIPS;
        }

        self.stage = GameStage::HandComplete;
        self.schedule_action(HAND_COMPLETE_DELAY_MS);
    }

    fn handle_player_action(&mut self) {
        if self.players.iter().all(|p| p.is_folded) {
            self.end_hand();
            return;
        }

        if self.all_in_this_street && self.check_betting_complete() {
            self.proceed_to_next_street();
            return;
        }

        let action = self.get_random_action();
        if self.execute_action(action) {
            if self.check_betting_complete() {
                self.proceed_to_next_street();
            } else {
                self.next_player();
                self.schedule_action(
                    ACTION_DELAY_MIN_MS + rand::thread_rng().gen_range(0..ACTION_DELAY_VAR_MS),
                );
            }
        }
    }

    fn proceed_to_next_street(&mut self) {
        for player in &mut self.players {
            player.current_bet = 0.0;
        }
        self.to_call = 0.0;
        self.all_in_this_street = false;

        match self.stage {
            GameStage::Preflop => {
                self.stage = GameStage::Flop;
                let cards = self.deck.deal(3);
                self.community_cards.extend(cards);
                self.update_ui("Flop dealt".to_string());
            }
            GameStage::Flop => {
                self.stage = GameStage::Turn;
                let cards = self.deck.deal(1);
                self.community_cards.extend(cards);
                self.update_ui("Turn card".to_string());
            }
            GameStage::Turn => {
                self.stage = GameStage::River;
                let cards = self.deck.deal(1);
                self.community_cards.extend(cards);
                self.update_ui("River card".to_string());
            }
            GameStage::River => {
                self.stage = GameStage::Showdown;
                self.update_ui("Showdown!".to_string());
                self.schedule_action(SHOWDOWN_DELAY_MS);
                return;
            }
            GameStage::Showdown => {
                self.end_hand();
                return;
            }
            GameStage::HandComplete => {
                self.dealer_position = (self.dealer_position + 1) % 2;
                self.start_new_hand();
                return;
            }
        }

        self.current_player = (self.dealer_position + 1) % 2;
        self.schedule_action(STREET_DELAY_MS);
    }

    fn schedule_action(&mut self, delay_ms: u64) {
        self.next_action_time = Some(Instant::now() + Duration::from_millis(delay_ms));
        self.pending_action = true;
    }

    fn start_timer(&mut self) {
        let game_rc = self.game_rc.clone().unwrap();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
            let mut game = game_rc.borrow_mut();
            if game.pending_action {
                if let Some(next_time) = game.next_action_time {
                    if Instant::now() >= next_time {
                        game.pending_action = false;
                        game.next_action();
                    }
                }
            }
        });
        self.timer = Some(timer);
    }

    fn next_action(&mut self) {
        match self.stage {
            GameStage::Preflop | GameStage::Flop | GameStage::Turn | GameStage::River => {
                self.handle_player_action();
            }
            GameStage::Showdown => {
                self.end_hand();
            }
            GameStage::HandComplete => {
                self.dealer_position = (self.dealer_position + 1) % 2;
                self.start_new_hand();
            }
        }
    }

    fn update_ui(&self, message: String) {
        if let Some(ui) = self.game_weak.upgrade() {
            ui.set_player1_name(self.players[0].name.clone().into());
            ui.set_player2_name(self.players[1].name.clone().into());
            ui.set_player1_chips(self.players[0].chips as f32);
            ui.set_player2_chips(self.players[1].chips as f32);
            ui.set_pot_size(self.pot as f32);
            ui.set_game_stage(self.get_stage_string().into());
            ui.set_dealer_position(self.dealer_position as i32);
            ui.set_message(message.into());

            self.update_player_cards(&ui);
            self.update_community_cards(&ui);
            self.update_player_status(&ui);
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
        }
        .to_string()
    }

    fn update_player_cards(&self, ui: &PokerApp) {
        ui.set_p1_card1(self.hole_card_string(0, 0));
        ui.set_p1_card2(self.hole_card_string(0, 1));
        ui.set_p2_card1(self.hole_card_string(1, 0));
        ui.set_p2_card2(self.hole_card_string(1, 1));

        ui.set_p1_card1_red(self.hole_card_red(0, 0));
        ui.set_p1_card2_red(self.hole_card_red(0, 1));
        ui.set_p2_card1_red(self.hole_card_red(1, 0));
        ui.set_p2_card2_red(self.hole_card_red(1, 1));
    }

    fn hole_card_string(&self, player_idx: usize, card_idx: usize) -> slint::SharedString {
        if self.players[player_idx].hole_cards.len() > card_idx {
            self.players[player_idx].hole_cards[card_idx]
                .to_string()
                .into()
        } else {
            "".into()
        }
    }

    fn hole_card_red(&self, player_idx: usize, card_idx: usize) -> bool {
        self.players[player_idx]
            .hole_cards
            .get(card_idx)
            .map(|c| c.is_red())
            .unwrap_or(false)
    }

    fn update_community_cards(&self, ui: &PokerApp) {
        let community_cards: Vec<String> =
            self.community_cards.iter().map(|c| c.to_string()).collect();
        let community_cards_red: Vec<bool> =
            self.community_cards.iter().map(|c| c.is_red()).collect();

        ui.set_flop1(
            community_cards
                .get(0)
                .map(|s| s.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_flop2(
            community_cards
                .get(1)
                .map(|s| s.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_flop3(
            community_cards
                .get(2)
                .map(|s| s.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_turn(
            community_cards
                .get(3)
                .map(|s| s.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_river(
            community_cards
                .get(4)
                .map(|s| s.as_str())
                .unwrap_or("")
                .into(),
        );

        ui.set_flop1_red(community_cards_red.get(0).copied().unwrap_or(false));
        ui.set_flop2_red(community_cards_red.get(1).copied().unwrap_or(false));
        ui.set_flop3_red(community_cards_red.get(2).copied().unwrap_or(false));
        ui.set_turn_red(community_cards_red.get(3).copied().unwrap_or(false));
        ui.set_river_red(community_cards_red.get(4).copied().unwrap_or(false));
    }

    fn update_player_status(&self, ui: &PokerApp) {
        ui.set_p1_acting(self.current_player == 0 && !self.players[0].is_folded);
        ui.set_p2_acting(self.current_player == 1 && !self.players[1].is_folded);
        ui.set_p1_folded(self.players[0].is_folded);
        ui.set_p2_folded(self.players[1].is_folded);
        ui.set_p1_current_bet(self.players[0].current_bet as f32);
        ui.set_p2_current_bet(self.players[1].current_bet as f32);
    }
}

fn main() {
    let app = PokerApp::new().unwrap();

    app.set_player1_name("Alice".into());
    app.set_player2_name("Bob".into());
    app.set_player1_chips(INITIAL_CHIPS as f32);
    app.set_player2_chips(INITIAL_CHIPS as f32);
    app.set_pot_size(0.0);
    app.set_game_stage("Starting".into());
    app.set_message("Initializing game...".into());
    app.set_bet_amount(0.0);
    app.set_min_bet(0.0);
    app.set_max_bet(100.0);
    app.set_call_amount(0.0);
    app.set_can_check(false);
    app.set_can_call(false);
    app.set_can_bet(false);
    app.set_can_raise(false);
    app.set_can_fold(false);

    let game_weak = app.as_weak();
    let game = Rc::new(RefCell::new(PokerGame::new(game_weak)));

    game.borrow_mut().start_new_hand();

    game.borrow_mut().game_rc = Some(game.clone());
    game.borrow_mut().start_timer();

    let game_clone = game.clone();
    app.on_bet_changed(move |value| {
        println!("Bet changed to: {}", value);
        let mut g = game_clone.borrow_mut();
        g.bet_amount = value as f64;
    });

    app.run().unwrap();
}
