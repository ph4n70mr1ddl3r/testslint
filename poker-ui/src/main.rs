use poker_core::{Card, PlayerAction, PokerGameState};
use slint::Weak;
use std::cell::RefCell;
use std::rc::Rc;

include!(env!("SLINT_INCLUDE_GENERATED"));

fn main() {
    let app = PokerApp::new().unwrap();

    app.set_player1_name("Alice".into());
    app.set_player2_name("Bob".into());
    app.set_player1_chips(poker_core::INITIAL_CHIPS as f32);
    app.set_player2_chips(poker_core::INITIAL_CHIPS as f32);
    app.set_pot_size(0.0);
    app.set_game_stage("Waiting".into());
    app.set_message("Click Start to begin".into());
    app.set_bet_amount(poker_core::CALL_AMOUNT_DEFAULT as f32);
    app.set_min_bet(poker_core::MIN_BET_DEFAULT as f32);
    app.set_max_bet(poker_core::MAX_BET_DEFAULT as f32);
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

pub struct PokerGame {
    state: poker_core::PokerGameState,
    game_weak: Weak<PokerApp>,
}

impl PokerGame {
    pub fn new(game_weak: Weak<PokerApp>) -> Self {
        PokerGame {
            state: poker_core::PokerGameState::new(),
            game_weak,
        }
    }

    pub fn start_new_hand(&mut self) -> Result<(), &'static str> {
        self.state.start_new_hand()?;
        self.update_ui("New hand started".to_string());
        Ok(())
    }

    pub fn perform_action(&mut self, action: PlayerAction) -> Result<(), &'static str> {
        let message = self.state.perform_action(action)?;
        self.update_ui(message);
        Ok(())
    }

    pub fn set_bet_amount(&mut self, amount: f32) {
        self.state.set_bet_amount(amount);
    }

    pub fn is_pending_action(&self) -> bool {
        self.state.is_pending_action()
    }

    pub fn update_ui(&mut self, message: String) {
        if let Some(ui) = self.game_weak.upgrade() {
            if !self.state.players.is_empty() {
                ui.set_player1_name(self.state.players[0].get_name().into());
                ui.set_player1_chips(self.state.players[0].get_chips() as f32);
                ui.set_p1_current_bet(self.state.players[0].get_current_bet() as f32);
                ui.set_p1_folded(self.state.players[0].is_folded());
            }
            if self.state.players.len() >= 2 {
                ui.set_player2_name(self.state.players[1].get_name().into());
                ui.set_player2_chips(self.state.players[1].get_chips() as f32);
                ui.set_p2_current_bet(self.state.players[1].get_current_bet() as f32);
                ui.set_p2_folded(self.state.players[1].is_folded());
            }
            ui.set_pot_size(self.state.pot as f32);
            self.state.update_pot_odds();
            ui.set_pot_odds(self.state.pot_odds);
            ui.set_game_stage(self.state.get_stage_string().into());
            ui.set_dealer_position(self.state.dealer_position as i32);
            ui.set_message(message.into());

            self.update_player_cards(&ui);
            self.update_community_cards(&ui);
            self.update_player_status(&ui);
            self.update_action_controls(&ui);
        }
    }

    fn update_player_cards(&self, ui: &PokerApp) {
        if !self.state.players.is_empty() {
            ui.set_p1_card1(self.hole_card_string(0, 0));
            ui.set_p1_card2(self.hole_card_string(0, 1));
            ui.set_p1_card1_red(self.hole_card_red(0, 0));
            ui.set_p1_card2_red(self.hole_card_red(0, 1));
        }
        if self.state.players.len() >= 2 {
            ui.set_p2_card1(self.hole_card_string(1, 0));
            ui.set_p2_card2(self.hole_card_string(1, 1));
            ui.set_p2_card1_red(self.hole_card_red(1, 0));
            ui.set_p2_card2_red(self.hole_card_red(1, 1));
        }
    }

    fn get_hole_card(&self, player_idx: usize, card_idx: usize) -> Option<&poker_core::Card> {
        self.state
            .players
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

    fn get_community_card(&self, card_idx: usize) -> Option<&poker_core::Card> {
        self.state.community_cards.get(card_idx)
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
        if !self.state.players.is_empty() {
            ui.set_p1_acting(self.state.current_player == 0 && !self.state.players[0].is_folded());
        }
        if self.state.players.len() >= 2 {
            ui.set_p2_acting(self.state.current_player == 1 && !self.state.players[1].is_folded());
        }
    }

    fn update_action_controls(&self, ui: &PokerApp) {
        ui.set_can_check(self.state.can_check());
        ui.set_can_call(self.state.can_call());
        ui.set_can_bet(self.state.can_bet());
        ui.set_can_raise(self.state.can_raise());
        ui.set_can_fold(self.state.can_fold());

        let call_amount = self.state.get_call_amount();
        ui.set_call_amount(call_amount as f32);
        ui.set_bet_amount(self.state.bet_amount as f32);
        ui.set_min_bet(self.state.min_bet as f32);
        ui.set_max_bet(self.state.max_bet as f32);
        ui.set_pot_odds(self.state.pot_odds);
    }
}
