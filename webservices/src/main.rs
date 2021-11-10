use std::{io::{BufRead, BufReader, Write}, net::TcpListener};
use rand::Rng;
use std::{thread, time};

fn simulate_work() -> Result<(), ()> {
    let mut rng = rand::thread_rng();
    let tiempo_trabajo = rng.gen_range(300..1000);
    let ok = rng.gen::<f32>() >= 0.2;
    thread::sleep(time::Duration::from_millis(tiempo_trabajo));

    if ok { Ok(()) } else { Err(()) }
}


fn run() {
    let direccion = "localhost:6000".to_string();
    let listener = TcpListener::bind(direccion).unwrap();
    
    for stream in listener.incoming() {
        let mut numero = String::new();
        if let Ok(mut stream) = stream {
            let mut reader = BufReader::new( &stream);
            reader.read_line(&mut numero).unwrap();
            let monto = numero.replace("\n", "").parse::<f64>().unwrap();
            println!("Procesando monto: {}", monto);
            let resultado = match simulate_work() {
                Ok(_) => "OK",
                Err(_) => "ERROR"
            };
            stream.write(resultado.as_bytes()).unwrap();
        }
    }
}


fn main() {
   run()
}
