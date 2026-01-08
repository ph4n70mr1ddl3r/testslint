use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use rand::Rng;
use slint::{Timer, TimerMode, Weak};

include!(env!("SLINT_INCLUDE_GENERATED"));

#[derive(Clone, Copy, PartialEq)]
enum GameStage {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    HandComplete,
}

#[derive(Clone, Copy, PartialEq)]
enum PlayerAction {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    AllIn,
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
        let mut cards = Vec::new();
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
            hole_cards: Vec::new(),
            current_bet: 0.0,
            is_folded: false,
            is_all_in: false,
        }
    }
    
    fn receive_cards(&mut self, cards: Vec<Card>) {
        self.hole_cards = cards;
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
}

impl PokerGame {
    fn new(game_weak: Weak<PokerApp>) -> Self {
        let mut deck = Deck::new();
        deck.shuffle();
        
        let mut players = Vec::new();
        players.push(Player::new("Alice".to_string(), 10000.0));
        players.push(Player::new("Bob".to_string(), 10000.0));
        
        PokerGame {
            deck,
            players,
            community_cards: Vec::new(),
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
        }
    }
    
    fn start_new_hand(&mut self) {
        for player in &mut self.players {
            player.reset_for_new_hand();
        }
        
        self.deck = Deck::new();
        self.deck.shuffle();
        
        self.community_cards.clear();
        self.pot = 0.0;
        self.stage = GameStage::Preflop;
        
        let small_blind = 10.0;
        let big_blind = 20.0;
        
        let sb_player = self.dealer_position;
        let bb_player = (self.dealer_position + 1) % 2;
        
        self.players[sb_player].bet(small_blind);
        self.players[bb_player].bet(big_blind);
        self.pot += small_blind + big_blind;
        
        self.to_call = big_blind - small_blind;
        self.current_player = bb_player;
        
        for _ in 0..2 {
            for player in &mut self.players {
                let cards = self.deck.deal(1);
                player.receive_cards(cards);
            }
        }
        
        self.update_ui("Dealing hole cards...".to_string());
        
        self.schedule_action(1500);
    }
    
    fn get_random_action(&self) -> PlayerAction {
        let mut rng = rand::thread_rng();
        let player = &self.players[self.current_player];
        
        let can_check = player.current_bet >= self.to_call || self.to_call == 0.0;
        
        if player.is_all_in || player.is_folded {
            return PlayerAction::Check;
        }
        
        let actions: Vec<PlayerAction> = if can_check {
                vec![PlayerAction::Check, PlayerAction::Bet, PlayerAction::Raise, PlayerAction::Fold]
            } else {
                vec![PlayerAction::Call, PlayerAction::Raise, PlayerAction::Fold, PlayerAction::AllIn]
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
                let bet_amount = (self.to_call * (1.0 + rng.gen_range(0.5..2.0))).max(self.to_call * 2.0);
                let actual_bet = player.bet(bet_amount);
                self.pot += actual_bet;
                self.to_call = player.current_bet;
                self.update_ui(format!("{} bets {}", player_name, actual_bet));
                action_taken = true;
            }
            PlayerAction::Raise => {
                let mut rng = rand::thread_rng();
                let raise_amount = (self.to_call * (1.0 + rng.gen_range(1.0..3.0))).max(self.to_call * 2.0);
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
        
        (p1_bet == p2_bet || self.players[0].is_all_in || self.players[1].is_all_in) &&
        (self.players[0].current_bet >= self.to_call || self.players[0].is_folded) &&
        (self.players[1].current_bet >= self.to_call || self.players[1].is_folded)
    }
    
    fn determine_winner(&mut self) {
        let mut rng = rand::thread_rng();
        let winner_idx = rng.gen_range(0..2);
        let winner_name = self.players[winner_idx].name.clone();
        let loser_idx = (winner_idx + 1) % 2;
        let loser = &self.players[loser_idx];
        
        if loser.is_folded {
            self.update_ui(format!("{} wins - opponent folded!", winner_name));
        } else {
            self.update_ui(format!("{} wins the pot!", winner_name));
        }
        
        self.players[winner_idx].collect_pot(self.pot);
        self.pot = 0.0;
        
        if self.players[0].chips < 10.0 || self.players[1].chips < 10.0 {
            self.update_ui("Game Over! Resetting chips...".to_string());
            self.players[0].chips = 10000.0;
            self.players[1].chips = 10000.0;
        }
        
        self.stage = GameStage::HandComplete;
    }
    
    fn handle_player_action(&mut self) {
        if self.players.iter().all(|p| p.is_folded) {
            let active_idx = self.players.iter().position(|p| !p.is_folded).unwrap();
            let winner_name = self.players[active_idx].name.clone();
            self.players[active_idx].collect_pot(self.pot);
            self.pot = 0.0;
            self.update_ui(format!("{} wins!", winner_name));
            self.stage = GameStage::HandComplete;
            self.schedule_action(3000);
            return;
        }
        
        let action = self.get_random_action();
        if self.execute_action(action) {
            if self.check_betting_complete() {
                self.proceed_to_next_street();
            } else {
                self.next_player();
                self.schedule_action(1000 + rand::thread_rng().gen_range(500..2000));
            }
        }
    }
    
    fn proceed_to_next_street(&mut self) {
        for player in &mut self.players {
            player.current_bet = 0.0;
        }
        self.to_call = 0.0;
        
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
                self.schedule_action(3000);
                return;
            }
            GameStage::Showdown => {
                self.determine_winner();
                self.schedule_action(3000);
                return;
            }
            GameStage::HandComplete => {
                self.dealer_position = (self.dealer_position + 1) % 2;
                self.start_new_hand();
                return;
            }
        }
        
