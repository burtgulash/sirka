use std::cmp;
use std::cmp::Ordering;
use std::mem;
use std::slice;
use std::iter::FromIterator;
use std::collections::BinaryHeap;
use std::collections::LinkedList;

use indox::*;

pub struct NuTrie<'a> {
    root: TrieNode<'a>,
}

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

fn slice_to_it<'a, T: Clone>(sit: slice::Iter<'a, T>) -> Box<Iterator<Item=T> + 'a> {
    Box::new(sit.cloned())
}

impl<'a> NuTrie<'a> {
    pub fn create<I>(mut term_serial: TermId, terms: I, docs: TermBuf, tf: TermBuf, poss: TermBuf) where I: Iterator<Item=&'a Term<'a>> {

        let root_term = Term{term: "", term_id: 0};
        let mut root = TrieNode::new(None, root_term, None);

        let mut last_node = &mut root as *mut TrieNode;
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
                        Some(Postings::Borrowed(PostingsT{
                            docs: slice_to_it(docs.get_iterator(current_term.term_id).unwrap()),
                            tfs: slice_to_it(tf.get_iterator(current_term.term_id).unwrap()),
                            positions: slice_to_it(poss.get_iterator(current_term.term_id).unwrap()),
                        })),
                    ));
                    continue;
                }

                while prefix_len < (*(&*last_node).parent()).t.term.len() {
                    last_node = (&*last_node).parent();
                    let prefix = (*last_node).t.term;

                    for child in (*last_node).children.iter_mut() {
                        // TODO flush
                        let child_term = child.t.term;
                        let suffix = &child_term[prefix.len()..];
                        println!("Flushing node {}|{}, term: {}", prefix, suffix, child_term);

                        // TODO enable this
                        //if let Some(ref mut postings) = child.postings {
                        //    while let Some(posting) = postings.docs.next() {
                        //        println!("{}", posting);
                        //    }
                        //}
                    }
                    (*last_node).children.clear();
                }

                if prefix_len == (*(&*last_node).parent()).t.term.len() {
                    parent = (&*last_node).parent();
                } else {
                    term_serial += 1;
                    let last_term = (*last_node).t.term;

                    let new_term = Term {
                        term: &last_term[..cmp::min(last_term.len(), prefix_len)],
                        term_id: term_serial,
                    };
                    let mut new_node = Box::new(TrieNode::new(
                        (*last_node).parent,
                        new_term,
                        Some(Postings::Borrowed(PostingsT{
                            docs: slice_to_it(docs.get_iterator(current_term.term_id).unwrap()),
                            tfs: slice_to_it(tf.get_iterator(current_term.term_id).unwrap()),
                            positions: slice_to_it(poss.get_iterator(current_term.term_id).unwrap()),
                        })),
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
                    Some(Postings::Borrowed(PostingsT{
                        docs: slice_to_it(docs.get_iterator(current_term.term_id).unwrap()),
                        tfs: slice_to_it(tf.get_iterator(current_term.term_id).unwrap()),
                        positions: slice_to_it(poss.get_iterator(current_term.term_id).unwrap()),
                    })),
                ));
            }
        }
    }
}

type BorrowedPostingsIter<'a> = Box<Iterator<Item=DocId> + 'a>;
struct PostingsT<T> {
    docs: T,
    tfs: T,
    positions: T,
}
enum Postings<'a> {
    Borrowed(PostingsT<BorrowedPostingsIter<'a>>),
    Owned(PostingsT<Vec<DocId>>),
}

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

impl<'a> Postings<'a> {
    fn merge(to_merge: &mut [&mut Postings]) -> Postings<'a> {
        unsafe {
        let mut owned_its = to_merge.iter_mut().map(|p| p as *mut &mut Postings)
            .filter_map(|p| match **p {
                Postings::Owned(ref x) => Some(PostingsT::<BorrowedPostingsIter> {
                    docs: Box::new(x.docs.iter().cloned()),
                    tfs: Box::new(x.docs.iter().cloned()),
                    positions: Box::new(x.docs.iter().cloned()),
                }),
                _ => None,
            })
            .collect::<Vec<PostingsT<BorrowedPostingsIter>>>();
        }

        let mut its = to_merge.iter_mut()
            .filter_map(|p| match **p {
                Postings::Borrowed(ref mut x) => Some(x),
                _ => None,
            })
            //.extend(owned_its.iter().map(|&p| &p))
            .collect::<Vec<&mut PostingsT<BorrowedPostingsIter>>>();

        let mut h = BinaryHeap::from_iter(its.iter_mut().enumerate().map(|(i, p)| {
            IteratorPointer{it_i: i, current_doc: p.docs.next().unwrap()}
        }));

        let mut res_docs = Vec::<DocId>::new();
        let mut res_tfs = Vec::<DocId>::new();
        let mut res_pos = Vec::<DocId>::new();

        let mut last_doc_id = 0;
        let mut tf = 0;
        let mut tmp_pos: Vec<DocId> = Vec::new();

        macro_rules! ADD_DOC {
            () => {
                res_docs.push(last_doc_id);
                res_tfs.push(tf);

                tmp_pos.sort();
                res_pos.extend_from_slice(&tmp_pos[..]);
                tmp_pos.clear();
            }
        }

        while let Some(mut itptr) = h.pop() {
            if let Some(doc_id) = its[itptr.it_i].docs.next() {
                itptr.current_doc = doc_id;
                h.push(itptr);

                let it_tf = its[itptr.it_i].tfs.next().unwrap();
                if doc_id == last_doc_id {
                    tf += it_tf;
                    for _ in 0..it_tf {
                        let pos = its[itptr.it_i].positions.next().unwrap();
                        tmp_pos.push(pos)
                    }
                } else {
                    if last_doc_id != 0 {
                        ADD_DOC!();
                    }

                    last_doc_id = doc_id;
                    tf = it_tf;
                }
            }
        }
        ADD_DOC!();

        Postings::Owned(PostingsT {
            docs: res_docs,
            tfs: res_tfs,
            positions: res_pos,
        })
    }
}

struct TrieNode<'a> {
    t: Term<'a>,
    postings: Option<Postings<'a>>,
    parent: Option<*mut TrieNode<'a>>,
    children: Vec<Box<TrieNode<'a>>>,
}

impl<'a> TrieNode<'a> {
    fn new(parent: Option<*mut TrieNode<'a>>, t: Term<'a>, postings: Option<Postings<'a>>) -> TrieNode<'a> {
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
