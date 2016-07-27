extern crate indox;

use std::path::Path;
use std::io::{BufReader,Read};
use std::fs::File;
use indox::*;

static USAGE: &'static str = "usage: search <indexdir> <term>";

fn create_reader(dirname: &Path, filename: &str) -> BufReader<File> {
    let path = dirname.join(Path::new(filename));
    let reader = BufReader::new(File::open(&path).unwrap());
    reader
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }

    let indexdir = Path::new(&args[1]);
    let search_term = &args[2];

    let mut meta_reader = create_reader(indexdir, "meta");
    let mut buf = Vec::new();
    meta_reader.read_to_end(&mut buf).unwrap();

    let meta = IndexMeta::from_bytes(&buf[..]);

    let mut trie_reader = create_reader(indexdir, "dict");
    let dictbuf = StaticTrie::read(&mut trie_reader);
    let dict = StaticTrie::new(&dictbuf[..], meta.dict_size as usize, meta.root_ptr as usize, meta.term_buffer_size as usize);

    let mut docs_reader = create_reader(indexdir, "docs");
    let mut docsbuf = Vec::new();
    docs_reader.read_to_end(&mut docsbuf).unwrap();

    let mut tfs_reader = create_reader(indexdir, "tfs");
    let mut tfsbuf = Vec::new();
    tfs_reader.read_to_end(&mut tfsbuf).unwrap();

    println!("META: {:?}", meta);
    match dict.find_term(search_term, true) {
        Some(_) => println!("found!"),
        None => println!("nothing!"),
    };
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

fn find_terms<'a>(dict: &'a StaticTrie, query: &[&str]) -> Option<Vec<&'a TrieNodeHeader>> {
    let mut headers = Vec::new();
    for term in query.iter() {
        match dict.find_term(term, true) {
            Some(header) => headers.push(header),
            None => return None,
        }
    }
    Some(headers)
}

fn daat<'a, S: Sequence<'a>>(docs: S, tfs: S, query_nodes: &[&TrieNodeHeader]) -> bool {
    let mut doc_sliders = query_nodes.iter().map(|n| {
        let mut slider = docs.slider();
        slider.skip_n(n.postings_ptr as usize);
        slider
    });
    let mut tf_sliders = query_nodes.iter().map(|n| {
        let mut slider = tfs.slider();
        slider.skip_n(n.postings_ptr as usize);
        slider
    });
    false
}
