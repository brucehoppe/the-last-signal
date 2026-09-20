use std::collections::{HashMap, VecDeque};
use the_last_signal::core::{Game, Outcome, Pos};

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

#[test]
fn expeditions_can_be_completed_through_normal_actions() {
    for seed in (1..=20).chain([42]) {
        let mut g = Game::new(seed);
        let targets: Vec<Pos> = g
            .archives
            .iter()
            .map(|a| a.pos)
            .chain([g.relay, g.lift])
            .collect();
        for target in targets {
            let mut attempts = 0;
            while g.player.distance(target) > 1 {
                attempts += 1;
                assert!(attempts < 2000, "Route stalled for seed {seed}");
                assert_eq!(g.outcome, Outcome::Exploring, "Seed {seed}");
                if g.hp < 12 && g.medkits > 0 {
                    g.heal();
                } else {
                    let next = next_step(&g, target);
                    g.step(next.x - g.player.x, next.y - g.player.y);
                }
            }
            g.interact();
        }
        assert_eq!(g.outcome, Outcome::Escaped, "Seed {seed}");
        g.validate().unwrap();
        assert!(
            g.replay_matches(),
            "Seed {seed}: the action log must replay to the same win"
        );
    }
}
