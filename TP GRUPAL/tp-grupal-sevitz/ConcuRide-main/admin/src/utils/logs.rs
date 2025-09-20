pub fn log_elections(_msg: String) {
    #[cfg(feature = "election_logs")]
    {
        println!("{}", _msg);
    }
}

pub fn log_trips(_msg: String) {
    #[cfg(feature = "trip_logs")]
    {
        println!("{}", _msg);
    }
}
