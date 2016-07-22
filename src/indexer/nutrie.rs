use std::cmp;
use std::rc::{Rc,Weak};
use std::cell::{RefCell,Ref,RefMut};
use std::mem;

use indexer::*;

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

pub struct TrieResult<'a> {
    pub new_terms: Vec<Term<'a>>,
    pub written_dict_positions: Vec<(TermId, usize)>,
}

pub fn create_trie<'a, I>(mut term_serial: TermId, terms: I, mut docs: TermBuf, mut tfs: TermBuf, mut positions: TermBuf) -> TrieResult<'a>
    where I: Iterator<Item=&'a Term<'a>>
{
    let mut written_positions = Vec::new();
    let mut new_terms = Vec::<Box<Term>>::new();

    let root_term = Term{term: "", term_id: 0};
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

    TrieResult {
        // unbox terms
        new_terms: new_terms.into_iter().map(|t| *t).collect(),
        written_dict_positions: written_positions,
    }
}



// 't: 'n means that terms ('t) can live longer than nodes ('n) It is needed so that root term can
// be allocated in shorter lifetime than that of other terms.  No other reason
type TrieNodeRef<'n, 't: 'n> = Rc<RefCell<_TrieNode<'n, 't>>>;
type TrieNodeWeak<'n, 't: 'n> = Weak<RefCell<_TrieNode<'n, 't>>>;
struct TrieNode<'n, 't: 'n>(TrieNodeRef<'n, 't>);
struct _TrieNode<'n, 't: 'n> {
    t: &'n Term<'t>,
    postings: Option<Postings>,
    parent: Option<TrieNodeWeak<'n, 't>>,
    children: Vec<TrieNodeRef<'n, 't>>,
}

impl<'n, 't: 'n> TrieNode<'n, 't> {
    fn new(parent: Option<TrieNode<'n, 't>>, t: &'n Term<'t>, postings: Option<Postings>) -> TrieNode<'n, 't> {
        TrieNode(Rc::new(RefCell::new(_TrieNode {
            t: t,
            postings: postings,
            parent: parent.map(|p| Rc::downgrade(&p.0.clone())),
            children: Vec::new(),
        })))
    }

    fn parent(&self) -> Option<TrieNode<'n, 't>> {
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

    fn add_child(&mut self, child: TrieNode<'n, 't>) -> TrieNode<'n, 't> {
        let borrow = child.0.clone();
        self.0.borrow_mut().children.push(child.0);
        TrieNode(borrow)
    }

    fn borrow(&self) -> Ref<_TrieNode<'n, 't>> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<_TrieNode<'n, 't>> {
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

impl<'n, 't: 'n> Clone for TrieNode<'n, 't> {
    fn clone(&self) -> TrieNode<'n, 't> {
        TrieNode(self.0.clone())
    }
}
