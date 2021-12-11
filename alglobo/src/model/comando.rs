use common::error::{ErrorApp, ErrorInterno, Resultado};

#[derive(Clone, PartialEq, Debug)]
pub enum Comando {
    REINTENTAR { id: usize },
    FINALIZAR
}

impl Comando {

    pub fn decodificar(mensaje_codificado: &String) -> Resultado<Comando> {
        let parseado = mensaje_codificado.split(' ').collect::<Vec<&str>>();
        match parseado[0] {
            "REINTENTAR" => Ok(Comando::REINTENTAR { id: parseado[1].parse::<usize>()? }),
            "FINALIZAR" => Ok(Comando::FINALIZAR),
            _ => return Err(ErrorApp::Interno(ErrorInterno::new(&format!("Mensaje erroneo: {}", parseado[0])))),
        }
    }
}