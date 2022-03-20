extern crate parsely;


use parsely::{internals::ParseResult, json2::JsonParser};
use std::io::BufReader;
use std::time::Instant;

fn main() -> ParseResult<()> {

    let args : Vec<String> = std::env::args().collect();

    let filename =  args.get(1).unwrap();
    
    for i in 0..20 {
        let f = std::fs::File::open( filename ).unwrap();
        let buf = BufReader::with_capacity(1024 * 128, f);

        let json = JsonParser::new(buf);
        let start_time = Instant::now();
        do_parse(json) ?;
        let time_ms = Instant::now().duration_since(start_time).as_millis();
        let bytes = std::fs::metadata(filename).unwrap().len();
        let time_s =  time_ms as f32 / 1000.0;
        // let mut json = JsonParser::new(buf);
        println!("\tTime {} s, {} MB/sec", time_s, bytes as f32 / (1024.0 * 1024.0 * time_s));
    }

    Ok(())

}

fn do_parse<R: std::io::Read>(mut p: JsonParser<R>) -> ParseResult<()> {
    let mut obj_count = 0;
    let mut str_count = 0;
    let mut num_count = 0;
    while let Some(e) = p.next_token()? {
        match e {
            parsely::json2::JsonEvent2::ObjectStart => obj_count += 1,
            parsely::json2::JsonEvent2::Number(_n) => num_count += 1,
            parsely::json2::JsonEvent2::String(_s) => str_count += 1,
            _ => {}
        }
    }
    println!("#objects: {}, strings: {}, numbers: {}", obj_count, str_count, num_count);
    Ok(())
}

