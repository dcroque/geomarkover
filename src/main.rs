use std::process::exit;

use geomarkover::{data_reader, google_routes, markov_chain, osm};

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
    data_source: markov_chain::TrafficDataSource,
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

fn main() {
    let cli = Cli::from_args();

    match cli {
        Cli::CalcTransitionMatrix(args) => {
            let nw = match args.nw_graph_path {
                Some(path) => data_reader::NetworkData::new_from_file(args.name, path),
                None => match args.place_name {
                    Some(place) => {
                        osm::get_data_from_place(&args.name, &place);
                        data_reader::NetworkData::new_from_file(
                            args.name.clone(),
                            format!("output/{}", args.name),
                        )
                    }
                    None => {
                        println!("noop");
                        exit(0)
                    }
                },
            };

            let mkv_chain = markov_chain::MarkovChain::new_from_network(args.data_source, nw);

            match args.show_output {
                true => println!("PRINT"),
                false => (),
            }

            match args.save_results {
                true => println!("SAVE"),
                false => (),
            }
        }
    }
}
