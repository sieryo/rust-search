use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};
use serde::{Serialize, Deserialize};
use xml::{reader::XmlEvent, EventReader};
use std::collections::HashMap;

pub type DocFreq = HashMap<String, usize>;
pub type TermFreq = HashMap<String, usize>;
pub type TermFreqPerDoc = HashMap<PathBuf, (usize,TermFreq)>;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Model {
    pub tfpd: TermFreqPerDoc,
    pub df: DocFreq,
}

pub fn calculate_tf(t: &str, count_all_term_tf_table: &(usize, TermFreq)) -> f32 {
    let count_term_in_doc = count_all_term_tf_table.1.get(t).cloned().unwrap_or(0) as f32;

    count_term_in_doc / count_all_term_tf_table.0 as f32
}

pub fn calculate_idf(t: &str, n: usize, df: &DocFreq) -> f32 {
    let n = n as f32;
    let total_count_for_term_in_document = df.get(t).cloned().unwrap_or(1) as f32;

    // println!("{} {}", n, total_count_for_term_in_document.max(1f32));
    (n / total_count_for_term_in_document.max(1f32)).log2()
}

#[derive(Debug)]
pub struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Self {
        Self { content }
    }
    fn trim_left(&mut self) {
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        &token
    }

    fn chop_while<P>(&mut self, mut predicate: P) -> &'a [char]
    where
        P: FnMut(&char) -> bool,
    {
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        return self.chop(n);
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|c| c.is_numeric()).iter().collect());
        }

        if self.content[0].is_alphabetic() {
            return Some(self.chop_while(|c| c.is_alphanumeric()).iter().collect());
        }

        return Some(self.chop(1).iter().collect());
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

pub fn parse_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let er = EventReader::new(BufReader::new(file));

    let mut content = String::new();
    for event in er.into_iter() {
        if let XmlEvent::Characters(text) = event.expect("TODO") {
            content.push_str(&text);
            content.push_str(" ");
        }
    }
    Ok(content)
}

fn index_folder(path: &PathBuf, model: &mut Model) {
    let dir = fs::read_dir(&path).unwrap();
    for entry in dir {
        let path = entry.unwrap().path();

        if path.is_dir() {
            index_folder(&path, model);
        } else {
            if let Some(tf) = index_file(&path, model) {
                model.tfpd.insert(path, tf);
            }
        }
    }
}

fn index_file(path: &PathBuf, model: &mut Model) -> Option<(usize, TermFreq)> {
    match path.extension() {
        Some(e) => {
            if e == "xhtml" {
                println!("Indexing {:?}....", path);
                let content = parse_entire_xml_file(&path)
                    .unwrap()
                    .to_lowercase()
                    .chars()
                    .collect::<Vec<_>>();

                let mut tf = TermFreq::new();

                let mut total_count_term_in_doc = 0;
                for term in Lexer::new(&content) {
                    if let Some(freq) = tf.get_mut(&term) {
                        *freq += 1;
                    } else {
                        tf.insert(term, 1);
                    }
                    total_count_term_in_doc += 1;
                }

                for t in tf.keys() {
                    if let Some(freq) = model.df.get_mut(t) {
                        *freq += 1
                    }else {
                        model.df.insert(t.to_string(), 1);
                    }
                }

                Some((total_count_term_in_doc,tf))
            } else {
                return None;
            }
        }
        None => {
            return None;
        }
    }
}

pub fn index_all_folder(path: PathBuf, model: &mut Model) -> io::Result<()> {
    if path.is_file() {
        if let Some(tf) = index_file(&path, model) {
            model.tfpd.insert(path, tf);
        }
        return Ok(());
    }

    index_folder(&path, model);
    Ok(())
}

pub fn search_query<'a>(model: &'a Model, query: &'a [char]) -> Vec<(&'a Path, f32)> {
    let mut rank_tf: Vec<(&Path, f32)> = Vec::new();
    // Mengecek semua isinya (corpust)
    for (path, count_all_term_tf_table) in model.tfpd.iter() {
        // Untuk setiap path dan tf_table (document) yang sudah dijadikan unique.
        let mut rank = 0f32;
        for term in Lexer::new(&query) {
            let idf = calculate_idf(&term,model.tfpd.len(), &model.df);
            let tfidf = calculate_tf(&term, &count_all_term_tf_table) * idf;
            rank += tfidf;
        }
        rank_tf.push((&path, rank));
    }

    rank_tf.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    rank_tf.reverse();

    rank_tf
}
