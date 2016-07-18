use std::cmp;
use std::cmp::Ordering;
use std::mem;
use std::iter::FromIterator;
use std::collections::BinaryHeap;

use indox::*;

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

pub fn create_trie<'a, I>(mut term_serial: TermId, terms: I, mut docs: TermBuf, mut tfs: TermBuf, mut positions: TermBuf)
    where I: Iterator<Item=&'a Term<'a>>
{
    let root_term = Term{term: "", term_id: 0};
    let mut root = TrieNode::new(None, root_term, None);

    let mut last_node: *mut TrieNode = &mut root;
    let mut parent: *mut TrieNode;

    unsafe {
        for current_term in terms {
            let prefix_len = get_common_prefix_len((*last_node).t.term, current_term.term);

            // println!("IT {} {} {}", (*last_node).t.term, current_term.term, prefix_len);

            if prefix_len >= (*last_node).t.term.len() {
                parent = last_node;
                last_node = (&mut *parent).add_child(TrieNode::new(
                    Some(parent),
                    current_term.clone(),
                    Some(Postings {
                        docs: docs.get_termbuf(current_term.term_id).unwrap(),
                        tfs: tfs.get_termbuf(current_term.term_id).unwrap(),
                        positions: positions.get_termbuf(current_term.term_id).unwrap(),
                    }),
                ));
                continue;
            }

            while prefix_len < (*(&*last_node).parent()).t.term.len() {
                last_node = (&*last_node).parent();
                flush_node(&mut *last_node);
            }

            let child_postings = Postings {
                docs: docs.get_termbuf(current_term.term_id).unwrap(),
                tfs: tfs.get_termbuf(current_term.term_id).unwrap(),
                positions: positions.get_termbuf(current_term.term_id).unwrap(),
            };

            if prefix_len == (*(&*last_node).parent()).t.term.len() {
                parent = (&*last_node).parent();
            } else {
                term_serial += 1;
                let last_term = (*last_node).t.term;

                let new_term = Term {
                    term: &last_term[..cmp::min(last_term.len(), prefix_len)],
                    term_id: term_serial,
                };

                let ref child2_postings = (*last_node).postings.as_mut().unwrap();
                let postings_to_merge = vec![&child_postings, child2_postings];

                let mut new_node = Box::new(TrieNode::new(
                    (*last_node).parent,
                    new_term,
                    Some(Postings::merge(&postings_to_merge[..])),
                ));

                parent = last_node;
                last_node = &mut *new_node as *mut TrieNode;
                mem::swap(&mut *last_node, &mut *parent);

                (*last_node).parent = Some(parent);
                (*parent).children.push(new_node);
            }

            last_node = (&mut *parent).add_child(TrieNode::new(
                Some(parent),
                current_term.clone(),
                Some(child_postings)
            ));
        }

        while let Some(parent) = (*last_node).parent {
            last_node = parent;
            flush_node(&mut *last_node);
        }
    }
}

fn flush_node(node: &mut TrieNode) {
    let prefix = node.t.term;
    for child in node.children.iter() {
        // TODO flush
        let child_term = child.t.term;
        let suffix = &child_term[prefix.len()..];
        // println!("Flushing node {}|{}, term: {}", prefix, suffix, child_term);

        // TODO enable this
        if let Some(ref postings) = child.postings {
            let mut iter = postings.docs.iter();
            while let Some(posting) = iter.next() {
                println!("TERM: {}, POSTING: {}", child_term, posting);
            }
        }
    }

    let prefix_postings = {
        let postings_to_merge = node.children.iter()
            .map(|p| {
                p.postings.as_ref().unwrap()
            })
            .collect::<Vec<_>>();
        Postings::merge(&postings_to_merge[..])
    };

    node.children.clear();
}

struct PostingsT<T> {
    docs: T,
    tfs: T,
    positions: T,
}
type Postings = PostingsT<Vec<DocId>>;

#[derive(Clone, Copy)]
struct IteratorPointer {
    it_i: usize,
    current_doc: DocId,
}

impl Ord for IteratorPointer {
    fn cmp(&self, other: &Self) -> Ordering {
        (&-(self.current_doc as isize)).cmp(&-(other.current_doc as isize))
    }
}

impl PartialOrd for IteratorPointer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IteratorPointer {
    fn eq(&self, other: &Self) -> bool {
        self.current_doc == other.current_doc
    }
}

impl Eq for IteratorPointer {}

fn keep_unique<T: Copy + PartialEq>(xs: &[T]) -> Vec<T> {
    let mut res = Vec::new();
    if xs.len() > 0 {
        let mut group_elem = xs[0];
        for x in xs[1..].into_iter().cloned() {
            if x != group_elem {
                res.push(group_elem);
                group_elem = x;
            }
        }
        res.push(group_elem);
    }
    res
}

impl Postings {
    fn merge(to_merge: &[&Postings]) -> Postings {
        let mut its = to_merge.iter().map(|p| PostingsT {
            docs: p.docs.iter().cloned(),
            tfs: p.tfs.iter().cloned(),
            positions: p.positions.iter().cloned(),
        }).collect::<Vec<PostingsT<_>>>();

        let mut h = BinaryHeap::from_iter(its.iter_mut().enumerate().map(|(i, p)| {
            IteratorPointer{it_i: i, current_doc: p.docs.next().unwrap()}
        }));

        let mut res_docs = Vec::<DocId>::new();
        let mut res_tfs = Vec::<DocId>::new();
        let mut res_pos = Vec::<DocId>::new();

        let mut last_doc_id = 0;
        let mut tmp_pos: Vec<DocId> = Vec::new();

        macro_rules! ADD_DOC {
            () => {
                tmp_pos.sort();
                let unique_positions = keep_unique(&tmp_pos);
                res_pos.extend_from_slice(&unique_positions);
                tmp_pos.clear();

                res_docs.push(last_doc_id);
                res_tfs.push(unique_positions.len() as DocId);
            }
        }

        while let Some(mut itptr) = h.pop() {
            if let Some(doc_id) = its[itptr.it_i].docs.next() {
                itptr.current_doc = doc_id;
                h.push(itptr);

                let it_tf = its[itptr.it_i].tfs.next().unwrap();
                if doc_id == last_doc_id {
                    for _ in 0..it_tf {
                        let pos = its[itptr.it_i].positions.next().unwrap();
                        tmp_pos.push(pos)
                    }
                } else {
                    if last_doc_id != 0 {
                        ADD_DOC!();
                    }

                    last_doc_id = doc_id;
                }
            }
        }
        ADD_DOC!();

        Postings {
            docs: res_docs,
            tfs: res_tfs,
            positions: res_pos,
        }
    }
}

struct TrieNode<'a> {
    t: Term<'a>,
    postings: Option<Postings>,
    parent: Option<*mut TrieNode<'a>>,
    children: Vec<Box<TrieNode<'a>>>,
}

impl<'a> TrieNode<'a> {
    fn new(parent: Option<*mut TrieNode<'a>>, t: Term<'a>, postings: Option<Postings>) -> TrieNode<'a> {
        TrieNode {
            t: t,
            postings: postings,
            parent: parent,
            children: Vec::new(),
        }
    }

    fn parent(&self) -> *mut TrieNode<'a> {
        self.parent.unwrap()
    }

    unsafe fn add_child(&mut self, child: TrieNode<'a>) -> *mut TrieNode<'a> {
        let mut newnode = Box::new(child);
        let ret = &mut *newnode as *mut TrieNode;
        self.children.push(newnode);
        ret
    }
}
