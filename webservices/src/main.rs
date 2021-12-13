mod model;
use common::error::Resultado;
use model::web_service::WebService;

// TODO: Documentacion?? Es privada
fn run() -> Resultado<()> {
    let id = match std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<usize>().ok())
    {
        Some(r) => r,
        None => {
            println!("Se debe indicar un id numerico para el servicio");
            println!("0 -> ");
            return Ok(());
        }
    };

    let mut web_service = WebService::new(id)?;
    
    Ok(web_service.run())
}

// TODO: Documentacion?? Es privada
fn main() {
    println!("WEBSERVICE"); //TODO: Agregar finalizacion
    if let Err(err) = run() { println!("{}", err); }
}
