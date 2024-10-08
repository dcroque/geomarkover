use std::process::Command;

pub fn get_data_from_place(name: &str, place: &str) {
    // poetry -C python-scripts run python3 osm_tool/__init__.py -p "José Mendes, Florianópolis" -n jose_mendes
    let poetry_run = Command::new("poetry")
        .arg("-C")
        .arg("python-scripts")
        .arg("run")
        .arg("python3")
        .arg("python-scripts/osm_tool/__init__.py")
        .arg("-p")
        .arg(place)
        .arg("-n")
        .arg(name)
        .spawn()
        .expect("Rust hereby announces that Python forsaken ourselves")
        .wait();
}

mod tests {
    use super::*;

    #[test]
    fn get_osm_data() {
        get_data_from_place("jose_mendes", "José Mendes, Florianópolis")
    }
}
