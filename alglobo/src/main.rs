use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
mod model;
use model::parser::Parser;


fn run() {
    let direccion = "localhost:6000".to_string();
    let mut resultado = String::new();
    let monto = String::from("123.56") + "\n";
    let mut web_service = TcpStream::connect(direccion).unwrap();
    web_service.write(monto.as_bytes()).unwrap();
    let mut reader = BufReader::new( &web_service);
    reader.read_line(&mut resultado).unwrap();
    
    println!("{}", resultado);
}


fn main() {
    run()
}
