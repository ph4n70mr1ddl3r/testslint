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

    fn deal_all_cards(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();

        self.players[0].hole_cards = self.deck.deal(2);
        self.players[1].hole_cards = self.deck.deal(2);
        self.community_cards = self.deck.deal(5);
        self.pot = 450.0;
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

    fn update_action_controls(&self, ui: &PokerApp) {
        ui.set_can_check(true);
        ui.set_can_call(true);
        ui.set_can_bet(true);
        ui.set_can_raise(true);
        ui.set_can_fold(true);
        ui.set_call_amount(50.0);
        ui.set_bet_amount(self.bet_amount as f32);
        ui.set_min_bet(20.0);
        ui.set_max_bet(500.0);
    }
}

fn main() {
    let app = PokerApp::new().unwrap();

    app.set_player1_name("Alice".into());
    app.set_player2_name("Bob".into());
    app.set_player1_chips(INITIAL_CHIPS as f32);
    app.set_player2_chips(INITIAL_CHIPS as f32);
    app.set_pot_size(0.0);
    app.set_game_stage("Showdown".into());
    app.set_message("Layout Preview - All cards revealed".into());
    app.set_bet_amount(100.0);
    app.set_min_bet(20.0);
    app.set_max_bet(500.0);
    app.set_call_amount(50.0);
    app.set_can_check(true);
    app.set_can_call(true);
    app.set_can_bet(true);
    app.set_can_raise(true);
    app.set_can_fold(true);
    app.set_dealer_position(0);

    let game_weak = app.as_weak();
    let game = Rc::new(RefCell::new(PokerGame::new(game_weak)));

    game.borrow_mut().deal_all_cards();
    game.borrow_mut()
        .update_ui("Layout Preview - All cards revealed".to_string());

    let game_clone = game.clone();
    app.on_bet_changed(move |value| {
        let mut g = game_clone.borrow_mut();
        g.bet_amount = value as f64;
        g.update_ui(format!("Bet: {}", value as f64));
    });

    app.run().unwrap();
}
