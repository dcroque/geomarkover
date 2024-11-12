use std::process::exit;

use geomarkover::{data_reader, markov_chain, osm};

use structopt::StructOpt;

#[derive(StructOpt)]
struct ArgsTransitionMatrix {
    #[structopt(short = "n", long = "name")]
    name: String,
    #[structopt(short = "p", long = "place")]
    place_name: Option<String>,
    #[structopt(short = "f", long = "filepath")]
    nw_graph_path: Option<String>,
    #[structopt(short = "d", long = "datasource", default_value = "osm")]
    data_source: String,
    #[structopt(short = "o", long = "output")]
    show_output: bool,
    #[structopt(short = "s", long = "save")]
    save_results: bool,
}

#[derive(StructOpt)]
enum Cli {
    #[structopt(about = "Calculate transition matrix for a given location.")]
    CalcTransitionMatrix(ArgsTransitionMatrix),
}

#[tokio::main]
async fn main() {
    let cli = Cli::from_args();

    match cli {
        Cli::CalcTransitionMatrix(args) => {
            let data_source = markov_chain::TrafficDataSource::from_str(&args.data_source).await;

            let filepath: String;
            let nw = match args.nw_graph_path {
                Some(path) => {
                    filepath = path.clone();
                    data_reader::NetworkData::new_from_file(args.name, path)
                }
                None => match args.place_name {
                    Some(place) => {
                        filepath = format!("output/{}", args.name);
                        osm::get_data_from_place(&args.name, &place);
                        data_reader::NetworkData::new_from_file(args.name.clone(), filepath.clone())
                    }
                    None => {
                        println!("noop");
                        exit(0)
                    }
                },
            };

            let mut mkv_chain = markov_chain::MarkovChain::new_from_network(data_source, nw).await;
            let t_mtx = markov_chain::TransitionMatrix::new_from_markov_chain(&mkv_chain);
            mkv_chain.calculate_density_from_matrix(&t_mtx, None);

            if args.show_output {
                println!("PRINT");
            }

            if args.save_results {
                if mkv_chain.save_data(filepath.clone(), args.data_source.clone()) {
                    println!(
                        "Saved markov chain data to {}/markov_chain.json",
                        filepath.clone()
                    );
                } else {
                    println!(
                        "Failed to save markov chain data to {}/markov_chain.json",
                        filepath.clone()
                    );
                }

                if t_mtx.save_to_file(filepath.clone(), args.data_source.clone()) {
                    println!(
                        "Saved markov chain data to {}/transition_matrix.csv",
                        filepath.clone()
                    );
                } else {
                    println!(
                        "Failed to save markov chain data to {}/transition_matrix.csv",
                        filepath.clone()
                    );
                }
            }
        }
    }
}
