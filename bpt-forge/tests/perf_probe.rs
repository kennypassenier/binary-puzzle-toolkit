//! Temporary measurement harness; not part of the suite's promises.
#[test]
#[ignore = "measurement only"]
fn probe() {
    use bpt_core::region::{PuzzleKind, Region};
    use bpt_forge::{carve::carve, fill, grade, rng};
    for kind in ["18", "20", "4x8x8", "9x6x6", "8in14", "6in10in14"] {
        let (n, regions) = match kind.parse::<usize>() {
            Ok(n) => (n, vec![Region::square(0, 0, n)]),
            Err(_) => {
                let k = PuzzleKind::from_tag(kind).unwrap();
                (k.grid_size(), k.regions())
            }
        };
        let (mut fill_ms, mut carve_ms) = (0u128, 0u128);
        for index in 0..3u64 {
            let mut r = rng::stream(11, index, 0);
            let t = std::time::Instant::now();
            let sol = fill::solution(n, &regions, &mut r).unwrap();
            fill_ms += t.elapsed().as_millis();
            let t2 = std::time::Instant::now();
            let _ = carve(&sol, &regions, grade::Level::L4, &mut r);
            carve_ms += t2.elapsed().as_millis();
        }
        eprintln!("{kind} (n={n}, 3 puzzles): fill {fill_ms}ms  carve {carve_ms}ms");
    }
}
