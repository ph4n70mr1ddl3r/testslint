# AGENTS.md - Poker Simulation Codebase

This document provides guidelines and commands for agentic coding agents operating on this repository.

## Project Overview

A Texas Hold'em poker simulator built with Rust and Slint UI framework. The codebase includes:
- Poker game logic (hand evaluation, betting, pot management)
- Slint-based GUI interface
- Comprehensive unit tests

## Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Rebuild after Slint UI changes
cargo build  # (build.rs automatically recompiles ui.slint)

# Full clean rebuild
cargo clean && cargo build
```

## Test Commands

```bash
# Run all tests
cargo test

# Run a single test by name
cargo test test_hand_evaluator_straight
cargo test test_player_betting
cargo test test_deck_creation

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode
cargo test --release
```

## Linting Commands

```bash
# Run clippy for all warnings
cargo clippy

# Run clippy with fix suggestions
cargo clippy --fix

# Check for formatting
cargo fmt --check

# Auto-format code
cargo fmt
```

## Code Style Guidelines

### Imports
- Use fully qualified paths for `std::collections::HashMap` (already imported at module level)
- Group imports: standard library first, then external crates
- Example:
```rust
use rand::Rng;
use slint::Weak;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
```

### Formatting
- Run `cargo fmt` before committing
- Use 4-space indentation (Rust default)
- Maximum line length: 100 characters (default fmt width)
- Use trailing commas in multi-line expressions

### Types
- Use `u8` for card ranks (2-14 where 11=J, 12=Q, 13=K, 14=A)
- Use `u64` for chip amounts and pot calculations
- Use `usize` for collection indices and counts
- Use `f32` for UI display values (float conversions)

### Naming Conventions
- `PascalCase` for types, enums, and trait implementations
- `snake_case` for functions, variables, and module names
- `UPPER_SNAKE_CASE` for constants
- Prefix getter methods with `get_` (e.g., `get_chips()`, `get_current_bet()`)
- Use descriptive names: `check_straight`, `advance_street`, `update_action_bounds`

### Error Handling
- Use `Result<T, &'static str>` for fallible operations
- Return descriptive string errors (e.g., "Insufficient chips", "Cannot check when a bet is pending")
- Use `?` operator or `.ok()` for error propagation in UI callbacks
- Validate inputs early with guard clauses

### Collections
- Use `Vec` for ordered collections
- Use `HashMap` for frequency counting
- Use `HashSet` for deduplication
- Prefer `.is_empty()` over `.len() > 0`
- Prefer `.first()` over `.get(0)`
- Use `.iter().all()`, `.iter().any()`, `.iter().find()` for queries

### Closures
- Omit type annotations where Rust can infer them
- Avoid redundant type annotations: `map(|c| c.rank)` not `map(|c: &Card| c.rank)`

### Hand Evaluation
- Hand ranking order: HighCard < Pair < TwoPair < ThreeOfAKind < Straight < Flush < FullHouse < FourOfAKind < StraightFlush < RoyalFlush
- Sort ranks in descending order for comparison
- Extract duplicate logic into helper functions (e.g., `has_consecutive_window`)

### UI Updates
- Batch UI updates in `update_ui()` method
- Use `Weak<PokerApp>` references to avoid reference cycles
- Convert types explicitly for UI setters: `self.pot as f32`

### Testing
- Write tests for all hand evaluation scenarios
- Test edge cases: empty deck, insufficient cards, all-in scenarios
- Use `assert_eq!` for expected values, `assert!` for boolean conditions

### Code Review Checklist
- All 14 tests pass: `cargo test`
- Clippy clean: `cargo clippy` (0 warnings)
- Formatted: `cargo fmt`
- No unnecessary type annotations in closures
- Consistent use of `is_empty()` over `len() > 0`
- No unused imports or dead code

## Slint UI Notes

- UI definition in `ui.slint` file
- Build automatically triggered via `build.rs` on `cargo build`
- Component naming: `PokerApp`, `PlayerAreaCompact`, `CardDisplay`, `BetSlider`, `ActionButton`
- Properties use snake_case naming convention
