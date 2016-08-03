#[macro_use]
extern crate sirka;

use std::path::Path;
use std::io::{BufReader,Read};
use std::fs::File;
use sirka::*;

static USAGE: &'static str = "usage: search <indexdir> <term>";

fn create_reader(dirname: &Path, filename: &str) -> BufReader<File> {
    let path = dirname.join(Path::new(filename));
    let reader = BufReader::new(File::open(&path).unwrap());
    reader
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        println!("{}", USAGE);
        std::process::exit(1);
    }

    let indexdir = Path::new(&args[1]);
    let query_to_seach = &args[2..];


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

    let mut pos_reader = create_reader(indexdir, "positions");
    let mut posbuf = Vec::new();
    pos_reader.read_to_end(&mut posbuf).unwrap();

    let input_postings = Postings {
        docs: bytes_to_typed(&docsbuf).to_sequence(),
        tfs: bytes_to_typed(&tfsbuf).to_sequence(),
        positions: bytes_to_typed(&posbuf).to_sequence(),
    };

    let exact = true;
    if let Some(result) = query(&dict, &input_postings, exact, query_to_seach) {
        println!("Found in {} docs!", result.docs.len());
        // println!("docs: {:?}", result.docs);
        // println!("tfs: {:?}", result.tfs);
        // println!("positions: {:?}", result.positions);
    } else {
        println!("Not found!");
    }
}

fn get_postings<DS, TS, PS>(ptr: usize, len: usize, p: &Postings<DS, TS, PS>) -> Postings<DS, TS, PS>
    where DS: Sequence,
          TS: Sequence,
          PS: Sequence
{
    Postings {
        docs: p.docs.subsequence(ptr, len),
        tfs: p.tfs.subsequence(ptr, len + 1),
        positions: p.positions.clone(),
    }
}

fn exact_postings<DS, TS, PS>(header: &TrieNodeHeader, postings: &Postings<DS, TS, PS>) -> Postings<DS, TS, PS>
    where DS: Sequence,
          TS: Sequence,
          PS: Sequence
{
    get_postings(header.postings_ptr as usize, header.num_postings as usize, postings)
}

/*
fn prefix_postings<DS, TS, PS>(header: &TrieNodeHeader, p: &Postings<DS, TS, PS>) -> Postings<DS, TS, PS>
    where DS: Sequence,
          TS: Sequence,
          PS: Sequence
{
    let mut postings_to_merge = Vec::with_capacity(2);
    if header.num_prefix_postings > 0 {
        let cursor = RawCursor::new(exact_postings(header, postings.clone())));
        postings_to_merge.push(cursor);
    }
    if header.num_prefix_postings > 0 {
        let prefix_postings_ptr = (header.postings_ptr + header.num_postings) as usize;
        let prefix_postings_len = header.num_prefix_postings as usize;
        let cursor = RawCursor::new(get_postings(prefix_postings_ptr, prefix_postings_len, postings.clone()));
        postings_to_merge.push(cursor);
    }
    Postings::merge_without_duplicates(&postings_to_merge[..])
}
*/

fn find_terms<'a>(dict: &'a StaticTrie, exact: bool, query: &[&str]) -> Option<Vec<&'a TrieNodeHeader>> {
    let mut headers = Vec::new();
    for term in query.iter() {
        match dict.find_term(term, !exact) {
            Some(header) => headers.push(header),
            None => return None,
        }
    }
    Some(headers)
}

fn query<STRING, DS, TS, PS>(dict: &StaticTrie, postings: &Postings<DS, TS, PS>, exact: bool, q: &[STRING]) -> Option<VecPostings>
    where STRING: AsRef<str>,
          DS: Sequence,
          TS: Sequence,
          PS: Sequence,
{
    let q = q.iter().map(|s| s.as_ref()).collect::<Vec<&str>>();
    println!("Searching query: {:?}", &q);
    let term_headers = tryopt!(find_terms(&dict, exact, &q));

    let mut term_cursors = term_headers.iter().enumerate().map(|(i, th)| {
        println!("Term found. term='{}', numdocs={}", th.term_id, th.num_postings);

        //let mut postings = if exact {
        //    exact_postings(&th, docs.clone(), tfs.clone(), pos.clone())
        //} else {
        //    prefix_postings(&th, docs.clone(), tfs.clone(), pos.clone())
        //};

        let ps = exact_postings(&th, postings);
        RawCursor::new(ps)
    }).collect::<Vec<_>>();

    // sort sequences ascending by their size to make daat skipping much faster
    term_cursors.sort_by(|a, b| {
        a.remains().cmp(&b.remains())
    });

    let intersection = Intersect::new(term_cursors).collect();
    Some(intersection)
}
