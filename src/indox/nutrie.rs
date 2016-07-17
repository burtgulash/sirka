use std::cmp;
use std::mem;
use std::cell::RefCell;
use indox::*;

pub struct NuTrie<'a> {
    new_terms: Vec<Box<Term<'a>>>,
    root: TrieNode<'a>,
}

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

impl<'a> NuTrie<'a> {
    pub fn create<I>(mut term_serial: TermId, terms: I, docs: TermBuf, tf: TermBuf, poss: TermBuf) where I: Iterator<Item=&'a Term<'a>> {

        let mut trie = NuTrie {
            root: TrieNode::new(None, &Term{term: "", term_id: 0}, None),
            new_terms: Vec::new(),
        };

        let mut last_node = &mut trie.root as *mut TrieNode;
        let mut parent: *mut TrieNode;

        unsafe {
            for current_term in terms {
                let prefix_len = get_common_prefix_len((*(*last_node).t).term, current_term.term);

                // println!("IT {} {} {}", (*(*last_node).t).term, current_term.term, prefix_len);

                if prefix_len >= (*(*last_node).t).term.len() {
                    parent = last_node;
                    last_node = (&mut *parent).add_child(TrieNode::new(
                        Some(parent),
                        current_term as *const Term,
                        Some(Postings {
                            docs: Box::new(docs.get_iterator(current_term.term_id).unwrap().cloned()),
                            tfs: Box::new(tf.get_iterator(current_term.term_id).unwrap().cloned()),
                            positions: Box::new(poss.get_iterator(current_term.term_id).unwrap().cloned()),
                        }),
                    ));
                    continue;
                }

                while prefix_len < (*(*(&*last_node).parent()).t).term.len() {
                    last_node = (&*last_node).parent();
                    let prefix = (*(*last_node).t).term;

                    for child in (*last_node).children.iter_mut() {
                        // TODO flush
                        //let child_term = (*child.t).term;
                        //let suffix = &child_term[prefix.len()..];
                        //println!("Flushing node {}|{}, term: {}", prefix, suffix, child_term);
                        //let mut y = &child.postings;
                        //let postings = child.postings.unwrap().docs;
                        if let Some(ref postings) = *child.postings.borrow_mut() {
                            let x = postings.docs.next();
                            println!("tu");
                        }
                        //let x = child.postings.borrow_mut().unwrap().docs;
                        // for posting in child.postings.unwrap().docs.next() {
                        // //for posting in child.postings.as_ref().unwrap().docs {
                        //     println!("posting: {:?}", posting);
                        // }
                        //for posting in child.postings.unwrap().docs {
                        //    println!("posting {:?}", posting);
                        //}
                    }
                    (*last_node).children.clear();
                }

                if prefix_len == (*(*(&*last_node).parent()).t).term.len() {
                    parent = (&*last_node).parent();
                } else {
                    term_serial += 1;
                    let last_term = (*(*last_node).t).term;
                    let new_term = Box::new(Term {
                        term: &last_term[..cmp::min(last_term.len(), prefix_len)],
                        term_id: term_serial,
                    });

                    let mut new_node = Box::new(TrieNode::new(
                        (*last_node).parent,
                        &*new_term,
                        Some(Postings {
                            docs: Box::new(docs.get_iterator(current_term.term_id).unwrap().cloned()),
                            tfs: Box::new(tf.get_iterator(current_term.term_id).unwrap().cloned()),
                            positions: Box::new(poss.get_iterator(current_term.term_id).unwrap().cloned()),
                        }),
                    ));
                    trie.new_terms.push(new_term);

                    parent = last_node;
                    last_node = &mut *new_node as *mut TrieNode;
                    mem::swap(&mut *last_node, &mut *parent);

                    (*last_node).parent = Some(parent);
                    (*parent).children.push(new_node);
                }

                last_node = (&mut *parent).add_child(TrieNode::new(
                    Some(parent),
                    current_term,
                    Some(Postings {
                        docs: Box::new(docs.get_iterator(current_term.term_id).unwrap().cloned()),
                        tfs: Box::new(tf.get_iterator(current_term.term_id).unwrap().cloned()),
                        positions: Box::new(poss.get_iterator(current_term.term_id).unwrap().cloned()),
                    }),
                ));
            }
        }
    }
}

struct Postings<'a> {
    docs: Box<Iterator<Item=DocId> + 'a>,
    tfs: Box<Iterator<Item=DocId> + 'a>,
    positions: Box<Iterator<Item=DocId> + 'a>,
}

struct TrieNode<'a> {
    t: *const Term<'a>,
    postings: RefCell<Option<Postings<'a>>>,
    parent: Option<*mut TrieNode<'a>>,
    children: Vec<Box<TrieNode<'a>>>,
}

impl<'a> TrieNode<'a> {
    fn new(parent: Option<*mut TrieNode<'a>>, t: *const Term<'a>, postings: Option<Postings<'a>>) -> TrieNode<'a> {
        TrieNode {
            t: t,
            postings: RefCell::new(postings),
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