        self.current_player = (self.dealer_position + 1) % 2;
        self.schedule_action(2000);
    }
    
    fn schedule_action(&mut self, delay_ms: u64) {
        self.next_action_time = Some(Instant::now() + Duration::from_millis(delay_ms));
        self.pending_action = true;
    }
    
    fn start_timer(&mut self) {
        let game_rc = self.game_rc.clone().unwrap();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
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
                self.determine_winner();
            }
            GameStage::HandComplete => {
                self.dealer_position = (self.dealer_position + 1) % 2;
                self.start_new_hand();
            }
        }
    }
    
    fn update_ui(&self, message: String) {
        if let Some(ui) = self.game_weak.upgrade() {
            let stage_str = match self.stage {
                GameStage::Preflop => "Preflop",
                GameStage::Flop => "Flop",
                GameStage::Turn => "Turn",
                GameStage::River => "River",
                GameStage::Showdown => "Showdown",
                GameStage::HandComplete => "Complete",
            };
            
            let community_cards: Vec<String> = self.community_cards.iter()
                .map(|c| c.to_string())
                .collect();
            
            let community_cards_red: Vec<bool> = self.community_cards.iter()
                .map(|c| c.is_red())
                .collect();
            
            ui.set_player1_name(self.players[0].name.clone().into());
            ui.set_player2_name(self.players[1].name.clone().into());
            ui.set_player1_chips(self.players[0].chips as f32);
            ui.set_player2_chips(self.players[1].chips as f32);
            ui.set_pot_size(self.pot as f32);
            ui.set_game_stage(stage_str.into());
            ui.set_dealer_position(self.dealer_position as i32);
            ui.set_message(message.into());
            ui.set_p1_card1(if self.players[0].hole_cards.len() > 0 { 
                self.players[0].hole_cards[0].to_string().into()
            } else { "".into() });
            ui.set_p1_card2(if self.players[0].hole_cards.len() > 1 { 
                self.players[0].hole_cards[1].to_string().into()
            } else { "".into() });
            ui.set_p2_card1(if self.players[1].hole_cards.len() > 0 { 
                self.players[1].hole_cards[0].to_string().into()
            } else { "".into() });
            ui.set_p2_card2(if self.players[1].hole_cards.len() > 1 { 
                self.players[1].hole_cards[1].to_string().into()
            } else { "".into() });
            
            // Red card properties
            ui.set_p1_card1_red(self.players[0].hole_cards.get(0).map(|c| c.is_red()).unwrap_or(false));
            ui.set_p1_card2_red(self.players[0].hole_cards.get(1).map(|c| c.is_red()).unwrap_or(false));
            ui.set_p2_card1_red(self.players[1].hole_cards.get(0).map(|c| c.is_red()).unwrap_or(false));
            ui.set_p2_card2_red(self.players[1].hole_cards.get(1).map(|c| c.is_red()).unwrap_or(false));
            
            ui.set_flop1(if community_cards.len() > 0 { community_cards[0].clone().into() } else { "".into() });
            ui.set_flop2(if community_cards.len() > 1 { community_cards[1].clone().into() } else { "".into() });
            ui.set_flop3(if community_cards.len() > 2 { community_cards[2].clone().into() } else { "".into() });
            ui.set_turn(if community_cards.len() > 3 { community_cards[3].clone().into() } else { "".into() });
            ui.set_river(if community_cards.len() > 4 { community_cards[4].clone().into() } else { "".into() });
            
            // Community card red properties
            ui.set_flop1_red(community_cards_red.get(0).copied().unwrap_or(false));
            ui.set_flop2_red(community_cards_red.get(1).copied().unwrap_or(false));
            ui.set_flop3_red(community_cards_red.get(2).copied().unwrap_or(false));
            ui.set_turn_red(community_cards_red.get(3).copied().unwrap_or(false));
            ui.set_river_red(community_cards_red.get(4).copied().unwrap_or(false));
            
            ui.set_p1_acting(self.current_player == 0 && !self.players[0].is_folded);
            ui.set_p2_acting(self.current_player == 1 && !self.players[1].is_folded);
            ui.set_p1_folded(self.players[0].is_folded);
            ui.set_p2_folded(self.players[1].is_folded);
            ui.set_p1_current_bet(self.players[0].current_bet as f32);
            ui.set_p2_current_bet(self.players[1].current_bet as f32);
        }
    }
}

fn main() {
    let app = PokerApp::new().unwrap();
    
    app.set_player1_name("Alice".into());
    app.set_player2_name("Bob".into());
    app.set_player1_chips(10000.0);
    app.set_player2_chips(10000.0);
    app.set_pot_size(0.0);
    app.set_game_stage("Starting".into());
    app.set_message("Initializing game...".into());
    
    let game_weak = app.as_weak();
    let game = Rc::new(RefCell::new(PokerGame::new(game_weak)));
    
    // Start the first hand
    game.borrow_mut().start_new_hand();
    
    // Set up the game rc and start the timer
    game.borrow_mut().game_rc = Some(game.clone());
    game.borrow_mut().start_timer();
    
    app.run().unwrap();
}
