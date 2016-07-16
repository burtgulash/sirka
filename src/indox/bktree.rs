use indox::*;
use std::mem;

#[derive(Clone)]
struct WideTerm {
    term: Vec<char>,
    term_id: TermId,
}

struct BKNode {
    t: WideTerm,
    distance: usize,
    children: Vec<BKNode>,
}

pub struct BKTree {
    root: BKNode,
    size: usize,
}

fn metric(a: &WideTerm, b: &WideTerm) -> usize {
    let alen = a.term.len();
    let blen = b.term.len();
    let mut buf: Vec<usize> = vec![0; 2 * (alen + 1)];

    let (mut v0, mut v1) = (0, alen + 1);

    for j in 0 .. alen + 1 {
        buf[v1 + j] = j;
    }

    for i in 1 .. blen + 1 {
        mem::swap(&mut v0, &mut v1);
        buf[v1] = i;
        let bchar = b.term[i - 1];

        for j in 1 .. alen + 1 {
            let substitution = (bchar != a.term[j - 1]) as usize;

            let x0 = buf[v1 + j - 1] + 1;
            let x1 = buf[v0 + j] + 1;
            let x2 = buf[v0 + j - 1] + substitution;

            let mut min = x0;
            if min > x1 {
                min = x1;
            }
            if min > x2 {
                min = x2;
            }

            buf[v1 + j] = min;
        }
    }

    buf[v1 + alen]
}

impl BKNode {
    fn new(t: &WideTerm, d: usize) -> BKNode {
        BKNode {
            t: t.clone(),
            distance: d,
            children: Vec::new(),
        }
    }

    fn insert(&mut self, t: &WideTerm) {
        let d = metric(&self.t, t);

        let mut pos = self.children.len();
        for (i, ch) in self.children.iter_mut().enumerate() {
            if ch.distance == d {
                ch.insert(t);
                return;
            } else if ch.distance > d {
                pos = i;
                break;
            }
        }
        self.children.insert(pos, BKNode::new(t, d));
    }

    fn print(&self, lvl: usize) {
        for ch in self.children.iter() {
            println!("{:width$}: {}", ch.distance,
                     ch.t.term.iter().cloned().collect::<String>(),
                     width=lvl * 2);
            ch.print(lvl + 1);
        }
    }
}

impl BKTree {
    pub fn new() -> BKTree {
        let root_term = WideTerm { term: Vec::new(), term_id: 0 };

        BKTree {
            root: BKNode::new(&root_term, 0),
            size: 0,
        }
    }

    pub fn print(&self) {
        self.root.print(0);
    }

    pub fn insert(&mut self, term: &str, term_id: TermId) {
        let wide_term = WideTerm {
            term: term.chars().collect(),
            term_id: term_id
        };
        self.root.insert(&wide_term);
        self.size += 1;
    }

    pub fn insert_term(&mut self, term: &Term) {
        self.insert(term.term, term.term_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test1() {
        let mut bk = BKTree::new();
        bk.insert("autobus", 1);
        bk.insert("panecku", 2);
        bk.insert("krakora", 3);
        bk.insert("hovna", 4);
        bk.insert("rovna", 5);
        bk.insert("rosa", 6);
        bk.insert("bosa", 6);
        bk.insert("bosak", 6);
        bk.insert("bosák", 6);
        bk.print();
        assert!(true);
    }
}
