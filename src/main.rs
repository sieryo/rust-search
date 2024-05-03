use search_engine::model::Model;
use search_engine::server::{self};
use std::fs::File;
use std::io::BufReader;
use std::{env, path::PathBuf, process::ExitCode};

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
        "      index <folder>                Index <folder> nya dan menyimpan indexnya ke index.json"
    );
    eprintln!("      search <index-file>     Mengecek seberapa banyak dokumen yang diindex didalam file tersebut ");
    eprintln!(
        "      serve <index-file>            Menjalankan local http server dengan tampilan website  "
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

            let mut new_model: Model = Model::new();
            new_model.begin_index(PathBuf::from(dir_path));

            new_model.save_model_to_json_file("index.json")?;
        }
        "reindex" => {
            let dir_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: tidak ada directory yang diberi");
            })?;

            let mut current_model: Model =
                serde_json::from_reader(BufReader::new(File::open("index.json").unwrap())).unwrap();
            current_model.begin_index(PathBuf::from(dir_path));

            current_model.save_model_to_json_file("index.json")?;
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
                    let _: Model = serde_json::from_reader(&file).unwrap();
                    println!(
                        "Mengecek total file dari {} yang ada : {}",
                        index_path, "test"
                    );
                }
                Err(_) => {
                    usage(&program);
                    eprintln!("ERROR: File tidak ada");
                }
            }
        }
        "serve" => {
            let index_path = args.next().ok_or_else(|| {
                usage(&program);
                eprintln!("ERROR: tidak ada index yang diberi");
            })?;

            let index_file = File::open(&index_path).map_err(|err| {
                eprintln!("ERROR: Index file : {err}");
            })?;

            let model: Model = serde_json::from_reader(&index_file).unwrap();
            server::begin_server(&model)?;
        }
        _ => usage("Search engine"),
    };
    Ok(())
}
