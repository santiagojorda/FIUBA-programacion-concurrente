use std::env;
use std::time::Instant;

mod game;
mod language;
mod output_data;
mod processor;
mod review_error;
mod review_record;
mod review_result;
mod top_review;
use processor::fork_join;

const ARGUMENTS_REQUIRED: usize = 4;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != ARGUMENTS_REQUIRED {
        eprintln!("Faltan o sobran argumentos. El formato esperado es -> <input-path> <num-threads> <output-file-name>");
        return;
    }

    let path = &args[1];
    let amount_threads: usize = match args[2].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("La cantidad de hilos a utilizar debe ser un número entero.");
            return;
        }
    };
    let output_path = &args[3];

    let start = Instant::now();

    match fork_join(amount_threads, path.to_string(), output_path.to_string()) {
        Ok(_) => {
            println!("{:?}", start.elapsed());
            println!("Process completed. Output file generated.")
        }
        Err(e) => e.display_error(),
    }
}
