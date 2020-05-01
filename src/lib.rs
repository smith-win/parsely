//#[macro_use]
extern crate log;

pub mod json;
pub mod internals;

#[cfg(test)]
mod tests {

        
    extern crate env_logger;
    
    //use super::json::{Chars,JsonParser};
    use std::fs::File;
    use std::io::{BufReader};

    fn init_log() {
        env_logger::builder().is_test(true).filter_level(log::LevelFilter::max()).init();
    }

    /// This test checks for a pattern of meat-and-two-veg, 
    /// It is simple test of being able to process sequences
    /// e..g (good) "lamb carrot peas" or "chicken peas carrot"
    /// (bad) "peas lamb carrot"  .. 1st is bad
    /// (bad) "lamb peas chicken" .. 3rd is bad
    #[test]
    fn check_meat_and_two_veg() {
        init_log();
        
        let file = File::open("test.dat");
        let buf_read = BufReader::new(file.unwrap());


    }

}
