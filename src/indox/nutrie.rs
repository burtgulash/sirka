use std::cmp;
use std::mem;
use indox::*;

extern crate typed_arena;

pub struct NuTrie<'a> {
    node_arena: typed_arena::Arena<TrieNode<'a>>,
    term_arena: typed_arena::Arena<Term<'a>>,
    pub root: TrieNode<'a>,
    root_term: Box<Term<'a>>,
}

pub fn get_common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

impl<'a> NuTrie<'a> {
    pub fn new<I>(term_id: TermId, terms: I) -> NuTrie<'a>
            where I: Iterator<Item=&'a mut Term<'a>> {

        let root_term = Box::new(Term{term: "", term_id: 0});
        let mut trie = NuTrie {
            node_arena: typed_arena::Arena::new(),
            term_arena: typed_arena::Arena::new(),
            root: TrieNode::new(None, &*root_term),
            root_term: root_term,
        };

        let mut term_serial = term_id;

        let mut last_node = &mut trie.root as *mut TrieNode;
        let mut parent: *mut TrieNode;

        unsafe {
            for current_term in terms {
                let prefix_len = get_common_prefix_len((*(*last_node).t).term, current_term.term);
                println!("IT {} {} {}", (*(*last_node).t).term, current_term.term, prefix_len);

                if prefix_len >= (*(*last_node).t).term.len() {
                    parent = last_node;
                    last_node = trie.node_arena.alloc(TrieNode::new(Some(parent), current_term as *const Term));
                    (&mut *parent).add_child(last_node);
                    continue;
                }

                while prefix_len < (*(*(&*last_node).parent()).t).term.len() {
                    last_node = (&*last_node).parent();
                    let mut ch = (*last_node).first_child;
                    while let Some(child) = ch {
                        ch = (*child).next;
                    }
                    (*last_node).first_child = None;
                    (*last_node).last_child = None;
                    // TODO
                }

                if prefix_len == (*(*(&*last_node).parent()).t).term.len() {
                    parent = (&*last_node).parent();
                } else {
                    term_serial += 1;
                    let last_term = (*(*last_node).t).term;
                    let newnode = trie.term_arena.alloc(Term {
                        term: &last_term[..cmp::min(last_term.len(), prefix_len)],
                        term_id: term_serial,
                    });

                    let new = trie.node_arena.alloc(TrieNode::new((*last_node).parent, newnode));
                    mem::swap(&mut *last_node, &mut *new);

                    parent = last_node;
                    last_node = new;

                    (&mut *parent).add_child(last_node);
                }

                //println!("{}", current_term.term);
                last_node = trie.node_arena.alloc(TrieNode::new(Some(parent), current_term));
                (&mut *parent).add_child(last_node);
            }
        }

        trie
    }
}

pub struct TrieNode<'a> {
    pub t: *const Term<'a>,
    parent: Option<*mut TrieNode<'a>>,
    first_child: Option<*mut TrieNode<'a>>,
    last_child: Option<*mut TrieNode<'a>>,
    next: Option<*mut TrieNode<'a>>,
}

impl<'a> TrieNode<'a> {
    fn new(parent: Option<*mut TrieNode<'a>>, t: *const Term<'a>) -> TrieNode<'a> {
        TrieNode {
            t: t,
            parent: parent,
            first_child: None,
            last_child: None,
            next: None,
        }
    }

    fn parent(&self) -> *mut TrieNode<'a> {
        self.parent.unwrap()
    }

    unsafe fn add_child(&mut self, child: *mut TrieNode<'a>) {
        (*child).parent = Some(self as *mut TrieNode);
        match self.first_child {
            None => {
                self.first_child = Some(child);
                (*child).next = None;
            },
            _ => (*self.last_child.unwrap()).next = Some(child),
        };

        self.last_child = Some(self as *mut TrieNode);
    }
}
