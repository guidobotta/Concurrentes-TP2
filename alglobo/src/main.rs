use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
mod model;
use model::parser::Parser;
use model::leader_election::LeaderElection;

fn run() {
    let direccion = "localhost:6000".to_string();
    let mut resultado = String::new();
    let monto = String::from("123.56") + "\n";
    let mut web_service = TcpStream::connect(direccion).unwrap();
    web_service.write(monto.as_bytes()).unwrap();
    let mut reader = BufReader::new(&web_service);
    reader.read_line(&mut resultado).unwrap();

    println!("{}", resultado);
}

fn main() {
    let id = std::env::args().nth(1).unwrap();
    let lider = LeaderElection::new(id.parse::<usize>().unwrap());
    //let aplicacion = Aplicacion::new(id, lider);
    //aplicacion.comenzar();

    let mut entrada = String::new();
    loop {
        let _ = std::io::stdin().read_line(&mut entrada);
        if entrada.contains("SALIR"){
            //aplicacion.finalizar();
            break; 
        }
    }
}

//Se levanta una instancia
//Eleccion de lider

//Loop
    //Si es lider:
        //Si conexion no establecida:
            //Conectar con webservices y las replicas
        //Leer una linea del archivo
        //Procesar pago (enviar a los webservices)
            //Si falla agregar a lista de falladas
        //Actualizar replicas   -> 
            //1. Enviar a cada replica por UDP un mensaje de "Estoy parado en esta linea"
            //2. Quedarse esperando por la respuesta de cada una (timeout). Si no hay respuesta, se reenvian todos.

    //Si no es el lider:
        //Recibir actualizacion del archivo (TIMEOUT)
        //Solicitar nuevo lider si salta timeout

