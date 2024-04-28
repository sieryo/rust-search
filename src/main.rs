use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::ExitCode,
};
use xml::reader::{EventReader, XmlEvent};
use tiny_http::{Response, Server};


#[derive(Debug)]
struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
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

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left();

        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|c| c.is_numeric()));
        }

        if self.content[0].is_alphabetic() {
            return Some(self.chop_while(|c| c.is_alphanumeric()));
        }

        return Some(self.chop(1));
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> io::Result<String> {
    let file = File::open(file_path)?;
    let er = EventReader::new(file);

    let mut content = String::new();
    for event in er.into_iter() {
        if let XmlEvent::Characters(text) = event.expect("TODO") {
            content.push_str(&text);
            content.push_str(" ");
        }
    }
    Ok(content)
}

type TermFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

fn main() {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    };
}

fn usage(command: &str) -> () {
    eprintln!("Penggunaan {command} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands");
    eprintln!(
        "      index <folder>           Index <folder> nya dan menyimpan indexnya ke index.json"
    );
    eprintln!("      search <index-file>      Mengecek seberapa banyak dokumen yang diindex didalam file tersebut ");
    eprintln!(
        "      serve                    Menjalankan local http server dengan tampilan website  "
    );
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let program = args.next().expect("path to program harus disediakan");

    let subcommand = args.next().ok_or_else(|| {
        usage(&program);
        eprintln!("ERROR: tidak ada subcommand yang diberi");
    })?;

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: tidak ada directory yang diberi");
            })?;

            let mut tf_freq = TermFreqIndex::new();

            index_all(PathBuf::from(dir_path), &mut tf_freq);

            let index_path = "index.json";
            let index_file = File::create(&index_path).unwrap();
            println!("Saving data to {index_path}...");
            serde_json::to_writer(index_file, &tf_freq).unwrap();
        }
        "search" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: tidak ada index yang diberi");
            })?;
            let index_file = File::open(&index_path);

            match index_file {
                Ok(file) => {
                    println!("Getting data from {}...", index_path);
                    let tf_index: TermFreqIndex = serde_json::from_reader(&file).unwrap();
                    println!(
                        "Mengecek total index dari {} yang ada : {}",
                        index_path,
                        tf_index.len()
                    );
                }
                Err(_) => {
                    usage(&program);
                    eprintln!("ERROR: File tidak ada");
                }
            }
        }
        "serve" => {
            let server = Server::http("127.0.0.1:3000").map_err(|err| {
                eprintln!("ERROR: tidak bisa menjalankan HTTP server dengan error : {err}");
            })?;

            println!("INFO: listening at http://127.0.0.1:3000");

            for request in server.incoming_requests() {
                println!("INFO: mendapatkan request. method: {:?}, url: {:?}", request.method(), request.url());
                let response = Response::from_string("hello");
                request.respond(response).unwrap_or_else(|_| {
                    eprintln!("Tidak bisa mengirim response")
                });
            }

            todo!("Not implemented");
        }
        _ => usage("Search engine"),
    };
    Ok(())
}

fn index_folder(path: &PathBuf, tf_freq: &mut TermFreqIndex) -> TermFreq {
    let dir = fs::read_dir(&path).unwrap();
    let tf = TermFreq::new();
    for entry in dir {
        let path = entry.unwrap().path();

        if path.is_dir() {
            index_folder(&path, tf_freq);
        } else {
            if let Some(tf) = index_file(&path) {
                tf_freq.insert(path.clone(), tf);
            }
        }
    }
    tf
}

fn index_file(path: &PathBuf) -> Option<TermFreq> {
    match path.extension() {
        Some(e) => {
            if e == "xhtml" {
                let content = read_entire_xml_file(&path)
                    .unwrap()
                    .to_lowercase()
                    .chars()
                    .collect::<Vec<_>>();

                let mut tf = TermFreq::new();

                println!("Indexing {:?}....", path);

                for token in Lexer::new(&content) {
                    let term = token.iter().collect::<String>().to_lowercase();

                    if let Some(freq) = tf.get_mut(&term) {
                        *freq += 1;
                    } else {
                        tf.insert(term, 1);
                    }
                }
                Some(tf)
            } else {
                return None;
            }
        }
        None => {
            return None;
        }
    }
}

fn index_all(path: PathBuf, tf_freq: &mut TermFreqIndex) -> io::Result<()> {
    if path.is_file() {
        if let Some(tf) = index_file(&path) {
            tf_freq.insert(path.clone(), tf);
        }
    }

    let dir = fs::read_dir(&path)?;

    for entry in dir {
        let path = entry?.path();

        if path.is_dir() {
            index_folder(&path, tf_freq);
        } else {
            if let Some(tf) = index_file(&path) {
                tf_freq.insert(path.clone(), tf);
            }
        }
    }
    Ok(())
}
