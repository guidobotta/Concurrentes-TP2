use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
mod model;
use model::error::{Resultado};
use model::parser::Parser;
use model::leader_election::LeaderElection;
use model::aplicacion::Aplicacion;

//Esto se tiene que borrar de aca
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

fn procesar(id: usize, path: String) -> Resultado<()> {

    let parseador = Parser::new(path)?;
    let lider = LeaderElection::new(id);
    let aplicacion = Aplicacion::new(id, lider, parseador);

    let mut entrada = String::new();
    loop {
        let _ = std::io::stdin().read_line(&mut entrada);
        if entrada.contains("SALIR"){
            aplicacion.finalizar();
            break; 
        }
    }
    Ok(())
}

fn main() {

    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => { 
            println!("Se debe indicar un path a un archivo de entrada");
            return
        } 
    };

    let id = match std::env::args().nth(2).and_then(|a| a.parse::<usize>().ok()) {
        Some(r) => r,
        None => {
            println!("Se debe indicar un id numerico para el nodo");
            return
        }
    };

    if let Err(err) = procesar(id, path) {
        println!("{}", err)
    }
}

