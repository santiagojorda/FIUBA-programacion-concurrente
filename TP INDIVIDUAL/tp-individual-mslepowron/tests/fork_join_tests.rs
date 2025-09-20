extern crate tp1_mslepowron;

use std::time::Instant;

use tp1_mslepowron::processor::fork_join;

#[test]
fn test_aumento_de_threads() {
    let dir_path = "data";
    let output = "test_threads.json";

    let time_1_thread = Instant::now();
    let _ = fork_join(1, dir_path.to_string(), output.to_string());
    let total_time_one_thread = time_1_thread.elapsed();

    let time_3_threads = Instant::now();
    let _ = fork_join(3, dir_path.to_string(), output.to_string());
    let total_time_3_threads = time_3_threads.elapsed();

    assert!(
        total_time_3_threads < total_time_one_thread,
        "El tiempo de 3 threads es mayor que de 1 thread"
    );
}

#[test]
fn test_fork_join_directory_not_found() {
    let result = fork_join(
        2,
        "non/existent/dir_path".to_string(),
        "test.json".to_string(),
    );
    assert!(
        result.is_err(),
        "Debería tirar error si el directorio no existe"
    );
}
