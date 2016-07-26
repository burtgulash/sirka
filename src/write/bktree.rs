use std::mem;
use write::*;
use types::*;

#[derive(Clone)]
struct WideTerm<'a> {
    term: &'a str,
    wide_term: Vec<char>,
    term_id: TermId,
}

struct BKNode<'a> {
    t: WideTerm<'a>,
    distance: usize,
    children: Vec<BKNode<'a>>,
}

pub struct BKTree<'a> {
    root: BKNode<'a>,
    size: usize,
}

#[derive(Debug)]
pub struct BKFindResult<'a> {
    distance: usize,
    term: &'a str,
    term_id: TermId,
}

fn to_wide(s: &str) -> Vec<char> {
    s.chars().collect()
}

#[allow(dead_code)]
fn from_wide(s: &Vec<char>) -> String {
    s.iter().cloned().collect()
}

fn metric(a: &Vec<char>, b: &Vec<char>) -> usize {
    let alen = a.len();
    let blen = b.len();
    let mut buf: Vec<usize> = vec![0; 2 * (alen + 1)];

    let (mut v0, mut v1) = (0, alen + 1);

    for j in 0 .. alen + 1 {
        buf[v1 + j] = j;
    }

    for i in 1 .. blen + 1 {
        mem::swap(&mut v0, &mut v1);
        buf[v1] = i;
        let bchar = b[i - 1];

        for j in 1 .. alen + 1 {
            let substitution = (bchar != a[j - 1]) as usize;

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

impl<'a> BKNode<'a> {
    fn new(t: &WideTerm<'a>, d: usize) -> BKNode<'a> {
        BKNode {
            t: t.clone(),
            distance: d,
            children: Vec::new(),
        }
    }

    fn insert(&mut self, t: &WideTerm<'a>) {
        let d = metric(&self.t.wide_term, &t.wide_term);

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

    fn find(&'a self, into: &mut Vec<BKFindResult<'a>>, wide_term: &Vec<char>, maxd: isize) {
        let d = metric(&self.t.wide_term, wide_term) as isize;
        let lowd = d - maxd;
        let highd = d + maxd;

        if d <= maxd {
            into.push(BKFindResult {
                distance: d as usize,
                term: self.t.term,
                term_id: self.t.term_id,
            });
        }

        for ch in self.children.iter() {
            let ch_dist = ch.distance as isize;
            if ch_dist > highd {
                break;
            } else if lowd <= ch_dist && ch_dist <= highd {
                ch.find(into, wide_term, maxd);
            }
        }
    }

    #[allow(dead_code)]
    fn print(&self, lvl: usize) {
        for ch in self.children.iter() {
            println!("{:width$}: {}", ch.distance, ch.t.term,
                     width=lvl * 2);
            ch.print(lvl + 1);
        }
    }
}

impl<'a> BKTree<'a> {
    pub fn new() -> BKTree<'a> {
        let root_term = WideTerm {
            wide_term: Vec::new(),
            term: "",
            term_id: 0,
        };

        BKTree {
            root: BKNode::new(&root_term, 0),
            size: 0,
        }
    }

    pub fn insert(&mut self, term: &'a str, term_id: TermId) {
        let wide_term = WideTerm {
            wide_term: to_wide(term),
            term: term,
            term_id: term_id,
        };
        self.root.insert(&wide_term);
        self.size += 1;
    }

    pub fn insert_term(&mut self, term: &'a Term) {
        self.insert(&term.term, term.term_id);
    }

    pub fn find(&'a self, term: &str, maxd: usize) -> Vec<BKFindResult<'a>> {
        let mut result = Vec::<BKFindResult>::new();
        self.root.find(&mut result, &to_wide(term), maxd as isize);
        result
    }

    pub fn print(&self) {
        self.root.print(0);
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
        bk.insert("bosa", 7);
        bk.insert("bosak", 8);
        bk.insert("bosák", 9);
        bk.insert("pasák", 10);
        bk.insert("osa", 11);
        bk.insert("osada", 12);
        bk.insert("havranu", 13);
        bk.insert("sady", 14);
        bk.insert("sadista", 15);
        bk.print();

        println!("{:?}", bk.find("ros", 2));
        println!("{:?}", bk.find("", 3));
        assert!(true);
    }
}
