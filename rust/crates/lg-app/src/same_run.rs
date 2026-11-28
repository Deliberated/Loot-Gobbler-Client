//! One dungeon run recorded twice (two Loot Gobblers of one player both there, their records merged by the sync):
//! the same dungeon ending within a few seconds. Only one of them is kept.

/// Two records of a dungeon whose ends are at most this far apart are the same run.
pub const WINDOW_MS: i64 = 5_000;

/// Which of `items` to drop as copies of another: per dungeon, records ending within [`WINDOW_MS`] of each other are
/// one run, of which the one ranking highest stays (on a tie the first). `key` gives a record's dungeon and end;
/// `rank` how good a record it is.
pub fn copies<T, R: PartialOrd>(items: &[T], key: impl Fn(&T) -> (&str, i64), rank: impl Fn(&T) -> R) -> Vec<bool> {
    let mut gone = vec![false; items.len()];
    for i in 0..items.len() {
        if gone[i] {
            continue;
        }
        for j in i + 1..items.len() {
            if gone[j] {
                continue;
            }
            let ((da, ea), (db, eb)) = (key(&items[i]), key(&items[j]));
            if da != db || (ea - eb).abs() > WINDOW_MS {
                continue;
            }
            if rank(&items[j]) > rank(&items[i]) {
                gone[i] = true;
                break;
            }
            gone[j] = true;
        }
    }
    gone
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_best_of_each_run() {
        // (dungeon, end, rank)
        let runs = [("Nest", 10_000, 1), ("Nest", 13_000, 5), ("Nest", 30_000, 1), ("Lab", 13_500, 1), ("Nest", 13_000, 5)];
        let gone = copies(&runs, |r| (r.0, r.1), |r| r.2);
        // the first two are one run (the better stays), the fifth a tie of the second (the first of them stays); the
        // third ends too late, the fourth is another dungeon
        assert_eq!(gone, vec![true, false, false, false, true]);
    }
}
