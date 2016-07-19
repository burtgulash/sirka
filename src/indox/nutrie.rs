use std::cmp::{self,Ordering};
use std::iter::FromIterator;
use std::collections::BinaryHeap;
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::mem;

use indox::*;

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

fn allocate_term<'a>(arena: &mut Vec<Box<Term<'a>>>, term: Term<'a>) -> *const Term<'a> {
    let boxed_term = Box::new(term);
    let handle = &*boxed_term as *const Term;
    arena.push(boxed_term);
    handle
}

pub fn create_trie<'a, I>(mut term_serial: TermId, terms: I, bk: &mut BKTree, mut docs: TermBuf, mut tfs: TermBuf, mut positions: TermBuf)
    where I: Iterator<Item=&'a Term<'a>>
{
    let root_term = Term{term: "", term_id: 0};
    let mut new_terms = Vec::<Box<Term>>::new();
    let root = TrieNode::new(None, &root_term, None);

    let mut last_node: TrieNode = root.clone();
    let mut parent: TrieNode;

    for current_term in terms {
        let prefix_len = get_common_prefix_len(last_node.borrow().t.term, current_term.term);

        // println!("IT {} {} {}", last_node.borrow().t.term, current_term.term, prefix_len);

        if prefix_len >= last_node.borrow().t.term.len() {
            parent = last_node;
            let parent_clone = parent.clone();
            last_node = parent.add_child(TrieNode::new(
                Some(parent_clone),
                current_term,
                Some(Postings {
                    docs: docs.get_termbuf(current_term.term_id).unwrap(),
                    tfs: tfs.get_termbuf(current_term.term_id).unwrap(),
                    positions: positions.get_termbuf(current_term.term_id).unwrap(),
                }),
            ));
            continue;
        }

        while prefix_len < last_node.parent_term_len() {
            last_node = last_node.parent().unwrap();
            last_node.flush();
        }

        let child_postings = Postings {
            docs: docs.get_termbuf(current_term.term_id).unwrap(),
            tfs: tfs.get_termbuf(current_term.term_id).unwrap(),
            positions: positions.get_termbuf(current_term.term_id).unwrap(),
        };

        if prefix_len == last_node.parent_term_len() {
            parent = last_node.parent().unwrap();
        } else {
            term_serial += 1;
            let last_term = last_node.borrow().t.term;

            let new_term: *const Term = allocate_term(&mut new_terms, Term {
                term: &last_term[..cmp::min(last_term.len(), prefix_len)],
                term_id: term_serial,
            });

            let new_node = {
                let _last_node_borrow = last_node.borrow();
                let ref child2_postings = _last_node_borrow.postings.as_ref().unwrap();
                let postings_to_merge = vec![&child_postings, child2_postings];
                TrieNode::new(
                    last_node.parent(),
                    // UNSAFE: new_term will be alive, because 'new_terms' arena will be dropped
                    // only after the trie is finished
                    unsafe { &*new_term },
                    Some(Postings::merge(&postings_to_merge[..])),
                )
            };

            parent = last_node;
            last_node = new_node.clone();
            mem::swap(&mut *last_node.borrow_mut(), &mut *parent.borrow_mut());

            (&mut *last_node.borrow_mut()).parent = Some(Rc::downgrade(&parent.0));
            // *(last_node.0.borrow().parent.unwrap().upgrade().unwrap().borrow_mut()) = Some(parent);
            parent.0.borrow_mut().children.push(new_node.0);
        }

        let parent_clone = parent.clone();
        last_node = parent.add_child(TrieNode::new(
            Some(parent_clone),
            current_term,
            Some(child_postings)
        ));
    }

    while let Some(parent) = last_node.parent() {
        last_node = parent;
        last_node.flush();
    }
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
        // Switch compare order because Rust's BinaryHeap is a maxheap
        // We want a minheap
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

type TrieNodeRef<'a> = Rc<RefCell<_TrieNode<'a>>>;
type TrieNodeWeak<'a> = Weak<RefCell<_TrieNode<'a>>>;
struct TrieNode<'a>(TrieNodeRef<'a>);
struct _TrieNode<'a> {
    t: &'a Term<'a>,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'a>>,
    children: Vec<TrieNodeRef<'a>>,
}

impl<'a> TrieNode<'a> {
    fn new(parent: Option<TrieNode<'a>>, t: &'a Term<'a>, postings: Option<Postings>) -> TrieNode<'a> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            postings: postings,
            parent: parent.map(|p| Rc::downgrade(&p.0.clone())),
            children: Vec::new(),
        })))
    }

    fn parent(&self) -> Option<TrieNode<'a>> {
        match self.0.borrow().parent {
            Some(ref weak_link) => Some(TrieNode(weak_link.upgrade().unwrap())),
            None => None,
        }
    }

    fn parent_term_len(&self) -> usize {
        let parent = self.parent().unwrap();
        let pb = parent.borrow();
        pb.t.term.len()
    }

    fn add_child(&mut self, child: TrieNode<'a>) -> TrieNode<'a> {
        let borrow = child.0.clone();
        self.0.borrow_mut().children.push(child.0);
        TrieNode(borrow)
    }

    fn borrow(&self) -> Ref<_TrieNode<'a>> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<_TrieNode<'a>> {
        self.0.borrow_mut()
    }

    fn flush(&self) {
        {
            let self_borrow = self.borrow();
            let prefix = self_borrow.t.term;
            for child in self_borrow.children.iter() {
                // TODO flush
                let child_term = child.borrow().t.term;
                let suffix = &child_term[prefix.len()..];
                //println!("Flushing node {}|{}, term: {}", prefix, suffix, child_term);

                // TODO enable this
                if let Some(ref postings) = child.borrow().postings {
                    let mut iter = postings.docs.iter();
                    while let Some(posting) = iter.next() {
                        //println!("TERM: {}, POSTING: {}", child_term, posting);
                    }
                }
            }

            // Need to store actual borrows first
            let borrows = self_borrow.children.iter().map(|p| { p.borrow() }).collect::<Vec<_>>();
            let postings_to_merge = borrows.iter().map(|p| { p.postings.as_ref().unwrap() }).collect::<Vec<_>>();
            let prefix_postings = Postings::merge(&postings_to_merge[..]);
        }

        self.borrow_mut().children.clear();
    }
}

impl<'a> Clone for TrieNode<'a> {
    fn clone(&self) -> TrieNode<'a> {
        TrieNode(self.0.clone())
    }
}
