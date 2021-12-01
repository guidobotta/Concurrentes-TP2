mod model;
use model::aplicacion::Aplicacion;
use common::error::Resultado;
use model::leader_election::LeaderElection;
use model::parser::Parser;

fn procesar(id: usize, path: String) -> Resultado<()> {
    let parseador = Parser::new(path)?;
    let lider = LeaderElection::new(id);
    let aplicacion = Aplicacion::new(id, lider, parseador)?;

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
    let path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Se debe indicar un path a un archivo de entrada");
            return;
        }
    };

    let id = match std::env::args()
        .nth(2)
        .and_then(|a| a.parse::<usize>().ok())
    {
        Some(r) => r,
        None => {
            println!("Se debe indicar un id numerico para el nodo");
            return;
        }
    };

    if let Err(err) = procesar(id, path) {
        println!("{}", err)
    }
}
