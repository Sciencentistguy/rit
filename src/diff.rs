use std::{collections::HashMap, fmt::Display};

/// Compute the difference between two slices of strings, using the Myers diff algorithm
pub fn diff<'a>(b: &[&'a str], a: &[&'a str]) -> Vec<Edit<'a>> {
    let a = a
        .iter()
        .enumerate()
        .map(|(index, line)| Line { line, index })
        .collect::<Vec<_>>();
    let b = b
        .iter()
        .enumerate()
        .map(|(index, line)| Line { line, index })
        .collect::<Vec<_>>();
    let myers = Myers { a: &a, b: &b };
    myers.diff()
}

/// Collect a slice of edits into a [`Vec`] of [`Hunk`]s.
pub fn hunks<'a>(edits: &[Edit<'a>]) -> Vec<Hunk<'a>> {
    let mut hunks = Vec::new();
    let mut offset = 0;

    loop {
        while edits.get(offset).map(|x| x.kind) == Some(EditKind::Equal) {
            offset += 1;
        }

        if offset >= edits.len() {
            return hunks;
        }

        offset = offset.saturating_sub(HUNK_CONTEXT + 1);

        let a_start = edits[offset].a_line.map(|x| x.index);
        let b_start = edits[offset].a_line.map(|x| x.index);
        hunks.push(Hunk {
            a_start,
            b_start,
            edits: Vec::new(),
        });

        offset = Hunk::build(hunks.last_mut().unwrap(), edits, offset);
    }
}

#[derive(Debug)]
struct Myers<'a, 'b> {
    a: &'b [Line<'a>],
    b: &'b [Line<'a>],
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EditKind {
    Insert,
    Delete,
    Equal,
}

#[derive(Debug, Clone, Copy)]
enum LineKind {
    A,
    B,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Edit<'a> {
    kind: EditKind,
    a_line: Option<Line<'a>>,
    b_line: Option<Line<'a>>,
}

impl<'a> Edit<'a> {
    fn new(kind: EditKind, a_line: Option<Line<'a>>, b_line: Option<Line<'a>>) -> Self {
        Edit {
            kind,
            a_line,
            b_line,
        }
    }

    pub fn kind(&self) -> EditKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy)]
struct Line<'a> {
    line: &'a str,
    index: usize,
}

impl PartialEq for Line<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
    }
}

impl Eq for Line<'_> {}

impl Display for Edit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let char = match self.kind {
            EditKind::Insert => '+',
            EditKind::Delete => '-',
            EditKind::Equal => ' ',
        };
        let line = self.a_line.unwrap_or_else(|| self.b_line.unwrap());
        write!(f, "{}{}", char, line.line)
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
                diff.push(Edit::new(EditKind::Insert, None, Some(*b_line.unwrap())));
            } else if y == prev_y {
                diff.push(Edit::new(EditKind::Delete, Some(*a_line.unwrap()), None));
            } else {
                diff.push(Edit::new(
                    EditKind::Equal,
                    Some(*a_line.unwrap()),
                    Some(*b_line.unwrap()),
                ));
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
        let mut v = HashMap::new();
        v.insert(1, 0);
        let mut trace = Vec::new();
        for d in 0..max {
            trace.push(v.clone());

            for k in (-d..=d).step_by(2) {
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

pub struct Hunk<'a> {
    a_start: Option<usize>,
    b_start: Option<usize>,
    edits: Vec<Edit<'a>>,
}

/// The amount of context given to each hunk displayed
const HUNK_CONTEXT: usize = 3;

impl<'a> Hunk<'a> {
    fn build(hunk: &mut Self, edits: &[Edit<'a>], mut offset: usize) -> usize {
        let mut counter = -1;

        while counter != 0 {
            if counter > 0 {
                hunk.edits.push(edits[offset].clone())
            }

            offset += 1;
            if offset >= edits.len() {
                break;
            }

            match edits.get(offset + HUNK_CONTEXT).map(|x| x.kind) {
                Some(EditKind::Insert | EditKind::Delete) => {
                    counter = 2 * (HUNK_CONTEXT as isize) + 1
                }
                Some(EditKind::Equal) | None => counter -= 1,
            }
        }

        offset
    }

    pub fn header(&self) -> String {
        let (a_start, a_len) = self.offsets_for(LineKind::A, self.a_start);
        let (b_start, b_len) = self.offsets_for(LineKind::B, self.b_start);

        format!("@@ -{},{} +{},{} @@", a_start, a_len, b_start, b_len)
    }

    fn offsets_for(&self, mode: LineKind, default: Option<usize>) -> (usize, usize) {
        let mut lines = self
            .edits
            .iter()
            .filter_map(|e| match mode {
                LineKind::A => e.a_line,
                LineKind::B => e.b_line,
            })
            .peekable();

        let start = lines
            .peek()
            .map(|x| x.index)
            .or(default)
            .unwrap_or_default();

        let lines = lines.count();

        (start, lines)
    }

    pub fn edits(&self) -> &[Edit] {
        self.edits.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_works() {
        let b = ["a", "b", "b", "a"];
        let a = ["a", "b", "b", "a", "c"];
        let diff = diff(&a, &b);
        println!("{:?}", diff);

        assert_eq!(diff.len(), 5);
        assert_eq!(
            diff.last().unwrap(),
            &Edit {
                kind: EditKind::Insert,
                a_line: None,
                b_line: Some(Line {
                    line: "c",
                    index: 4
                }),
            }
        );
    }
}
