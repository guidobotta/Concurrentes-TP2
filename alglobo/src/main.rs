mod model;
use model::aplicacion::Aplicacion;
use common::error::Resultado;
use model::comando::Comando;
use model::eleccion_lider::EleccionLider;
use model::parser::Parser;
use std::sync::mpsc::channel;


fn procesar(id: usize, path_pagos: String, path_fallidos: String) -> Resultado<()> {
    let parseador = Parser::new(path_pagos)?;
    let lider = EleccionLider::new(id);
    let (enviador, receptor) = channel::<Comando>();
    let app = Aplicacion::new(id, lider, parseador, receptor)?;

    loop {
        let mut entrada = String::new();
        let _ = std::io::stdin().read_line(&mut entrada);
        entrada = entrada.replace("\n", "");

        if let Ok(comando) = Comando::decodificar(&entrada) {
            enviador.send(comando.clone()).unwrap(); //TODO
            if let Comando::FINALIZAR = comando { break; }
        } else {
            println!("[Aplicacion]: Comando no interpretado")
        }
    }

    app.join();

    Ok(())
}

fn main() {
    println!("NODO DE ALGLOBO");
    let path_pagos = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Se debe indicar un path a un archivo de pagos");
            return;
        }
    };

    let path_fallidos = match std::env::args().nth(2) {
        Some(path) => path,
        None => {
            println!("Se debe indicar un path a un archivo de pagos fallidos");
            return;
        }
    };

    let id = match std::env::args()
        .nth(3)
        .and_then(|a| a.parse::<usize>().ok())
    {
        Some(r) => r,
        None => {
            println!("Se debe indicar un id numerico para el nodo");
            return;
        }
    };

    if let Err(err) = procesar(id, path_pagos, path_fallidos) {
        println!("{}", err)
    }
}
