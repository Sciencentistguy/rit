use std::collections::HashMap;

/// Compute the difference between two slices of strings, using the Myers diff algorithm
pub fn diff<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Edit<'a>> {
    let myers = Myers { a, b };
    myers.diff()
}

#[derive(Debug)]
struct Myers<'a, 'b> {
    a: &'b [&'a str],
    b: &'b [&'a str],
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EditKind {
    Insert,
    Delete,
    Equal,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Edit<'a> {
    kind: EditKind,
    line: &'a str,
}

impl<'a> Edit<'a> {
    fn new(kind: EditKind, line: &'a str) -> Self {
        Edit { kind, line }
    }
}

impl<'a> Myers<'a, '_> {
    fn diff(&self) -> Vec<Edit<'a>> {
        // TODO: Prealloc capacity?
        let mut diff = Vec::new();
        self.backtrack(|prev_x, prev_y, x, y| {
            let a_line = self.a.get(prev_x as usize);
            let b_line = self.b.get(prev_y as usize);

            if x == prev_x {
                diff.push(Edit::new(EditKind::Insert, b_line.unwrap()));
            } else if y == prev_y {
                diff.push(Edit::new(EditKind::Delete, a_line.unwrap()));
            } else {
                diff.push(Edit::new(EditKind::Equal, a_line.unwrap()));
            }
        });

        diff.reverse();
        diff
    }

    fn backtrack<F>(&self, mut callback: F)
    where
        F: FnMut(i64, i64, i64, i64),
    {
        let mut x = self.a.len() as i64;
        let mut y = self.b.len() as i64;
        for (d, v) in self.shortest_edit().iter().enumerate().rev() {
            let d = d as i64;
            let k = x - y;
            let prev_k = if (k == -d) || ((k != d) && v[&(k - 1)] < v[&(k + 1)]) {
                k + 1
            } else {
                k - 1
            };
            let prev_x = v[&prev_k];
            let prev_y = prev_x - prev_k;
            while x > prev_x && y > prev_y {
                callback(x - 1, y - 1, x, y);
                x -= 1;
                y -= 1;
            }
            if d.is_positive() {
                callback(prev_x, prev_y, x, y);
            }
            x = prev_x;
            y = prev_y;
        }
    }

    fn shortest_edit(&self) -> Vec<HashMap<i64, i64>> {
        let n = self.a.len() as i64;
        let m = self.b.len() as i64;
        let max = n + m;
        let mut v: HashMap<i64, i64> = HashMap::new();
        v.insert(1, 0);
        let mut trace = Vec::new();
        for d in 0..max {
            trace.push(v.clone());

            for k in (-(d as i64)..=d).step_by(2) {
                let mut x = if (k == -d) || ((k != d) && (v[&(k - 1)] < v[&(k + 1)])) {
                    v[&(k + 1)]
                } else {
                    v[&(k - 1)] + 1
                };

                let mut y = x - k;

                while x < n && y < m && self.a[x as usize] == self.b[y as usize] {
                    x += 1;
                    y += 1;
                }

                v.insert(k, x);

                if x >= n && y >= m {
                    return trace;
                }
            }
        }
        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_works() {
        let a = ["a", "b", "b", "a"];
        let b = ["a", "b", "b", "a", "c"];
        let diff = diff(&a, &b);
        println!("{:?}", diff);

        assert_eq!(diff.len(), 5);
        assert_eq!(
            diff.last().unwrap(),
            &Edit {
                kind: EditKind::Insert,
                line: "c"
            }
        );
    }
}
