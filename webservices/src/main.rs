use std::{io::{BufRead, BufReader, Write}, net::{TcpListener, TcpStream}};
use rand::Rng;
use std::{thread, time};

fn simulate_work() -> Result<(), ()> {
    let mut rng = rand::thread_rng();
    let tiempo_trabajo = rng.gen_range(300..1000);
    let ok = rng.gen::<f32>() >= 0.2;
    thread::sleep(time::Duration::from_millis(tiempo_trabajo));

    if ok { Ok(()) } else { Err(()) }
}


fn procesar_conexion(mut stream: TcpStream) {
    let mut numero = String::new();
    loop {
        let mut reader = BufReader::new( &stream);
        let bytes = reader.read_line(&mut numero).unwrap();
        if bytes == 0 { break };    //EOF detectado
        let monto = numero.replace("\n", "").parse::<f64>().unwrap();
        println!("Procesando monto: {}", monto);
        let resultado = match simulate_work() {
            Ok(_) => "OK\n",
            Err(_) => "ERROR\n"
        };
        stream.write(resultado.as_bytes()).unwrap();
    }
}

fn run() {
    let direccion = "localhost:6000".to_string();
    let listener = TcpListener::bind(direccion).unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => procesar_conexion(stream),
            Err(_) => break
        }
    }
}

fn main() {
   run()
}
