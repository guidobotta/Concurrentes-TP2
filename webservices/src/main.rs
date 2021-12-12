mod model;
use model::web_service::WebService;

// TODO: Documentacion?? Es privada
fn run() {
    let id = match std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<usize>().ok())
    {
        Some(r) => r,
        None => {
            println!("Se debe indicar un id numerico para el servicio");
            println!("0 -> ");
            return;
        }
    };

    let mut web_service = WebService::new(id);
    web_service.run();
}

// TODO: Documentacion?? Es privada
fn main() {
    println!("WEBSERVICE");
    run()
}
