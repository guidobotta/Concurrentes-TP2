mod model;
use model::aplicacion::Aplicacion;
use common::error::Resultado;
use model::escritor_fallidos::EscritorFallidos;
use model::eleccion_lider::EleccionLider;
use model::parser::Parser;

fn procesar(id: usize, path_pagos: String, path_fallidos: String) -> Resultado<()> {
    let parseador = Parser::new(path_pagos)?;
    let lider = EleccionLider::new(id);
    let escritor = EscritorFallidos::new(path_fallidos)?;
    let aplicacion = Aplicacion::new(id, lider, parseador, escritor)?;
    let mut entrada = String::new();
    //Loopea infinitamente si la app finaliza
    loop {
        let _ = std::io::stdin().read_line(&mut entrada);
        if entrada.contains("SALIR") {
            aplicacion.finalizar();
            break;
        }
    }
    Ok(())
}

fn main() {
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
