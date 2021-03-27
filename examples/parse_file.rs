extern crate parsely;


use parsely::json2::JsonParser;
use std::io::BufReader;


fn main() {

    let args : Vec<String> = std::env::args().collect();

    let filename =  args.get(1).unwrap();
    
    for i in 0..5 {
        println!("Parsing #{}", i);
        let f = std::fs::File::open( filename ).unwrap();
        let buf = BufReader::new(f);
        // let mut json = JsonParser::new(buf.bytes());
        // json.parse().unwrap();
        let mut json = JsonParser::new(buf);
        json.parse().unwrap();
    }

}

