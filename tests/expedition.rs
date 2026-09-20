use std::collections::{HashMap, VecDeque};
use the_last_signal::core::{Game, Outcome, Pos, FLOORS};

// A test-only omniscient route planner verifies that generated expeditions
// are finishable through the real action API. Never used in model context.
fn next_step(g: &Game, target: Pos) -> Pos {
    let mut previous = HashMap::new();
    previous.insert(g.player, g.player);
    let mut queue = VecDeque::from([g.player]);
    while let Some(p) = queue.pop_front() {
        if p == target {
            break;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = p.offset(dx, dy);
            if g.floor(n) && !previous.contains_key(&n) {
                previous.insert(n, p);
                queue.push_back(n);
            }
        }
    }
    let mut p = target;
    while previous[&p] != g.player {
        p = previous[&p];
    }
    p
}

/// Play one expedition to the end with the omniscient planner, through the
/// same action API a player uses. Returns the finished game.
fn play(seed: u64) -> Game {
    let mut g = Game::new(seed);
    let mut steps = 0;
    while g.outcome == Outcome::Exploring {
        steps += 1;
        assert!(steps < 20_000, "seed {seed} stalled on floor {}", g.floor);
        let target = if let Some(a) = g.archives.iter().find(|a| !a.recovered) {
            a.pos
        } else if !g.restored {
            g.relay
        } else {
            g.lift
        };
        if g.player.distance(target) <= 1 {
            g.interact();
        } else if g.hp < 12 && g.medkits > 0 {
            g.heal();
        } else {
            let next = next_step(&g, target);
            g.step(next.x - g.player.x, next.y - g.player.y);
        }
    }
    g
}

/// Difficulty guard. The planner knows the whole map and charges every foe, so it
/// is a floor for player skill, not a ceiling. If a change to foes, healing or
/// floor layout moves this band, that is a deliberate balance decision.
#[test]
fn difficulty_stays_in_band() {
    let (mut won, mut died_on) = (0, [0u32; FLOORS]);
    let seeds = 100;
    for seed in 1..=seeds {
        let g = play(seed);
        g.validate().unwrap();
        assert!(
            g.replay_matches(),
            "seed {seed}: the log must replay exactly, win or lose"
        );
        if g.outcome == Outcome::Escaped {
            won += 1;
            assert_eq!(g.floor, FLOORS - 1);
            assert_eq!(g.records_found.len(), 9);
        } else {
            died_on[g.floor] += 1;
        }
    }
    println!("planner wins {won}/{seeds}, deaths by floor {died_on:?}");
    assert!(won >= 70, "too hard: only {won}/{seeds} planner wins");
    assert!(won <= 95, "too easy: {won}/{seeds} planner wins");
    assert_eq!(died_on[0], 0, "floor 1 must be safe for the planner");
}

#[test]
fn a_full_three_floor_expedition_can_be_won_and_replayed() {
    let g = (1..=40)
        .map(play)
        .find(|g| g.outcome == Outcome::Escaped)
        .expect("some seed among the first forty is a planner win");
    assert_eq!(g.outcome, Outcome::Escaped);
    assert_eq!((g.floor, g.records_found.len()), (FLOORS - 1, 9));
    assert!(g.replay_matches());
    assert!(
        g.turn > 250,
        "three floors is a real expedition, got {} turns",
        g.turn
    );
}
